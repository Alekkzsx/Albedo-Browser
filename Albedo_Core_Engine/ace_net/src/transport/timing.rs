use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Estrutura para coletar métricas granulares da conexão durante a execução de uma requisição.
#[derive(Debug, Default, Clone)]
pub struct ConnectionTiming {
    pub dns_start: Option<Instant>,
    pub dns_duration: Option<Duration>,
    pub tcp_start: Option<Instant>,
    pub tcp_duration: Option<Duration>,
    pub tls_start: Option<Instant>,
    pub tls_duration: Option<Duration>,
}

tokio::task_local! {
    /// Armazena as métricas da conexão em andamento na task atual.
    pub static CONNECTION_TIMING: Arc<Mutex<ConnectionTiming>>;
}

use hyper::Uri;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower_service::Service;

#[derive(Clone)]
pub struct TimingConnector<C> {
    inner: C,
    is_tls: bool,
}

impl<C> TimingConnector<C> {
    pub fn new(inner: C, is_tls: bool) -> Self {
        Self { inner, is_tls }
    }
}

impl<C> Service<Uri> for TimingConnector<C>
where
    C: Service<Uri> + Clone + Send + 'static,
    C::Response: Send,
    C::Error: Send,
    C::Future: Send + 'static,
{
    type Response = C::Response;
    type Error = C::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Uri) -> Self::Future {
        let mut inner = self.inner.clone();
        let is_tls = self.is_tls;

        Box::pin(async move {
            let start = Instant::now();
            if let Ok(timing_arc) = CONNECTION_TIMING.try_with(|t| t.clone()) {
                let mut guard = timing_arc.lock().await;
                if is_tls && guard.tls_start.is_none() {
                    guard.tls_start = Some(start);
                } else if !is_tls && guard.tcp_start.is_none() {
                    guard.tcp_start = Some(start);
                }
            }

            let result = inner.call(req).await;
            
            let duration = start.elapsed();
            if let Ok(timing_arc) = CONNECTION_TIMING.try_with(|t| t.clone()) {
                let mut guard = timing_arc.lock().await;
                if is_tls {
                    // tls_duration é a duração total menos a duração de DNS e TCP que aconteceram dentro dela
                    let tcp_total = guard.tcp_duration.unwrap_or_default() + guard.dns_duration.unwrap_or_default();
                    guard.tls_duration = Some(duration.saturating_sub(tcp_total));
                } else {
                    // tcp_duration é a duração total TCP menos a duração de DNS que aconteceu dentro dela
                    let dns_total = guard.dns_duration.unwrap_or_default();
                    guard.tcp_duration = Some(duration.saturating_sub(dns_total));
                }
            }
            result
        })
    }
}
