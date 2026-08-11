// ============================================================================
// Albedo Core Engine (ACE)
// File: id.rs
// Description: Definidor de Identificadores Base (Newtypes) de uso global,
//              utilizando AtomicU64 para isolamento e thread-safety.
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Identificadores Primários da Engine
//! 
//! Para garantir total rastreabilidade (sem misturar u64 soltos pela codebase)
//! e segurança contra concorrência massiva de processos, o Albedo define um modelo
//! rigoroso de NewTypes (`TabId`, `NodeId`, etc.) baseados num gerador atômico seguro.

use std::sync::atomic::{AtomicU64, Ordering};

// ----------------------------------------------------------------------------
// Generator Macro
// ----------------------------------------------------------------------------

/// Macro interna para padronizar a criação de tipos fortemente tipados (Newtypes).
///
/// Implementa as traits fundamentais (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`)
/// e os métodos essenciais para uso thread-safe e inter-processo.
macro_rules! define_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(u64);

        impl $name {
            /// Gera um novo ID global e único, acessível de forma concorrente sem locks.
            pub fn new() -> Self {
                static COUNTER: AtomicU64 = AtomicU64::new(1);
                Self(COUNTER.fetch_add(1, Ordering::Relaxed))
            }

            /// Cria um ID diretamente a partir de um valor cru primitivo.
            /// 
            /// ⚠️ **Uso Restrito:** Deve ser usado primariamente no módulo IPC durante
            /// a deserialização de pacotes binários entre processos do SO.
            pub fn from_raw(id: u64) -> Self {
                Self(id)
            }

            /// Recupera o valor primário int para fins de logs ou serialização.
            pub fn raw(&self) -> u64 {
                self.0
            }
        }
        
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

// ----------------------------------------------------------------------------
// ID Definitions
// ----------------------------------------------------------------------------

define_id!(
    /// Identificador global absoluto de uma Janela ou Aba no aplicativo nativo.
    TabId
);

define_id!(
    /// Identificador rastreável de Requisições de Rede (Resource Fetch).
    RequestId
);

define_id!(
    /// Identificador único para cada nó dentro do modelo em memória (DOM e Render Tree).
    NodeId
);

define_id!(
    /// Identificador do Processo em sandboxing a nível de SO (PIDs virtuais do Albedo).
    ProcessId
);

define_id!(
    /// Identificador persistente em memórias flash / disco para Cookies.
    CookieId
);

define_id!(
    /// Chave identificadora para operações atômicas de Key/Value Store.
    StorageKey
);
