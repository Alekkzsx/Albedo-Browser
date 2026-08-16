//! Identificadores globais e fracamente acoplados (Newtypes).
//!
//! Este módulo provê tipos fortemente tipados para identificar recursos únicos
//! dentro do navegador (ex: nós do DOM, requisições de rede). 
//! O uso de "Newtypes" (`struct Nome(u64)`) previne que um ID de Aba seja acidentalmente
//! passado para uma função que espera um ID de Nó, garantindo segurança estrita no tempo de compilação.

use std::sync::atomic::{AtomicU64, Ordering};
use std::fmt;

/// Função interna para geração lock-free de IDs únicos globais.
/// Utiliza operações atômicas relaxadas para máxima performance, 
/// garantindo que nenhum ID seja gerado duas vezes na mesma execução do navegador.
#[inline]
fn next_global_id() -> u64 {
    // Inicializamos em 1 para reservar o 0 como possível valor "nulo" no futuro, se necessário.
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Macro utilitária para gerar structs Newtype de IDs com toda a infraestrutura necessária.
/// 
/// O macro implementa automaticamente os traits `Clone`, `Copy`, `PartialEq`, `Eq`, 
/// `PartialOrd`, `Ord`, `Hash`, `Debug` e `Display`, tornando o ID completamente
/// interoperável com coleções do Rust (como `HashMap` e `BTreeMap`).
macro_rules! define_id_type {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            /// Cria um novo identificador garantidamente único durante a vida do processo.
            /// 
            /// # Performance
            /// Ocupa apenas o tempo de uma instrução atômica rápida (`fetch_add`).
            #[inline]
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self(next_global_id())
            }
            
            /// Restaura um identificador a partir de um valor de 64 bits.
            /// 
            /// **Atenção:** Só deve ser utilizado para processos de deserialização (ex: serialização IPC) 
            /// ou mockagem determinística em testes unitários.
            #[inline]
            pub const fn from_raw(id: u64) -> Self {
                Self(id)
            }

            /// Extrai a representação primária de 64 bits do identificador.
            /// Útil para roteamento em FFI, serialização IPC ou logs binários.
            #[inline]
            pub const fn raw(&self) -> u64 {
                self.0
            }
        }

        impl Default for $name {
            #[inline]
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

define_id_type!(NodeId, "Identificador único global para um nó na Árvore DOM ou Render Tree. Previne colisões e vazamentos entre iframes.");
define_id_type!(TabId, "Identificador de alto nível para uma Aba (Contexto de Navegação Top-Level) na interface do navegador.");
define_id_type!(ProcessId, "Identificador para rastrear processos filhos na arquitetura multiprocesso isolada (Network Process, GPU Process, etc).");
define_id_type!(RequestId, "Rastreia unicamente o ciclo de vida de um fetch na rede, desde o DNS até o recebimento dos bytes finais do body.");
