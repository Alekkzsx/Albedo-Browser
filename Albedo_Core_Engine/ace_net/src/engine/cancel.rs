//! # Registro e Controle de Cancelamento de Requisições
//!
//! Permite o cancelamento atômico e colaborativo de requisições de rede em voo
//! através de `RequestId` e `CancellationToken` (WHATWG AbortController).

use ace_core::id::RequestId;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use tokio_util::sync::CancellationToken;

/// Registro central de tokens de cancelamento indexados por `RequestId`.
#[derive(Debug, Default)]
pub struct CancellationRegistry {
    tokens: RwLock<FxHashMap<RequestId, CancellationToken>>,
}

impl CancellationRegistry {
    /// Cria uma nova instância de `CancellationRegistry`.
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(FxHashMap::default()),
        }
    }

    /// Registra um token existente associado a um `RequestId`.
    pub fn register(&self, id: RequestId, token: CancellationToken) {
        self.tokens.write().insert(id, token);
    }

    /// Obtém o token existente para o `RequestId` ou cria e registra um novo.
    pub fn get_or_create(&self, id: RequestId) -> CancellationToken {
        let mut map = self.tokens.write();
        map.entry(id).or_default().clone()
    }

    /// Dispara o cancelamento da requisição associada ao `RequestId`.
    /// Retorna `true` se a requisição existia e foi sinalizada.
    pub fn cancel(&self, id: RequestId) -> bool {
        let map = self.tokens.read();
        if let Some(token) = map.get(&id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    /// Verifica se uma requisição já foi cancelada.
    pub fn is_cancelled(&self, id: RequestId) -> bool {
        self.tokens
            .read()
            .get(&id)
            .map(|t| t.is_cancelled())
            .unwrap_or(false)
    }

    /// Remove a requisição do registro após sua conclusão para evitar vazamentos de memória.
    pub fn unregister(&self, id: RequestId) {
        self.tokens.write().remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancellation_registry_flow() {
        let registry = CancellationRegistry::new();
        let id = RequestId::new();

        let token = registry.get_or_create(id);
        assert!(!token.is_cancelled());
        assert!(!registry.is_cancelled(id));

        let cancelled = registry.cancel(id);
        assert!(cancelled);
        assert!(token.is_cancelled());
        assert!(registry.is_cancelled(id));

        registry.unregister(id);
        assert!(!registry.cancel(id));
    }
}
