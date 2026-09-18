use crate::engine::priority::{PrioritizedItem, PriorityLevel};
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::oneshot;
use ace_core::id::RequestId;

/// Configuração de limites do ResourceScheduler.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub max_global: u32,
    pub max_per_host: u32,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_global: 32,
            max_per_host: 6,
        }
    }
}

struct PendingRequest {
    host: SmolStr,
    wake_tx: oneshot::Sender<SchedulerPermit>,
    priority: PriorityLevel,
    sequence_id: u64,
}

struct SchedulerState {
    active_global: u32,
    active_per_host: FxHashMap<SmolStr, u32>,
    active_high_priority: u32, // Para o Tail Scheduling
    queue: BTreeSet<PrioritizedItem<RequestId>>,
    pending: FxHashMap<RequestId, PendingRequest>,
}

/// Orquestrador de contenção e prioridade. Limita requests em voo globalmente e por host.
pub struct ResourceScheduler {
    config: SchedulerConfig,
    state: Mutex<SchedulerState>,
    sequence: AtomicU64,
}

/// Permissão adquirida para executar uma request. Libera o slot ao ser dropada (RAII).
pub struct SchedulerPermit {
    scheduler: Arc<ResourceScheduler>,
    pub priority: PriorityLevel,
    host: SmolStr,
}

impl Drop for SchedulerPermit {
    fn drop(&mut self) {
        if !self.host.is_empty() {
            self.scheduler.release_permit(&self.host, self.priority, self.scheduler.clone());
        }
    }
}

impl ResourceScheduler {
    /// Cria um novo escalonador de recursos com a configuração especificada.
    pub fn new(config: SchedulerConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            state: Mutex::new(SchedulerState {
                active_global: 0,
                active_per_host: FxHashMap::default(),
                active_high_priority: 0,
                queue: BTreeSet::new(),
                pending: FxHashMap::default(),
            }),
            sequence: AtomicU64::new(0),
        })
    }

    /// Tenta adquirir um slot de execução. Se os limites (global ou por host) estiverem saturados,
    /// aguarda em uma fila de prioridade. Requisições `VeryHigh` sempre recebem bypass.
    pub async fn acquire(self: &Arc<Self>, request_id: RequestId, priority: PriorityLevel, host: SmolStr) -> SchedulerPermit {
        // Bypass completo para documentos críticos
        if priority == PriorityLevel::VeryHigh {
            return SchedulerPermit {
                scheduler: self.clone(),
                priority,
                host: SmolStr::default(),
            };
        }

        let rx = {
            let mut state = self.state.lock();

            // Tail Scheduling agressivo: bloqueia requisicoes Low/Lowest se houver alta prioridade
            let is_low_priority = priority <= PriorityLevel::Low;
            let tail_blocked = is_low_priority && state.active_high_priority > 0;

            // Tenta adquirir imediatamente
            if !tail_blocked && state.active_global < self.config.max_global {
                let host_count = state.active_per_host.get(&host).copied().unwrap_or(0);
                if host_count < self.config.max_per_host {
                    state.active_global += 1;
                    if priority >= PriorityLevel::High {
                        state.active_high_priority += 1;
                    }
                    *state.active_per_host.entry(host.clone()).or_insert(0) += 1;
                    return SchedulerPermit {
                        scheduler: self.clone(),
                        priority,
                        host,
                    };
                }
            }

            // Limite atingido: entra na fila de prioridade
            let (tx, rx) = oneshot::channel();
            let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
            state.queue.insert(PrioritizedItem {
                priority,
                sequence_id: seq,
                item: request_id,
            });
            state.pending.insert(request_id, PendingRequest {
                host: host.clone(),
                wake_tx: tx,
                priority,
                sequence_id: seq,
            });
            
            crate::telemetry::net_log::log_net_event(
                crate::telemetry::net_log::NetEventType::Queue,
                host.as_str(),
                "Request queued by scheduler",
            );
            rx
        };

        // Aguarda ser acordado por outra request que terminou
        rx.await.unwrap_or_else(|_| SchedulerPermit {
            scheduler: self.clone(),
            priority,
            host,
        })
    }

    fn release_permit(&self, host: &SmolStr, priority: PriorityLevel, arc_self: Arc<ResourceScheduler>) {
        let mut state = self.state.lock();

        // 1. Libera a contagem
        state.active_global = state.active_global.saturating_sub(1);
        if priority >= PriorityLevel::High {
            state.active_high_priority = state.active_high_priority.saturating_sub(1);
        }
        if let Some(count) = state.active_per_host.get_mut(host) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                state.active_per_host.remove(host);
            }
        }

        // 2. Procura o próximo elegível na fila (maior prioridade que respeite os limites per-host)
        // Tail Scheduling: Se a próxima request for Low, verificamos se tem High ativa.
        let mut skipped: Vec<(RequestId, PendingRequest)> = Vec::new();
        
        while let Some(prioritized) = state.queue.pop_last() {
            let request_id = prioritized.item;
            
            // Tail Scheduling
            let is_low_priority = prioritized.priority <= PriorityLevel::Low;
            if is_low_priority && state.active_high_priority > 0 {
                // Bloqueia a execução, mas temos que devolver ela na fila
                // Como as requests de menor prioridade estão no final da queue, as subsequentes tbm estarão bloqueadas?
                // `pop_last` tira as de MAIOR prioridade primeiro! Então não devemos bloquear o loop inteiro.
                // Na verdade, se chegamos num Low e temos High ativo, todas as restantes no `queue` são <= Low,
                // então podemos simplesmente interromper o loop e devolver!
                state.queue.insert(prioritized);
                break;
            }

            // Remover do pending temporariamente para examinar
            if let Some(pending) = state.pending.remove(&request_id) {
                let pending_host = &pending.host;
                let host_count = state.active_per_host.get(pending_host).copied().unwrap_or(0);

                if host_count < self.config.max_per_host {
                    // Acorda esta request
                    state.active_global += 1;
                    if pending.priority >= PriorityLevel::High {
                        state.active_high_priority += 1;
                    }
                    *state.active_per_host.entry(pending_host.clone()).or_insert(0) += 1;

                    let permit = SchedulerPermit {
                        scheduler: arc_self,
                        priority: pending.priority,
                        host: pending_host.clone(),
                    };

                    // Melhor soltar o lock antes de enviar
                    drop(state);
                    
                    let _send_result = pending.wake_tx.send(permit);
                    
                    // Re-adquire o lock para devolver os skips
                    let mut state2 = self.state.lock();
                    for (req_id_skipped, pending_skipped) in skipped {
                        state2.queue.insert(PrioritizedItem {
                            priority: pending_skipped.priority,
                            sequence_id: pending_skipped.sequence_id,
                            item: req_id_skipped,
                        });
                        state2.pending.insert(req_id_skipped, pending_skipped);
                    }
                    
                    return;
                } else {
                    // Host limit reached, guarda para devolver pra fila
                    skipped.push((request_id, pending));
                }
            }
        }

        // Devolve os pulados
        for (req_id_skipped, pending_skipped) in skipped {
            state.queue.insert(PrioritizedItem {
                priority: pending_skipped.priority,
                sequence_id: pending_skipped.sequence_id,
                item: req_id_skipped,
            });
            state.pending.insert(req_id_skipped, pending_skipped);
        }
    }

    /// Reprioritiza dinamicamente uma request na fila em tempo O(log N).
    pub fn reprioritize(&self, request_id: RequestId, new_priority: PriorityLevel) -> bool {
        let mut state = self.state.lock();
        if let Some(mut pending) = state.pending.remove(&request_id) {
            let old_priority = pending.priority;
            let sequence_id = pending.sequence_id;
            
            // Remove from BTreeSet
            state.queue.remove(&PrioritizedItem {
                priority: old_priority,
                sequence_id,
                item: request_id,
            });

            // Update priority and re-insert
            pending.priority = new_priority;
            state.queue.insert(PrioritizedItem {
                priority: new_priority,
                sequence_id,
                item: request_id,
            });
            
            state.pending.insert(request_id, pending);
            true
        } else {
            false
        }
    }
}
