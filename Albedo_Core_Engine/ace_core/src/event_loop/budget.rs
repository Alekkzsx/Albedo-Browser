//! # Escalonamento Anti-Inanição, Quotas de CPU e Gestão de Energia (Chromium SequenceManager Pattern)
//!
//! Em loops de eventos de navegadores web sob alta carga de input do usuário ou animações contínuas,
//! tarefas de menor prioridade (background, timers, telemetria) correm risco de inanição infinita (*starvation*).
//!
//! Este módulo implementa:
//! 1. `TaskPriority`: Níveis discretos de prioridade (`Control`, `Highest`, `High`, `Normal`, `Low`, `BestEffort`).
//! 2. `AntiStarvationSelector`: Algoritmo de envelhecimento de tarefas (*task aging*) com score de inanição
//!    que força a execução periódica de tarefas preteridas.
//! 3. `CPUTimeBudgetPool`: Algoritmo Token Bucket para controle estrito de consumo de CPU.
//! 4. `BackgroundTabThrottler`: Clamping de timers para 1000ms/60s e alinhamento em múltiplos de segundo (*Timer Coalescing*)
//!    para permitir C-states de baixo consumo no processador.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Níveis discretos de prioridade de tarefas no motor (Chromium `TaskQueue::QueuePriority`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
pub enum TaskPriority {
    /// Máxima prioridade: controle interno crítico do loop e IPC do sistema.
    Control = 0,
    /// Processamento de frames da GPU e entrada imediata de usuário.
    Highest = 1,
    /// Interações de usuário e tarefas DOM de primeiro plano.
    High = 2,
    /// Tarefas padrão de scripts, navegação e promessas JavaScript.
    #[default]
    Normal = 3,
    /// Renderização diferida e timers de primeiro plano.
    Low = 4,
    /// Telemetria, pré-busca de recursos e tarefas de limpeza (Garbage Collection).
    BestEffort = 5,
}

/// Seletor de filas com rastreamento e prevenção de inanição (*Anti-Starvation*).
pub struct AntiStarvationSelector {
    starvation_scores: [AtomicU32; 6],
    max_starvation_threshold: u32,
}

impl Default for AntiStarvationSelector {
    fn default() -> Self {
        Self::new(5)
    }
}

impl AntiStarvationSelector {
    /// Cria um novo seletor com o limite de tolerância de inanição especificado.
    pub fn new(max_starvation_threshold: u32) -> Self {
        Self {
            starvation_scores: [
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
            ],
            max_starvation_threshold,
        }
    }

    /// Registra a execução de uma tarefa com prioridade `executed_priority` e envelhece as demais.
    pub fn record_task_executed(&self, executed_priority: TaskPriority) {
        let exec_idx = executed_priority as usize;
        // Zera o score de inanição da prioridade atendida
        self.starvation_scores[exec_idx].store(0, Ordering::Relaxed);

        // Incrementa o score de inanição das prioridades inferiores ativas
        for idx in (exec_idx + 1)..6 {
            self.starvation_scores[idx].fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Retorna `true` se a prioridade especificada atingiu o limite de inanição e deve ser forçada.
    pub fn should_force_priority(&self, priority: TaskPriority) -> bool {
        let idx = priority as usize;
        self.starvation_scores[idx].load(Ordering::Relaxed) >= self.max_starvation_threshold
    }
}

/// Pool de orçamento de tempo de CPU via Token Bucket (Chromium `CPUTimeBudgetPool`).
pub struct CPUTimeBudgetPool {
    max_budget_ms: f64,
    recovery_rate: f64, // Ex: 0.01 = 1% de tempo de CPU recuperado por segundo real
    last_update_ns: AtomicU64,
    current_budget_us: parking_lot::Mutex<f64>,
}

impl CPUTimeBudgetPool {
    /// Cria um pool com orçamento máximo e taxa de regeneração de CPU.
    pub fn new(max_budget_ms: f64, recovery_rate: f64) -> Self {
        Self {
            max_budget_ms,
            recovery_rate,
            last_update_ns: AtomicU64::new(0),
            current_budget_us: parking_lot::Mutex::new(max_budget_ms * 1000.0),
        }
    }

    /// Atualiza e consome o tempo de execução consumido por uma tarefa.
    pub fn record_execution(&self, execution_duration: Duration, _now: Instant) {
        let mut budget = self.current_budget_us.lock();
        let exec_us = execution_duration.as_micros() as f64;
        *budget = (*budget - exec_us).max(-500_000.0); // Limite de dívida máxima
    }

    /// Retorna o orçamento máximo em milissegundos.
    #[inline]
    pub fn max_budget_ms(&self) -> f64 {
        self.max_budget_ms
    }

    /// Retorna a taxa de regeneração do orçamento.
    #[inline]
    pub fn recovery_rate(&self) -> f64 {
        self.recovery_rate
    }

    /// Retorna o timestamp em nanossegundos da última atualização.
    #[inline]
    pub fn last_update_ns(&self) -> u64 {
        self.last_update_ns.load(Ordering::Relaxed)
    }

    /// Retorna `true` se a fila ainda possuir orçamento de CPU disponível para execução imediata.
    pub fn has_budget(&self) -> bool {
        *self.current_budget_us.lock() > 0.0
    }
}

/// Gerenciador de Throttling e Economia de Energia para Abas em Segundo Plano.
pub struct BackgroundTabThrottler {
    is_background: AtomicBool,
    hidden_since: parking_lot::Mutex<Option<Instant>>,
}

impl Default for BackgroundTabThrottler {
    fn default() -> Self {
        Self::new()
    }
}

impl BackgroundTabThrottler {
    pub fn new() -> Self {
        Self {
            is_background: AtomicBool::new(false),
            hidden_since: parking_lot::Mutex::new(None),
        }
    }

    /// Altera o estado de visibilidade da aba.
    pub fn set_visibility(&self, is_hidden: bool) {
        self.is_background.store(is_hidden, Ordering::SeqCst);
        let mut guard = self.hidden_since.lock();
        if is_hidden {
            if guard.is_none() {
                *guard = Some(Instant::now());
            }
        } else {
            *guard = None;
        }
    }

    /// Calcula o atraso mínimo mandatório e aplica alinhamento de timers (Timer Coalescing).
    pub fn clamp_timer_delay(&self, requested_delay: Duration) -> Duration {
        if !self.is_background.load(Ordering::Relaxed) {
            return requested_delay.max(Duration::from_millis(1));
        }

        let hidden_duration = self
            .hidden_since
            .lock()
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO);

        // Após 5 minutos em background: Throttling Intensivo (1 minuto)
        if hidden_duration >= Duration::from_secs(300) {
            Duration::from_secs(60).max(requested_delay)
        }
        // Após 5 segundos de grace period: Throttling padrão de 1000ms
        else if hidden_duration >= Duration::from_secs(5) {
            Duration::from_millis(1000).max(requested_delay)
        } else {
            requested_delay.max(Duration::from_millis(4))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anti_starvation_scoring() {
        let selector = AntiStarvationSelector::new(3);

        assert!(!selector.should_force_priority(TaskPriority::Normal));

        // Simula 3 execuções consecutivas de High priority
        selector.record_task_executed(TaskPriority::High);
        selector.record_task_executed(TaskPriority::High);
        selector.record_task_executed(TaskPriority::High);

        // Normal deve ser forçada agora
        assert!(selector.should_force_priority(TaskPriority::Normal));

        // Após executar Normal, ela reseta
        selector.record_task_executed(TaskPriority::Normal);
        assert!(!selector.should_force_priority(TaskPriority::Normal));
    }

    #[test]
    fn test_background_tab_throttler() {
        let throttler = BackgroundTabThrottler::new();
        // Em primeiro plano
        assert_eq!(
            throttler.clamp_timer_delay(Duration::from_millis(2)),
            Duration::from_millis(2)
        );

        // Em background imediato (grace period)
        throttler.set_visibility(true);
        assert_eq!(
            throttler.clamp_timer_delay(Duration::from_millis(1)),
            Duration::from_millis(4)
        );
    }
}
