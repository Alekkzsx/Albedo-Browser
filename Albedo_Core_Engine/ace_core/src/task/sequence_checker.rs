//! # Verificador de Afinidade de Thread e Sequência (Chromium base::SequenceChecker Pattern)
//!
//! Em motores de navegador, a grande maioria dos subsistemas de renderização (DOM, Style, Layout, EventLoop)
//! não são thread-safe por design para evitar contenção e overhead de mutexes.
//!
//! O `SequenceChecker` garante em tempo de desenvolvimento (`debug_assertions`) que estruturas de dados
//! sensíveis sejam acessadas apenas pela thread ou sequência de execução à qual foram vinculadas.
//! Em builds de produção (`release`), o compilador otimiza o `SequenceChecker` para **zero bytes de memória**
//! e chamadas inline no-op de **zero ciclos de CPU**.

use std::fmt;

#[cfg(debug_assertions)]
use std::cell::Cell;
#[cfg(debug_assertions)]
use std::thread::{current, ThreadId};

/// Verificador de afinidade de thread para proteção de estruturas não-thread-safe.
pub struct SequenceChecker {
    #[cfg(debug_assertions)]
    bound_thread: Cell<Option<ThreadId>>,
}

impl Default for SequenceChecker {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl SequenceChecker {
    /// Cria um novo `SequenceChecker` vinculado imediatamente à thread atual.
    #[inline]
    pub fn new() -> Self {
        #[cfg(debug_assertions)]
        {
            Self {
                bound_thread: Cell::new(Some(current().id())),
            }
        }
        #[cfg(not(debug_assertions))]
        {
            Self {}
        }
    }

    /// Cria um novo `SequenceChecker` desacoplado que se vinculará à primeira thread que o consultar.
    #[inline]
    pub const fn new_unbound() -> Self {
        #[cfg(debug_assertions)]
        {
            Self {
                bound_thread: Cell::new(None),
            }
        }
        #[cfg(not(debug_assertions))]
        {
            Self {}
        }
    }

    /// Retorna `true` se a chamada atual estiver ocorrendo na sequência/thread correta.
    #[inline]
    pub fn called_on_valid_sequence(&self) -> bool {
        #[cfg(debug_assertions)]
        {
            let current_id = current().id();
            match self.bound_thread.get() {
                Some(bound) => bound == current_id,
                None => {
                    self.bound_thread.set(Some(current_id));
                    true
                }
            }
        }
        #[cfg(not(debug_assertions))]
        {
            true
        }
    }

    /// Dispara pânico se a execução estiver ocorrendo em uma thread diferente da vinculada.
    #[inline]
    #[track_caller]
    pub fn assert_called_on_valid_sequence(&self) {
        #[cfg(debug_assertions)]
        {
            assert!(
                self.called_on_valid_sequence(),
                "Violação de Concorrência: Estrutura acessada em thread distinta da sequência proprietária!"
            );
        }
    }

    /// Desvincula o checker permitindo que seja transferido para outra thread.
    #[inline]
    pub fn detach(&self) {
        #[cfg(debug_assertions)]
        {
            self.bound_thread.set(None);
        }
    }
}

impl fmt::Debug for SequenceChecker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("SequenceChecker");
        #[cfg(debug_assertions)]
        d.field("bound_thread", &self.bound_thread.get());
        d.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_sequence_checker_same_thread() {
        let checker = SequenceChecker::new();
        assert!(checker.called_on_valid_sequence());
        checker.assert_called_on_valid_sequence();
    }

    #[test]
    fn test_sequence_checker_cross_thread_violation() {
        let checker = Arc::new(SequenceChecker::new());
        assert!(checker.called_on_valid_sequence());

        let checker_clone = Arc::clone(&checker);
        let handle = thread::spawn(move || {
            // Em outra thread, deve retornar false
            checker_clone.called_on_valid_sequence()
        });

        let is_valid_in_child = handle.join().unwrap();
        #[cfg(debug_assertions)]
        assert!(!is_valid_in_child);
        #[cfg(not(debug_assertions))]
        assert!(is_valid_in_child);
    }
}
