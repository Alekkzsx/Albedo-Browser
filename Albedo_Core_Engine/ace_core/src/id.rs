//! # Identificadores Globais Fortemente Tipados com Niche Optimization
//!
//! Este módulo provê Newtypes atômicos fortemente tipados baseados em `NonZeroU64`.
//! O uso de `NonZeroU64` garante **Discriminant Elision (Niche Optimization)**:
//! `Option<NodeId>`, `Option<TabId>`, etc., ocupam **exatos 8 bytes** na memória em vez de 16 bytes,
//! reduzindo pela metade o consumo de memória de ponteiros na árvore DOM e no grafo de Render.

use std::fmt;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

/// Função interna para geração lock-free de IDs únicos globais garantidamente não nulos.
#[inline]
fn next_global_id() -> NonZeroU64 {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    match NonZeroU64::new(id) {
        Some(nz) => nz,
        None => NonZeroU64::MIN,
    }
}

/// Macro utilitária para gerar structs Newtype de IDs com Niche Optimization e conversões automáticas.
macro_rules! define_id_type {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(std::num::NonZeroU64);

        impl $name {
            /// Cria um novo identificador garantidamente único durante a vida do processo.
            #[inline]
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self(next_global_id())
            }

            /// Cria o identificador a partir de um `NonZeroU64`.
            #[inline]
            pub const fn from_non_zero(id: std::num::NonZeroU64) -> Self {
                Self(id)
            }

            /// Restaura um identificador a partir de um valor bruto de 64 bits.
            /// Retorna `None` se o valor for zero.
            #[inline]
            pub const fn from_raw(id: u64) -> Option<Self> {
                match std::num::NonZeroU64::new(id) {
                    Some(nz) => Some(Self(nz)),
                    None => None,
                }
            }

            /// Restaura um identificador a partir de um valor bruto, convertendo zero para `NonZeroU64::MIN`.
            #[inline]
            pub const fn from_raw_unchecked(id: u64) -> Self {
                match std::num::NonZeroU64::new(id) {
                    Some(nz) => Self(nz),
                    None => Self(std::num::NonZeroU64::MIN),
                }
            }

            /// Extrai a representação primária de 64 bits do identificador.
            #[inline]
            pub const fn raw(&self) -> u64 {
                self.0.get()
            }

            /// Extrai o `NonZeroU64` interno.
            #[inline]
            pub const fn non_zero(&self) -> std::num::NonZeroU64 {
                self.0
            }
        }

        impl Default for $name {
            #[inline]
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<std::num::NonZeroU64> for $name {
            #[inline]
            fn from(id: std::num::NonZeroU64) -> Self {
                Self::from_non_zero(id)
            }
        }

        impl From<u64> for $name {
            #[inline]
            fn from(id: u64) -> Self {
                Self::from_raw_unchecked(id)
            }
        }

        impl From<$name> for u64 {
            #[inline]
            fn from(id: $name) -> Self {
                id.raw()
            }
        }

        impl From<$name> for std::num::NonZeroU64 {
            #[inline]
            fn from(id: $name) -> Self {
                id.non_zero()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0.get())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0.get())
            }
        }
    };
}

define_id_type!(NodeId, "Identificador único global para um nó na Árvore DOM ou Render Tree. Previne colisões e vazamentos entre iframes.");
define_id_type!(TabId, "Identificador de alto nível para uma Aba (Contexto de Navegação Top-Level) na interface do navegador.");
define_id_type!(ProcessId, "Identificador para rastrear processos filhos na arquitetura multiprocesso isolada (Network Process, GPU Process, etc).");
define_id_type!(RequestId, "Rastreia unicamente o ciclo de vida de um fetch na rede, desde o DNS até o recebimento dos bytes finais do body.");
define_id_type!(FrameId, "Identificador único para um frame ou iframe dentro da hierarquia da janela.");
define_id_type!(LayerId, "Identificador para uma camada de pintura isolada no Compositor de GPU.");
define_id_type!(ScriptId, "Identificador de um script ECMAScript compilado ou em execução.");
define_id_type!(StyleSheetId, "Identificador único para uma folha de estilos CSS parsed na Render Tree.");
define_id_type!(TaskId, "Identificador de tarefa ou timer agendado no Event Loop (usado para cancelamento via clearTimeout).");
define_id_type!(CookieId, "Identificador único de entrada persistente no Cookie Jar.");
define_id_type!(StorageKeyId, "Identificador de chave de particionamento para LocalStorage/SessionStorage.");
define_id_type!(ContextId, "Identificador de contexto de renderização (Canvas2D / WebGL / Surface).");
define_id_type!(ChannelId, "Identificador único de canal bidirecional no protocolo IPC.");
