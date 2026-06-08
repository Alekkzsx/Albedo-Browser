use crate::parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// Importa a função utilitária comum em vez de duplicar
fn current_timestamp_ms() -> u64 {
    albedo_browser::ace::utils::current_timestamp_ms()
}

/// Identificador único para uma função JavaScript.
/// Composto pelo nome do script/função e um hash do código/posição.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FunctionId(pub String);

/// Contadores de execução e metadados de uma função JS específica.
#[derive(Debug)]
pub struct ExecutionCounter {
    pub call_count: AtomicUsize,
    pub first_call_timestamp_ms: u64,
    pub last_call_timestamp_ms: AtomicUsize,
}

impl ExecutionCounter {
    pub fn new() -> Self {
        let now = current_timestamp_ms();
        Self {
            call_count: AtomicUsize::new(1),
            first_call_timestamp_ms: now,
            last_call_timestamp_ms: AtomicUsize::new(now as usize),
        }
    }

    pub fn increment(&self) -> usize {
        let count = self.call_count.fetch_add(1, Ordering::Relaxed) + 1;
        self.last_call_timestamp_ms
            .store(current_timestamp_ms() as usize, Ordering::Relaxed);
        count
    }

    pub fn count(&self) -> usize {
        self.call_count.load(Ordering::Relaxed)
    }
}

/// Configurações do Profiler do AlbedoJIT.
#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    /// Número de chamadas necessárias para marcar uma função como "Quente" (Hot)
    /// e elegível para Tier 1 (Baseline JIT). Padrão: 50.
    pub hot_threshold: usize,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self { hot_threshold: 50 }
    }
}

/// Snapshot de estatísticas de uma função retornado pelas APIs de consulta.
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub call_count: usize,
    pub is_hot: bool,
    pub time_since_first_call_ms: u64,
}

/// O Profiler de Execução do AlbedoJIT.
///
/// Monitora a frequência de chamadas das funções JavaScript puras para
/// determinar quais são os "Hot Paths" que devem ser compilados para
/// código de máquina nativo.
///
/// Projetado para ser thread-safe, minimizando bloqueios através do uso
/// de `AtomicUsize` dentro de um `RwLock`.
pub struct JitProfiler {
    config: ProfilerConfig,
    counters: Arc<RwLock<HashMap<FunctionId, Arc<ExecutionCounter>>>>,
    hot_queue: Arc<Mutex<Vec<FunctionId>>>,
}

impl JitProfiler {
    pub fn new(config: ProfilerConfig) -> Self {
        Self {
            config,
            counters: Arc::new(RwLock::new(HashMap::new())),
            hot_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Registra a chamada de uma função.
    ///
    /// Se a contagem atingir o limiar (hot_threshold) exatamente nesta chamada,
    /// a função é adicionada à fila de pré-compilação JIT (`hot_queue`).
    pub fn record_call(&self, id: FunctionId) {
        let current_count;

        // Fast path: A função já existe no mapa?
        // Usamos read lock para alta concorrência.
        let counter_arc = {
            let map = self.counters.read();
            map.get(&id).cloned() // Clone to arc
        };

        if let Some(counter) = counter_arc {
            current_count = counter.increment();
        } else {
            // Slow path: Primeira vez que a função é chamada (precisa de write lock)
            let mut map = self.counters.write();
            // Checagem dupla (outra thread pode ter inserido enquanto esperávamos)
            let counter = map
                .entry(id.clone())
                .or_insert_with(|| Arc::new(ExecutionCounter::new()));

            // Só incrementamos aqui se nós NÃO fomos quem criou (pq new() ja vem com 1)
            // Mas o or_insert_with já resolve isso implicitamente pela semântica de 1 na criação.
            current_count = counter.count();
        }

        // Se exatamente nesta chamada bater o threshold, enfileiramos pro JIT.
        // Impedimos de enfileirar múltiplas vezes (ex: se ja passou de 50).
        if current_count == self.config.hot_threshold {
            let mut queue = self.hot_queue.lock();
            queue.push(id.clone());
        }
    }

    /// Verifica se uma função já ultrapassou o limiar de JIT (Tier 1).
    pub fn is_hot(&self, id: &FunctionId) -> bool {
        let map = self.counters.read();
        if let Some(counter) = map.get(id) {
            counter.count() >= self.config.hot_threshold
        } else {
            false
        }
    }

    /// Extrai e limpa a fila de funções prontas para compilação.
    ///
    /// Deve ser consumido pela thread do JIT Compiler.
    pub fn drain_hot_queue(&self) -> Vec<FunctionId> {
        let mut queue = self.hot_queue.lock();
        std::mem::take(&mut queue)
    }

    /// Retorna as estatísticas detalhadas de uma função, se ela foi rastreada.
    pub fn get_stats(&self, id: &FunctionId) -> Option<ExecutionStats> {
        let map = self.counters.read();
        map.get(id).map(|c| {
            let count = c.count();
            ExecutionStats {
                call_count: count,
                is_hot: count >= self.config.hot_threshold,
                time_since_first_call_ms: current_timestamp_ms()
                    .saturating_sub(c.first_call_timestamp_ms),
            }
        })
    }

    /// Quantas funções distintas o profiler já rastreou.
    pub fn total_tracked_functions(&self) -> usize {
        self.counters.read().len()
    }

    /// Zera completamente o profiler (limpa contadores e filar).
    pub fn reset(&self) {
        self.counters.write().clear();
        self.hot_queue.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler_basic_counting() {
        let profiler = JitProfiler::new(ProfilerConfig::default());
        let id = FunctionId("test_func".into());

        for _ in 0..10 {
            profiler.record_call(id.clone());
        }

        let stats = profiler.get_stats(&id).unwrap();
        assert_eq!(stats.call_count, 10);
        assert!(!stats.is_hot); // 10 < 50
    }

    #[test]
    fn test_profiler_hot_threshold() {
        let profiler = JitProfiler::new(ProfilerConfig { hot_threshold: 15 });
        let id = FunctionId("func_hot".into());

        // Ainda não é hot
        for _ in 0..14 {
            profiler.record_call(id.clone());
        }
        assert!(!profiler.is_hot(&id));

        // Aqui bate o threshold exato (15)
        profiler.record_call(id.clone());
        assert!(profiler.is_hot(&id));

        // Verifica se tá na queue
        let mut queue = profiler.drain_hot_queue();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0], id);

        // Ultrapassa
        profiler.record_call(id.clone());
        assert!(profiler.is_hot(&id));

        // Não enfileira de novo
        queue = profiler.drain_hot_queue();
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_profiler_hot_queue_drain() {
        let profiler = JitProfiler::new(ProfilerConfig { hot_threshold: 2 });
        let id1 = FunctionId("func1".into());
        let id2 = FunctionId("func2".into());

        profiler.record_call(id1.clone());
        profiler.record_call(id1.clone()); // Bateu!
        profiler.record_call(id2.clone());
        profiler.record_call(id2.clone()); // Bateu!

        let queue = profiler.drain_hot_queue();
        assert_eq!(queue.len(), 2);
        assert!(queue.contains(&id1));
        assert!(queue.contains(&id2));

        // Após dreanar, fila tá vazia
        assert!(profiler.drain_hot_queue().is_empty());
    }

    #[test]
    fn test_profiler_thread_safety() {
        let profiler = Arc::new(JitProfiler::new(ProfilerConfig::default()));
        let id = FunctionId("parallel_func".into());

        let mut handles = vec![];

        for _ in 0..10 {
            let prof_clone = Arc::clone(&profiler);
            let id_clone = id.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..10 {
                    prof_clone.record_call(id_clone.clone());
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(profiler.get_stats(&id).unwrap().call_count, 100);
        assert_eq!(profiler.total_tracked_functions(), 1);
        // Queue deve ter capturado (100 > 50)
        assert_eq!(profiler.drain_hot_queue().len(), 1);
    }

    #[test]
    fn test_profiler_reset() {
        let profiler = JitProfiler::new(ProfilerConfig { hot_threshold: 5 });
        let id = FunctionId("func_reset".into());

        for _ in 0..5 {
            profiler.record_call(id.clone());
        }
        assert_eq!(profiler.total_tracked_functions(), 1);
        assert_eq!(profiler.drain_hot_queue().len(), 1);

        profiler.reset();

        assert_eq!(profiler.total_tracked_functions(), 0);
        assert_eq!(profiler.drain_hot_queue().len(), 0);
        assert!(profiler.get_stats(&id).is_none());
    }
}
