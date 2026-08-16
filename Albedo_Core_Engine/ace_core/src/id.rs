use std::sync::atomic::{AtomicU64, Ordering};
use std::fmt;

/// Gera um novo ID único globalmente durante a execução do processo.
fn next_global_id() -> u64 {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

macro_rules! define_id_type {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            /// Cria um novo ID único.
            pub fn new() -> Self {
                Self(next_global_id())
            }
            
            /// Cria um ID a partir de um valor bruto (apenas para deserialização/testes).
            pub const fn from_raw(id: u64) -> Self {
                Self(id)
            }

            /// Retorna o valor bruto do ID.
            pub const fn raw(&self) -> u64 {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
        
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_id_type!(NodeId, "Identificador único para um nó na árvore DOM ou Render Tree.");
define_id_type!(TabId, "Identificador único para uma aba (Tab) do navegador.");
define_id_type!(ProcessId, "Identificador único para um processo do navegador.");
define_id_type!(RequestId, "Identificador único para uma requisição de rede.");
