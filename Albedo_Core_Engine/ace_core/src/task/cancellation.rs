//! # Cancelamento Cooperativo e Composição de Sinais (WHATWG AbortSignal)
//!
//! Primitivas atômicas thread-safe para interrupção de tarefas assíncronas, requisições de rede
//! e composição de sinais de aborto (`AbortSignal.any()`).

use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

type Callback = Box<dyn Fn() + Send + Sync + 'static>;

struct CancellationTokenInner {
    is_cancelled: AtomicBool,
    callbacks: Mutex<Vec<Callback>>,
}

/// Token thread-safe para notificação e consulta de cancelamento cooperativo com alocação única de `Arc`.
#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<CancellationTokenInner>,
}

impl CancellationToken {
    /// Cria um novo token no estado não cancelado com alocação consolidada única.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CancellationTokenInner {
                is_cancelled: AtomicBool::new(false),
                callbacks: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Retorna `true` se o token já foi cancelado.
    #[inline(always)]
    pub fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled.load(Ordering::Acquire)
    }

    /// Cancela o token atomicamente e dispara todos os callbacks registrados.
    ///
    /// Se já estiver cancelado, esta chamada não tem efeito adicional.
    pub fn cancel(&self) {
        if !self.inner.is_cancelled.swap(true, Ordering::AcqRel) {
            let callbacks = {
                let mut guard = self.inner.callbacks.lock();
                std::mem::take(&mut *guard)
            };

            for callback in callbacks {
                callback();
            }
        }
    }

    /// Registra um callback para ser executado no momento em que o token for cancelado.
    ///
    /// Se o token **já estiver cancelado**, o callback é executado imediatamente.
    pub fn on_cancel<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        if self.is_cancelled() {
            callback();
        } else {
            let mut guard = self.inner.callbacks.lock();
            if self.inner.is_cancelled.load(Ordering::Acquire) {
                drop(guard);
                callback();
            } else {
                guard.push(Box::new(callback));
            }
        }
    }

    /// Cria um token filho que será cancelado se este token pai for cancelado.
    ///
    /// O cancelamento do filho **não** cancela o pai.
    pub fn child(&self) -> Self {
        let child_token = Self::new();
        let child_clone = child_token.clone();
        self.on_cancel(move || {
            child_clone.cancel();
        });
        child_token
    }

    /// Cria um token composto que é cancelado se **qualquer** um dos tokens fornecidos for cancelado.
    ///
    /// Implementa o comportamento canônico de `AbortSignal.any()` da especificação WHATWG DOM.
    pub fn any(tokens: &[&CancellationToken]) -> Self {
        let composite = Self::new();

        for token in tokens {
            if token.is_cancelled() {
                composite.cancel();
                return composite;
            }
            let comp_clone = composite.clone();
            token.on_cancel(move || {
                comp_clone.cancel();
            });
        }

        composite
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CancellationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancellationToken")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}
