//! # Detector de Portal Cativo em Redes Públicas (`CaptivePortalDetector`)
//!
//! Emite sondas HTTP canônicas leves (RFC 6585 / W3C) contra endpoints 204
//! para identificar se a conexão de rede está sendo interceptada por telas de login
//! de hotéis, aeroportos ou redes públicas, prevenindo erros falsos de SSL/TLS
//! e viabilizando a apresentação de uma interface de login para o usuário.

use crate::error::NetResult;
use crate::http::request::{Method, RedirectPolicy, RequestBuilder};
use crate::transport::TransportClient;
use http::StatusCode;
use parking_lot::RwLock;
use std::sync::Arc;
use url::Url;

/// Endpoint canônico padrão de validação de conectividade.
pub const DEFAULT_CAPTIVE_PORTAL_PROBE_URL: &str = "http://connectivitycheck.gstatic.com/generate_204";

/// Status da conectividade em relação a portais cativos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptivePortalStatus {
    /// A rede possui conectividade direta desimpedida com a internet (retornou 204 No Content).
    Online,
    /// A rede está interceptada por um portal cativo, com URL de login/autenticação detectada.
    BehindCaptivePortal(Url),
    /// Sem conectividade física ou falha de resolução.
    Offline,
    /// Sonda em execução ou status não determinado.
    Unknown,
}

/// Detector e monitor de portal cativo.
#[derive(Debug, Clone)]
pub struct CaptivePortalDetector {
    probe_url: Url,
    current_status: Arc<RwLock<CaptivePortalStatus>>,
}

impl Default for CaptivePortalDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptivePortalDetector {
    /// Cria uma nova instância de `CaptivePortalDetector` apontando para o probe canônico padrão.
    pub fn new() -> Self {
        Self::with_probe_url(Url::parse(DEFAULT_CAPTIVE_PORTAL_PROBE_URL).expect("URL canônica válida"))
    }

    /// Cria uma instância apontando para uma URL de probe personalizada.
    pub fn with_probe_url(probe_url: Url) -> Self {
        Self {
            probe_url,
            current_status: Arc::new(RwLock::new(CaptivePortalStatus::Unknown)),
        }
    }

    /// Retorna o último status detectado.
    pub fn status(&self) -> CaptivePortalStatus {
        self.current_status.read().clone()
    }

    /// Executa uma verificação ativa de portal cativo através do `TransportClient`.
    pub async fn check_portal(&self, transport: &TransportClient) -> NetResult<CaptivePortalStatus> {
        let req = RequestBuilder::new(self.probe_url.clone(), Method::GET)
            .redirect_policy(RedirectPolicy::Manual)
            .build();

        let status = match transport.execute(&req).await {
            Ok(resp) => {
                if resp.status == StatusCode::NO_CONTENT {
                    CaptivePortalStatus::Online
                } else if resp.status.is_redirection() {
                    let location_url = resp
                        .headers
                        .get(http::header::LOCATION)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|loc| self.probe_url.join(loc).ok())
                        .unwrap_or_else(|| self.probe_url.clone());
                    CaptivePortalStatus::BehindCaptivePortal(location_url)
                } else if resp.status == StatusCode::OK {
                    // Servidor respondeu 200 em vez de 204: portal interceptador servindo HTML de login
                    CaptivePortalStatus::BehindCaptivePortal(self.probe_url.clone())
                } else {
                    CaptivePortalStatus::Offline
                }
            }
            Err(_) => CaptivePortalStatus::Offline,
        };

        *self.current_status.write() = status.clone();
        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_captive_portal_initial_status() {
        let detector = CaptivePortalDetector::new();
        assert_eq!(detector.status(), CaptivePortalStatus::Unknown);
    }
}

