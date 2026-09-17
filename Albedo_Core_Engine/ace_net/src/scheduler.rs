use crate::priority::{PrioritizedItem, PriorityLevel};
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::oneshot;

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
}

struct SchedulerState {
    active_global: u32,
    active_per_host: FxHashMap<SmolStr, u32>,
    queue: BTreeSet<PrioritizedItem<()>>,
    pending: FxHashMap<u64, PendingRequest>,
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
    host: SmolStr,
}

impl Drop for SchedulerPermit {
    fn drop(&mut self) {
        if !self.host.is_empty() {
            self.scheduler.release_permit(&self.host, self.scheduler.clone());
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
                queue: BTreeSet::new(),
                pending: FxHashMap::default(),
            }),
            sequence: AtomicU64::new(0),
        })
    }

    /// Tenta adquirir um slot de execução. Se os limites (global ou por host) estiverem saturados,
    /// aguarda em uma fila de prioridade. Requisições `VeryHigh` sempre recebem bypass.
    pub async fn acquire(self: &Arc<Self>, priority: PriorityLevel, host: SmolStr) -> SchedulerPermit {
        // Bypass completo para documentos críticos
        if priority == PriorityLevel::VeryHigh {
            return SchedulerPermit {
                scheduler: self.clone(),
                host: SmolStr::default(),
            };
        }

        let rx = {
            let mut state = self.state.lock();

            // Tenta adquirir imediatamente
            if state.active_global < self.config.max_global {
                let host_count = state.active_per_host.get(&host).copied().unwrap_or(0);
                if host_count < self.config.max_per_host {
                    state.active_global += 1;
                    *state.active_per_host.entry(host.clone()).or_insert(0) += 1;
                    return SchedulerPermit {
                        scheduler: self.clone(),
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
                item: (),
            });
            state.pending.insert(seq, PendingRequest {
                host: host.clone(),
                wake_tx: tx,
                priority,
            });
            
            crate::net_log::log_net_event(
                crate::net_log::NetEventType::Queue,
                host.as_str(),
                "Request queued by scheduler",
            );
            rx
        };

        // Aguarda ser acordado por outra request que terminou
        rx.await.unwrap_or_else(|_| SchedulerPermit {
            scheduler: self.clone(),
            host,
        })
    }

    fn release_permit(&self, host: &SmolStr, arc_self: Arc<ResourceScheduler>) {
        let mut state = self.state.lock();

        // 1. Libera a contagem
        state.active_global = state.active_global.saturating_sub(1);
        if let Some(count) = state.active_per_host.get_mut(host) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                state.active_per_host.remove(host);
            }
        }

        // 2. Procura o próximo elegível na fila (maior prioridade que respeite os limites per-host)
        let mut skipped = Vec::new();
        
        while let Some(prioritized) = state.queue.pop_last() {
            let seq = prioritized.sequence_id;
            // Remover do pending temporariamente para examinar
            if let Some(pending) = state.pending.remove(&seq) {
                let pending_host = &pending.host;
                let host_count = state.active_per_host.get(pending_host).copied().unwrap_or(0);

                if host_count < self.config.max_per_host {
                    // Acorda esta request
                    state.active_global += 1;
                    *state.active_per_host.entry(pending_host.clone()).or_insert(0) += 1;

                let permit = SchedulerPermit {
                    scheduler: arc_self,
                    host: pending_host.clone(),
                };

                // Se o send falhar (request cancelada enquanto na fila), 
                // a contagem e os limites devem ser revertidos/repassados.
                // Isso é tratado porque ao falhar, nós ignoramos e o drop() do
                // permit construído AQUI iria rodar e liberar o slot novamente.
                // Mas wait, se `send` falha, o `permit` cai no `Err(permit)` e é dropado na hora!
                // O drop dele chamará `release_permit` e vai recursar ou repassar.
                // Para evitar recursão aninhada sob lock, não vamos construir o permit
                // até garantir que o receiver está vivo, ou construímos e soltamos fora do lock.
                
                // Melhor soltar o lock antes de enviar
                drop(state);
                
                let send_result = pending.wake_tx.send(permit);
                
                // Re-adquire o lock para devolver os skips
                let mut state2 = self.state.lock();
                for (seq_skipped, pending_skipped) in skipped {
                    state2.queue.insert(PrioritizedItem {
                        priority: pending_skipped.priority,
                        sequence_id: seq_skipped,
                        item: (),
                    });
                    state2.pending.insert(seq_skipped, pending_skipped);
                }
                
                // Se falhou o send, o receiver morreu, então o permit é descartado 
                // fora do lock e isso já causa outro release_permit internamente!
                if send_result.is_err() {
                    // O permit vai dar drop e disparar outro release_permit.
                }
                return;
            } else {
                // Host limit reached, guarda para devolver pra fila
                skipped.push((seq, pending));
            }
        }

        // Devolve os pulados
        for (seq_skipped, pending_skipped) in skipped {
            state.queue.insert(PrioritizedItem {
                priority: pending_skipped.priority,
                sequence_id: seq_skipped,
                item: (),
            });
            state.pending.insert(seq_skipped, pending_skipped);
        }
    }

    /// Reprioritiza dinamicamente uma request na fila em tempo O(log N).
    pub fn reprioritize(&self, sequence_id: u64, new_priority: PriorityLevel) -> bool {
        let mut state = self.state.lock();
        if let Some(mut pending) = state.pending.remove(&sequence_id) {
            let old_priority = pending.priority;
            
            // Remove from BTreeSet
            state.queue.remove(&PrioritizedItem {
                priority: old_priority,
                sequence_id,
                item: (),
            });

            // Update priority and re-insert
            pending.priority = new_priority;
            state.queue.insert(PrioritizedItem {
                priority: new_priority,
                sequence_id,
                item: (),
            });
            
            state.pending.insert(sequence_id, pending);
            true
        } else {
            false
        }
    }
}
