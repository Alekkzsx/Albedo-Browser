//! Métricas de uso das arenas.
//!
//! O `PLANO.md` exige ferramentas de *profiling* de memória (Heaptrack / Windows
//! Performance Toolkit) para caçar vazamentos em SPAs. [`ArenaStats`] é a fonte de dados
//! primária dessas ferramentas: cada arena expõe um snapshot barato e sem alocação.

/// Snapshot das métricas de uma arena.
///
/// Todos os campos são cópias baratas (`Copy`), então coletar estatísticas em um loop de
/// renderização ou em um teste de longa duração não introduz overhead relevante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArenaStats {
    /// Número de valores vivos neste momento.
    pub live: usize,
    /// Capacidade alocada (slots para [`super::Arena`], bytes para [`super::BumpArena`]).
    pub capacity: usize,
    /// Total de alocações ao longo da vida da arena.
    pub total_allocated: u64,
    /// Total de liberações explícitas (sempre 0 em arenas de bump).
    pub total_freed: u64,
    /// Quantas vezes um slot livre foi reutilizado (sempre 0 em arenas de bump).
    pub slot_reuses: u64,
    /// Bytes ocupados pelos valores vivos (estimativa em [`super::Arena`]).
    pub bytes_allocated: usize,
}

impl ArenaStats {
    /// Razão de reutilização de slots, entre `0.0` e `1.0`.
    ///
    /// Um valor alto indica que a arena está reciclando memória em vez de crescer —
    /// sinal de um DOM com muita mutação e pouca criação líquida de nós.
    #[must_use]
    pub fn reuse_ratio(&self) -> f64 {
        if self.total_allocated == 0 {
            return 0.0;
        }
        self.slot_reuses as f64 / self.total_allocated as f64
    }

    /// Bytes médios por valor vivo. Útil para detectar nós unexpectedly grandes.
    #[must_use]
    pub fn bytes_per_live(&self) -> f64 {
        if self.live == 0 {
            return 0.0;
        }
        self.bytes_allocated as f64 / self.live as f64
    }
}

impl std::fmt::Display for ArenaStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ArenaStats {{ live: {}, cap: {}, allocs: {}, freed: {}, reuses: {}, bytes: {} }}",
            self.live, self.capacity, self.total_allocated, self.total_freed, self.slot_reuses, self.bytes_allocated
        )
    }
}