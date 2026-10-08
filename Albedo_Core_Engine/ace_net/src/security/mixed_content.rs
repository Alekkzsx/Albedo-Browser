//! # Fiscalização de Conteúdo Misto (W3C Mixed Content Level 2)
//!
//! Bloqueia ou promove automaticamente sub-recursos inseguros (`http://`) carregados
//! a partir de documentos seguros (`https://` ou localhost), prevenindo ataques de
//! Man-in-the-Middle (MitM) e degradação criptográfica da sessão do usuário.

use crate::error::{NetError, NetResult};
use crate::http::request::RequestDestination;
use ace_core::security::origin::Origin;
use url::Url;

/// Categoria do sub-recurso segundo o W3C Mixed Content Level 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MixedContentCategory {
    /// Conteúdo ativo que pode executar código ou alterar o DOM (Script, Style, Iframe, Fetch, Font).
    Active,
    /// Conteúdo passivo exibicional (Image, Media/Áudio/Vídeo).
    Passive,
}

impl MixedContentCategory {
    /// Classifica o destino semântico da requisição em Conteúdo Ativo ou Passivo.
    pub const fn from_destination(dest: RequestDestination) -> Self {
        match dest {
            RequestDestination::Script
            | RequestDestination::Style
            | RequestDestination::Font
            | RequestDestination::Document
            | RequestDestination::Fetch
            | RequestDestination::Other => Self::Active,

            RequestDestination::Image | RequestDestination::Media => Self::Passive,
        }
    }
}

/// Decisão tomada pelo motor de avaliação de conteúdo misto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MixedContentDecision {
    /// Requisição permitida sem modificações (destino seguro ou contexto pai não criptografado).
    Allowed,
    /// URL insegura promovida silenciosamente para `https://` (CSP upgrade ou autoupgrade de mídia/imagem).
    Upgraded(Url),
    /// Requisição bloqueada por representar conteúdo misto inseguro proibido.
    Blocked(String),
}

/// Avalia se uma requisição de sub-recurso viola a política de Mixed Content do documento superior.
///
/// # Regras do W3C Mixed Content Level 2:
/// 1. Se o documento pai não for seguro (ex: HTTP simples), qualquer sub-recurso HTTP/HTTPS é permitido.
/// 2. Se o documento pai for seguro (`https:` ou localhost):
///    - Se a URL de destino for `https:`, `data:`, ou `blob:`, é permitida.
///    - Se a URL de destino for `http:`:
///      - Se `upgrade_insecure_requests` estiver ativo (CSP) OU se for conteúdo passivo (auto-upgrade moderno):
///        Promove a URL para `https://`.
///      - Se for conteúdo ativo e não elegível a upgrade: Bloqueia imediatamente com erro de segurança.
pub fn evaluate_mixed_content(
    parent_origin: Option<&Origin>,
    target_url: &Url,
    dest: RequestDestination,
    csp_upgrade_insecure: bool,
) -> MixedContentDecision {
    let parent = match parent_origin {
        Some(origin) => origin,
        None => return MixedContentDecision::Allowed,
    };

    // Se o documento pai não é seguro, não há exigência de Mixed Content
    if !parent.is_secure() {
        return MixedContentDecision::Allowed;
    }

    let scheme = target_url.scheme();

    // Se o destino já é seguro ou esquema seguro de dados em memória
    if scheme == "https" || scheme == "data" || scheme == "blob" || scheme == "about" {
        return MixedContentDecision::Allowed;
    }

    // Se o destino for localhost, também é considerado seguro
    if let Some(host_str) = target_url.host_str() {
        if host_str == "localhost" || host_str == "127.0.0.1" || host_str == "::1" {
            return MixedContentDecision::Allowed;
        }
    }

    // Caso seja HTTP inseguro requisitado por contexto seguro:
    if scheme == "http" {
        let category = MixedContentCategory::from_destination(dest);

        // Se CSP exige upgrade para todos os sub-recursos, ou se for passivo (auto-upgrade padrão do Chrome/W3C Level 2)
        if csp_upgrade_insecure || category == MixedContentCategory::Passive {
            let mut upgraded_url = target_url.clone();
            if upgraded_url.set_scheme("https").is_ok() {
                // Se a porta for a padrão 80 de HTTP, redefine para a padrão 443 de HTTPS
                if upgraded_url.port() == Some(80) {
                    let _ = upgraded_url.set_port(None);
                }
                return MixedContentDecision::Upgraded(upgraded_url);
            }
        }

        // Conteúdo ativo em HTTP sem upgrade é terminantemente bloqueado
        return MixedContentDecision::Blocked(format!(
            "Conteúdo Misto Ativo bloqueado: destino '{:?}' em '{}' solicitado por contexto seguro",
            dest, target_url
        ));
    }

    MixedContentDecision::Allowed
}

/// Aplica a avaliação de Mixed Content a uma URL e opcionalmente substitui a URL da requisição se promovida.
pub fn check_and_apply_mixed_content(
    parent_origin: Option<&Origin>,
    target_url: &mut Url,
    dest: RequestDestination,
    csp_upgrade: bool,
) -> NetResult<()> {
    match evaluate_mixed_content(parent_origin, target_url, dest, csp_upgrade) {
        MixedContentDecision::Allowed => Ok(()),
        MixedContentDecision::Upgraded(new_url) => {
            *target_url = new_url;
            Ok(())
        }
        MixedContentDecision::Blocked(reason) => Err(NetError::SecurityViolation(reason)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insecure_parent_allows_anything() {
        let parent = Origin::parse("http://insecure.site").unwrap();
        let target = Url::parse("http://insecure.site/api.js").unwrap();

        let decision = evaluate_mixed_content(Some(&parent), &target, RequestDestination::Script, false);
        assert_eq!(decision, MixedContentDecision::Allowed);
    }

    #[test]
    fn test_secure_parent_upgrades_passive_content() {
        let parent = Origin::parse("https://secure.site").unwrap();
        let target = Url::parse("http://cdn.site/photo.png").unwrap();

        let decision = evaluate_mixed_content(Some(&parent), &target, RequestDestination::Image, false);
        match decision {
            MixedContentDecision::Upgraded(url) => {
                assert_eq!(url.scheme(), "https");
                assert_eq!(url.as_str(), "https://cdn.site/photo.png");
            }
            other => panic!("Esperado Upgraded, obtido {:?}", other),
        }
    }

    #[test]
    fn test_secure_parent_blocks_active_content_without_csp() {
        let parent = Origin::parse("https://secure.site").unwrap();
        let script_target = Url::parse("http://insecure.site/tracking.js").unwrap();

        let decision = evaluate_mixed_content(Some(&parent), &script_target, RequestDestination::Script, false);
        match decision {
            MixedContentDecision::Blocked(msg) => {
                assert!(msg.contains("Conteúdo Misto Ativo"));
            }
            other => panic!("Esperado Blocked, obtido {:?}", other),
        }
    }

    #[test]
    fn test_secure_parent_upgrades_active_content_with_csp() {
        let parent = Origin::parse("https://secure.site").unwrap();
        let script_target = Url::parse("http://cdn.site/script.js").unwrap();

        let decision = evaluate_mixed_content(Some(&parent), &script_target, RequestDestination::Script, true);
        match decision {
            MixedContentDecision::Upgraded(url) => {
                assert_eq!(url.scheme(), "https");
                assert_eq!(url.as_str(), "https://cdn.site/script.js");
            }
            other => panic!("Esperado Upgraded via CSP, obtido {:?}", other),
        }
    }

    #[test]
    fn test_check_and_apply_mutation() {
        let parent = Origin::parse("https://secure.site").unwrap();
        let mut target = Url::parse("http://cdn.site/img.jpg").unwrap();

        let res = check_and_apply_mixed_content(Some(&parent), &mut target, RequestDestination::Image, false);
        assert!(res.is_ok());
        assert_eq!(target.as_str(), "https://cdn.site/img.jpg");
    }
}

