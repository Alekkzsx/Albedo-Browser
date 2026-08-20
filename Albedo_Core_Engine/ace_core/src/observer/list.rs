//! # Lista de Observadores Reentrante (Chromium base::ObserverList Pattern)
//!
//! Gerenciador de eventos e callbacks seguro contra perigos de reentrância (adicionar ou remover
//! ouvintes durante a própria notificação de um evento).

use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static OBSERVER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

type Callback<E> = Arc<dyn Fn(&E) + Send + Sync>;

/// Uma lista thread-safe de observadores protegida contra reentrância via isolamento por snapshot.
pub struct ObserverList<E> {
    observers: RwLock<Vec<(u64, Callback<E>)>>,
}

impl<E> ObserverList<E> {
    /// Cria uma nova lista de observadores vazia.
    pub fn new() -> Self {
        Self {
            observers: RwLock::new(Vec::new()),
        }
    }

    /// Registra um novo callback observador. Retorna o ID único para posterior cancelamento (`unregister`).
    pub fn register<F>(&self, callback: F) -> u64
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        let id = OBSERVER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut list = self.observers.write();
        list.push((id, Arc::new(callback)));
        id
    }

    /// Remove um observador registrado pelo seu ID. Retorna `true` se encontrado e removido.
    pub fn unregister(&self, id: u64) -> bool {
        let mut list = self.observers.write();
        if let Some(pos) = list.iter().position(|(obs_id, _)| *obs_id == id) {
            list.remove(pos);
            true
        } else {
            false
        }
    }

    /// Notifica todos os observadores registrados com o evento especificado.
    ///
    /// Esta operação realiza uma cópia snapshot dos ponteiros `Arc` sob lock em stack (`InlineVec`)
    /// e executa os callbacks **fora do lock**, garantindo que qualquer modificação reentrante seja 100% segura
    /// e que listas com até 8 ouvintes tenham zero alocações de heap.
    pub fn notify(&self, event: &E) {
        let snapshot: crate::collections::InlineVec<Callback<E>, 8> = {
            let list = self.observers.read();
            let mut snap = crate::collections::InlineVec::with_capacity(list.len());
            for (_, cb) in list.iter() {
                snap.push(Arc::clone(cb));
            }
            snap
        };

        for callback in snapshot.iter() {
            callback(event);
        }
    }

    /// Retorna a quantidade de observadores atualmente ativos.
    #[inline]
    pub fn len(&self) -> usize {
        self.observers.read().len()
    }

    /// Retorna `true` se não houver observadores registrados.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.observers.read().is_empty()
    }

    /// Remove todos os observadores.
    pub fn clear(&self) {
        self.observers.write().clear();
    }
}

impl<E> Default for ObserverList<E> {
    fn default() -> Self {
        Self::new()
    }
}
