//! # Agendador de Caminho Crítico do Navegador (RFC 9218)
//!
//! Orquestra requisições de recursos web categorizadas por prioridade de renderização:
//! HTML raiz > CSS bloqueante > Fontes > Scripts > Imagens > Telemetria.

use parking_lot::Mutex;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::Arc;

/// Prioridade normativa de recursos do navegador alinhada ao RFC 9218.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ResourcePriority {
    /// Beacons e telemetria em segundo plano
    Lowest = 0,
    /// Imagens fora da viewport (lazy-load) e prefetch especulativo
    Low = 1,
    /// Imagens na viewport e scripts assíncronos
    #[default]
    Medium = 2,
    /// Folhas de estilo assíncronas e fontes críticas (@font-face)
    High = 3,
    /// Documento HTML raiz e folhas de estilo CSS bloqueantes de renderização
    Urgent = 4,
}

/// Tarefa agendada com ordem de submissão e prioridade estrita.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledResourceTask {
    pub task_id: u64,
    pub priority: ResourcePriority,
    pub url: String,
    pub initiator: String,
    pub sequence: u64,
}

impl Ord for ScheduledResourceTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Maior prioridade primeiro; para mesma prioridade, menor sequence (FIFO)
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl PartialOrd for ScheduledResourceTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Agendador concorrente de recursos de caminho crítico.
#[derive(Debug, Default, Clone)]
pub struct CriticalResourceScheduler {
    sequence_gen: Arc<AtomicU64>,
    task_id_gen: Arc<AtomicU64>,
    queue: Arc<Mutex<BinaryHeap<ScheduledResourceTask>>>,
    active_in_flight: Arc<AtomicU64>,
    max_concurrent: usize,
}

impl CriticalResourceScheduler {
    /// Cria uma nova instância do agendador com limite de requisições concorrentes.
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            sequence_gen: Arc::new(AtomicU64::new(0)),
            task_id_gen: Arc::new(AtomicU64::new(1)),
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
            active_in_flight: Arc::new(AtomicU64::new(0)),
            max_concurrent,
        }
    }

    /// Enfileira uma nova requisição com prioridade atribuída.
    pub fn schedule(
        &self,
        url: impl Into<String>,
        priority: ResourcePriority,
        initiator: impl Into<String>,
    ) -> u64 {
        let task_id = self.task_id_gen.fetch_add(1, AtomicOrdering::SeqCst);
        let seq = self.sequence_gen.fetch_add(1, AtomicOrdering::SeqCst);

        let task = ScheduledResourceTask {
            task_id,
            priority,
            url: url.into(),
            initiator: initiator.into(),
            sequence: seq,
        };

        let mut lock = self.queue.lock();
        lock.push(task);
        task_id
    }

    /// Despacha a próxima tarefa de maior prioridade se o limite de concorrência permitir.
    pub fn poll_next(&self) -> Option<ScheduledResourceTask> {
        let current_in_flight = self.active_in_flight.load(AtomicOrdering::SeqCst);
        if current_in_flight >= self.max_concurrent as u64 {
            return None;
        }

        let mut lock = self.queue.lock();
        let task = lock.pop()?;
        self.active_in_flight.fetch_add(1, AtomicOrdering::SeqCst);
        Some(task)
    }

    /// Sinaliza a conclusão de uma tarefa despachada, liberando slot de concorrência.
    pub fn mark_completed(&self) {
        self.active_in_flight.fetch_sub(1, AtomicOrdering::SeqCst);
    }

    /// Quantidade de tarefas ainda enfileiradas aguardando despacho.
    pub fn pending_count(&self) -> usize {
        self.queue.lock().len()
    }

    /// Quantidade de tarefas atualmente em trânsito (in-flight).
    pub fn in_flight_count(&self) -> u64 {
        self.active_in_flight.load(AtomicOrdering::SeqCst)
    }
}
