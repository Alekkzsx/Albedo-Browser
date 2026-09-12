//! # Integração de Service Workers (Hooks)
//!
//! Ponto de interceptação assíncrona para que a camada de JS (Service Workers)
//! possa capturar requisições de rede (evento `fetch`) antes de atingirem o cache HTTP
//! ou a rede, conforme a especificação de Service Workers do W3C.

use crate::error::NetResult;
use crate::request::Request;
use crate::response::Response;
use std::future::Future;
use std::pin::Pin;

/// Trait que define o contrato de interceptação para Service Workers.
/// A engine JS (`ace_js`) deve implementar este trait e registrá-lo no `ResourceFetcher`.
pub trait ServiceWorkerHook: Send + Sync {
    /// Dispara o evento de `fetch` no Service Worker.
    ///
    /// Se o Service Worker chamar `respondWith()`, este método deve retornar
    /// `Ok(Some(Response))`. Caso contrário (ex: requisição não interceptada ou fallback),
    /// deve retornar `Ok(None)` para que o `ResourceFetcher` prossiga normalmente.
    fn on_fetch(
        &self,
        req: &Request,
    ) -> Pin<Box<dyn Future<Output = NetResult<Option<Response>>> + Send + '_>>;
}
