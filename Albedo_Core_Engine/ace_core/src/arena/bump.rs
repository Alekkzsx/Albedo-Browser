//! Arena heterogênea de *bump allocation* para dados auxiliares.
//!
//! Enquanto a [`Arena`](super::Arena) homogênea guarda os **nós** (que precisam se
//! referenciar por ID), a [`BumpArena`] guarda os **payloads**: strings de atributos,
//! fatias de bytes, valores temporários do parser. Esses dados não precisam de
//! identificadores — só precisam viver exatamente tanto quanto a página.
//!
//! A alocação em si é delegada à crate madura [`bumpalo`], em conformidade com o
//! Paradigma Pragmático: infraestrutura de alocação é fundação; a orquestração é nossa.

use super::stats::ArenaStats;
use bumpalo::Bump;
use core::sync::atomic::{AtomicU64, Ordering};

/// Arena de *bump allocation* para dados heterogêneos.
///
/// Aloca qualquer tipo `T` e retorna `&mut T` com lifetime atrelado à arena. É a
/// ferramenta certa para dados que não se referenciam entre si e morrem juntos no
/// [`reset`](BumpArena::reset).
///
/// # Exemplo
///
/// ```
/// use ace_core::arena::BumpArena;
///
/// let arena = BumpArena::new();
/// let nome: &mut str = arena.alloc_str("class");
/// let valor: &mut str = arena.alloc_str("botao-primario");
/// assert_eq!(nome, "class");
/// assert_eq!(valor, "botao-primario");
/// ```
#[derive(Debug)]
pub struct BumpArena {
    /// Alocador subjacente (fundação).
    bump: Bump,
    /// Contador de alocações. `AtomicU64` permite manter `alloc` como `&self`,
    /// casando com a assinatura do `bumpalo` e permitindo uso compartilhado.
    allocations: AtomicU64,
}

impl BumpArena {
    /// Cria uma arena de bump vazia.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bump: Bump::new(),
            allocations: AtomicU64::new(0),
        }
    }

    /// Cria uma arena com `capacity` bytes pré-alocados.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bump: Bump::with_capacity(capacity),
            allocations: AtomicU64::new(0),
        }
    }

    /// Aloca `value` e retorna uma referência válida até o [`reset`](BumpArena::reset).
    ///
    /// **Atenção:** assim como no `bumpalo`, o `Drop` de `T` **não** é executado no
    /// `reset`. Para payloads do DOM (dados planos, strings, IDs) isso é exatamente o
    /// comportamento desejado — e a fonte da performance.
    #[inline]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        self.bump.alloc(value)
    }

    /// Aloca uma cópia de uma `str`. Conveniência para atributos e texto.
    #[inline]
    pub fn alloc_str(&self, s: &str) -> &mut str {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        self.bump.alloc_str(s)
    }

    /// Aloca uma cópia de uma fatia. Conveniência para buffers e dados binários.
    #[inline]
    pub fn alloc_slice_copy<U: Copy>(&self, slice: &[U]) -> &mut [U] {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        self.bump.alloc_slice_copy(slice)
    }

    /// Descarta todas as alocações de uma só vez, reaproveitando a memória dos blocos.
    pub fn reset(&mut self) {
        self.bump.reset();
        self.allocations.store(0, Ordering::Relaxed);
    }

    /// Coleta métricas de uso.
    #[must_use]
    pub fn stats(&self) -> ArenaStats {
        let allocated = self.allocations.load(Ordering::Relaxed);
        ArenaStats {
            live: allocated as usize,
            capacity: self.bump.allocated_bytes(),
            total_allocated: allocated,
            total_freed: 0,
            slot_reuses: 0,
            bytes_allocated: self.bump.allocated_bytes(),
        }
    }
}

impl Default for BumpArena {
    fn default() -> Self {
        Self::new()
    }
}