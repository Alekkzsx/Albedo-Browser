//! Identificadores geracionais para a [`Arena`](super::Arena).
//!
//! Um [`ArenaId`] é um *handle* de 8 bytes, `Copy`, que referencia um valor dentro de
//! uma arena sem possuí-lo. Ele empacota um **índice** (posição do *slot*) e uma
//! **versão** (geração), e é essa combinação que torna as referências cíclicas do DOM
//! seguras e baratas.

use core::fmt;
use core::marker::PhantomData;
use core::num::NonZeroU64;

/// Handle estável e copiável para um valor dentro de uma [`Arena`](super::Arena).
///
/// # Layout interno
///
/// Os dois campos de 32 bits são empacotados em um único [`NonZeroU64`]:
///
/// ```text
///  63                               32 31                                0
/// ┌───────────────────────────────────┬───────────────────────────────────┐
/// │              versão               │               índice              │
/// └───────────────────────────────────┴───────────────────────────────────┘
/// ```
///
/// * **índice** — posição do *slot* no armazenamento denso da arena.
/// * **versão** — valor do contador monotônico global da arena no momento da alocação.
///
/// # Por que isso é seguro
///
/// A cada nova alocação a arena avança seu contador global de versão. Quando um *slot*
/// é liberado e reutilizado, ele recebe uma versão **maior** do que qualquer uma que já
/// teve. Assim, um `ArenaId` antigo que ainda aponte para aquele índice carregará uma
/// versão desatualizada e será rejeitado — eliminando *use-after-free* e acessos a nós
/// de uma página já descartada.
///
/// # Por que `NonZeroU64`
///
/// Garante que `Option<ArenaId<T>>` ocupe os mesmos 8 bytes de um `ArenaId<T>`
/// (otimização de *niche*). Isso é crítico: o DOM armazena milhões de ponteiros
/// opcionais (`parent`, `first_child`, `next_sibling`, ...), e dobrar o tamanho de cada
/// um custaria gigabytes de RAM.
pub struct ArenaId<T> {
    /// Chave empacotada: `(versão << 32) | índice`. Sempre diferente de zero.
    key: NonZeroU64,
    /// Ancora o tipo `T` sem impor posse nem variância indevida.
    ///
    /// Usamos `fn() -> T` (e não `T`) para que o handle seja covariante em `T` e
    /// permaneça `Send + Sync` para qualquer `T` — um identificador é apenas um número,
    /// não possui nem acessa o valor diretamente.
    _marker: PhantomData<fn() -> T>,
}

impl<T> ArenaId<T> {
    /// Cria um identificador a partir de um `índice` e uma `versão`.
    ///
    /// # Contrato interno
    ///
    /// `version` deve ser diferente de zero. A arena garante isso iniciando o contador
    /// em 1 e pulando o zero em caso de *wrap-around* (ver [`super::slab::next_version`]).
    #[inline]
    pub(crate) fn new(index: u32, version: u32) -> Self {
        debug_assert!(version != 0, "a versão de um ArenaId nunca pode ser zero");
        let key = (u64::from(version) << 32) | u64::from(index);
        // SAFETY: `version != 0` ⇒ `key >= 1 << 32` ⇒ `key != 0`.
        let key = unsafe { NonZeroU64::new_unchecked(key) };
        Self {
            key,
            _marker: PhantomData,
        }
    }

    /// Posição do *slot* no armazenamento da arena.
    #[inline]
    #[must_use]
    pub(crate) const fn index(self) -> u32 {
        self.key.get() as u32
    }

    /// Geração registrada no momento da alocação.
    #[inline]
    #[must_use]
    pub(crate) const fn version(self) -> u32 {
        (self.key.get() >> 32) as u32
    }

    /// Retorna a representação compacta bruta de 64 bits do identificador.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.key.get()
    }

    /// Reconstrói um `ArenaId` a partir de um valor bruto de 64 bits.
    /// Retorna `None` se o valor for zero.
    #[inline]
    pub fn from_raw(raw: u64) -> Option<Self> {
        NonZeroU64::new(raw).map(|key| Self {
            key,
            _marker: PhantomData,
        })
    }

    /// Converte este `ArenaId` no identificador global `NodeId` para IPC e telemetria.
    #[inline]
    pub fn to_node_id(self) -> crate::id::NodeId {
        crate::id::NodeId::from_non_zero(self.key)
    }

    /// Converte um `NodeId` global de volta em `ArenaId`, se válido.
    #[inline]
    pub fn from_node_id(node_id: crate::id::NodeId) -> Option<Self> {
        Self::from_raw(node_id.raw())
    }
}

impl<T> Clone for ArenaId<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ArenaId<T> {}

impl<T> PartialEq for ArenaId<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for ArenaId<T> {}

impl<T> core::hash::Hash for ArenaId<T> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state)
    }
}

impl<T> PartialOrd for ArenaId<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for ArenaId<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

impl<T> fmt::Debug for ArenaId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArenaId")
            .field("index", &self.index())
            .field("version", &self.version())
            .finish()
    }
}