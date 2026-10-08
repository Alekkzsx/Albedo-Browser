//! # Políticas de Isolamento de Origem e Anti-Spectre (CORP, COOP e COEP)
//!
//! Implementa os mecanismos de contenção de fronteira de segurança entre origens
//! da W3C e WHATWG para mitigação de vazamento de memória e ataques de canal lateral:
//! - **CORP (Cross-Origin-Resource-Policy):** Bloqueio de entrega de sub-recursos a origens não autorizadas.
//! - **COOP (Cross-Origin-Opener-Policy):** Isolamento de grupo de navegação entre janelas e abas.
//! - **COEP (Cross-Origin-Embedder-Policy):** Requisito de autorização explícita para embutimento de sub-recursos.

use crate::error::{NetError, NetResult};
use ace_core::security::origin::Origin;
use http::HeaderMap;
use url::Url;

/// Política de entrega de sub-recursos definida pelo cabeçalho `Cross-Origin-Resource-Policy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CorpPolicy {
    /// Recurso livre para consumo por qualquer origem.
    #[default]
    CrossOrigin,
    /// Permitido apenas para origens que compartilham o mesmo eTLD+1 (Same-Site).
    SameSite,
    /// Permitido estritamente para a mesma origem exata (Same-Origin: scheme, host, port).
    SameOrigin,
}

impl CorpPolicy {
    /// Analisa o valor textual do cabeçalho `Cross-Origin-Resource-Policy`.
    pub fn parse(val: &str) -> Option<Self> {
        let trimmed = val.trim();
        if trimmed.eq_ignore_ascii_case("same-origin") {
            Some(Self::SameOrigin)
        } else if trimmed.eq_ignore_ascii_case("same-site") {
            Some(Self::SameSite)
        } else if trimmed.eq_ignore_ascii_case("cross-origin") {
            Some(Self::CrossOrigin)
        } else {
            None
        }
    }
}

/// Diretiva de isolamento de grupo de abas/janelas (`Cross-Origin-Opener-Policy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CoopPolicy {
    /// Comportamento padrão da web sem isolamento de opener.
    #[default]
    UnsafeNone,
    /// Mantém referências apenas para popups abertos pela mesma origem.
    SameOriginAllowPopups,
    /// Isola completamente o contexto de navegação da janela de qualquer outra origem.
    SameOrigin,
    /// Permite comunicação restrita (postMessage) com janelas cross-origin sob COOP.
    RestrictProperties,
}

impl CoopPolicy {
    /// Analisa o valor do cabeçalho `Cross-Origin-Opener-Policy`.
    pub fn parse(val: &str) -> Self {
        let trimmed = val.trim();
        if trimmed.eq_ignore_ascii_case("same-origin") {
            Self::SameOrigin
        } else if trimmed.eq_ignore_ascii_case("same-origin-allow-popups") {
            Self::SameOriginAllowPopups
        } else if trimmed.eq_ignore_ascii_case("restrict-properties") {
            Self::RestrictProperties
        } else {
            Self::UnsafeNone
        }
    }
}

/// Diretiva de restrição de embutimento de recursos (`Cross-Origin-Embedder-Policy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CoepPolicy {
    /// Permite embutir qualquer recurso cross-origin sem restrições.
    #[default]
    UnsafeNone,
    /// Exige que todo recurso embutido possua CORP (`cross-origin`) ou permissão CORS.
    RequireCorp,
    /// Carrega recursos de terceiros omitindo cookies e credenciais automaticamente.
    Credentialless,
}

impl CoepPolicy {
    /// Analisa o valor do cabeçalho `Cross-Origin-Embedder-Policy`.
    pub fn parse(val: &str) -> Self {
        let trimmed = val.trim();
        if trimmed.eq_ignore_ascii_case("require-corp") {
            Self::RequireCorp
        } else if trimmed.eq_ignore_ascii_case("credentialless") {
            Self::Credentialless
        } else {
            Self::UnsafeNone
        }
    }
}

/// Estado agregado de isolamento cruzado de uma janela/documento web.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct IsolationContext {
    pub coop: CoopPolicy,
    pub coep: CoepPolicy,
}

impl IsolationContext {
    /// Cria um novo contexto de isolamento com as políticas fornecidas.
    pub const fn new(coop: CoopPolicy, coep: CoepPolicy) -> Self {
        Self { coop, coep }
    }

    /// Avalia os cabeçalhos de uma resposta HTTP para determinar o contexto de isolamento do documento.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        let coop = headers
            .get("cross-origin-opener-policy")
            .and_then(|v| v.to_str().ok())
            .map(CoopPolicy::parse)
            .unwrap_or_default();

        let coep = headers
            .get("cross-origin-embedder-policy")
            .and_then(|v| v.to_str().ok())
            .map(CoepPolicy::parse)
            .unwrap_or_default();

        Self { coop, coep }
    }

    /// Retorna `true` se o contexto for considerado "Cross-Origin Isolated"
    /// segundo as especificações da W3C (habilitando recursos de alta performance como `SharedArrayBuffer`).
    pub fn is_cross_origin_isolated(&self) -> bool {
        self.coop == CoopPolicy::SameOrigin
            && (self.coep == CoepPolicy::RequireCorp || self.coep == CoepPolicy::Credentialless)
    }
}

/// Extrai a política de CORP presente nos cabeçalhos de resposta HTTP, se houver.
pub fn extract_corp_policy(headers: &HeaderMap) -> Option<CorpPolicy> {
    headers
        .get("cross-origin-resource-policy")
        .and_then(|v| v.to_str().ok())
        .and_then(CorpPolicy::parse)
}

/// Valida a entrega de um sub-recurso contra a política CORP (`Cross-Origin-Resource-Policy`).
///
/// # Parâmetros
/// - `initiator_origin`: A origem que requisitou o sub-recurso (se `None`, assume navegação de topo ou contexto opaco seguro).
/// - `resource_url`: A URL final de onde o recurso foi obtido.
/// - `corp_header`: A política extraída da resposta do recurso.
///
/// # Erros
/// Retorna `NetError::SecurityViolation` se a política CORP rejeitar a entrega à origem solicitante.
pub fn validate_corp(
    initiator_origin: Option<&Origin>,
    resource_url: &Url,
    corp_header: Option<CorpPolicy>,
) -> NetResult<()> {
    let policy = match corp_header {
        Some(p) => p,
        None => return Ok(()), // Sem cabeçalho CORP, a entrega não é restringida por esta camada
    };

    if policy == CorpPolicy::CrossOrigin {
        return Ok(());
    }

    let initiator = match initiator_origin {
        Some(origin) => origin,
        None => return Ok(()), // Contexto sem origem iniciadora identificada
    };

    // Origens opacas são bloqueadas em recursos com restrição Same-Origin ou Same-Site
    if initiator.is_opaque() {
        return Err(NetError::SecurityViolation(
            "CORP violado: origem opaca bloqueada de acessar recurso restrito por CORP".into(),
        ));
    }

    let target_origin = Origin::parse(resource_url.as_str())
        .map_err(|e| NetError::SecurityViolation(format!("Origem inválida na URL de destino: {}", e)))?;

    match policy {
        CorpPolicy::SameOrigin => {
            if !initiator.same_origin(&target_origin) {
                return Err(NetError::SecurityViolation(format!(
                    "CORP violado: recurso em '{}' exige 'same-origin', mas foi solicitado por '{}'",
                    target_origin.ascii_serialization(),
                    initiator.ascii_serialization()
                )));
            }
        }
        CorpPolicy::SameSite => {
            if !initiator.is_same_site(&target_origin) {
                return Err(NetError::SecurityViolation(format!(
                    "CORP violado: recurso em '{}' exige 'same-site', mas foi solicitado por '{}'",
                    target_origin.ascii_serialization(),
                    initiator.ascii_serialization()
                )));
            }
        }
        CorpPolicy::CrossOrigin => unreachable!(),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corp_policy_parsing() {
        assert_eq!(CorpPolicy::parse("same-origin"), Some(CorpPolicy::SameOrigin));
        assert_eq!(CorpPolicy::parse(" SAME-SITE "), Some(CorpPolicy::SameSite));
        assert_eq!(CorpPolicy::parse("cross-origin"), Some(CorpPolicy::CrossOrigin));
        assert_eq!(CorpPolicy::parse("invalid-policy"), None);
    }

    #[test]
    fn test_coop_coep_parsing() {
        assert_eq!(CoopPolicy::parse("same-origin"), CoopPolicy::SameOrigin);
        assert_eq!(CoopPolicy::parse("same-origin-allow-popups"), CoopPolicy::SameOriginAllowPopups);
        assert_eq!(CoopPolicy::parse("unknown"), CoopPolicy::UnsafeNone);

        assert_eq!(CoepPolicy::parse("require-corp"), CoepPolicy::RequireCorp);
        assert_eq!(CoepPolicy::parse("credentialless"), CoepPolicy::Credentialless);
        assert_eq!(CoepPolicy::parse("none"), CoepPolicy::UnsafeNone);
    }

    #[test]
    fn test_cross_origin_isolation_logic() {
        let isolated = IsolationContext::new(CoopPolicy::SameOrigin, CoepPolicy::RequireCorp);
        assert!(isolated.is_cross_origin_isolated());

        let credentialless = IsolationContext::new(CoopPolicy::SameOrigin, CoepPolicy::Credentialless);
        assert!(credentialless.is_cross_origin_isolated());

        let non_isolated = IsolationContext::new(CoopPolicy::UnsafeNone, CoepPolicy::RequireCorp);
        assert!(!non_isolated.is_cross_origin_isolated());
    }

    #[test]
    fn test_validate_corp_allow_and_block() {
        let origin_a = Origin::parse("https://app.albedo.dev").unwrap();
        let origin_sub_a = Origin::parse("https://api.albedo.dev").unwrap();
        let origin_b = Origin::parse("https://malicious.com").unwrap();

        let resource_url = Url::parse("https://app.albedo.dev/profile.json").unwrap();

        // 1. Same-Origin permitido para origin_a
        assert!(validate_corp(Some(&origin_a), &resource_url, Some(CorpPolicy::SameOrigin)).is_ok());

        // 2. Same-Origin bloqueado para subdomínio (origin_sub_a)
        assert!(validate_corp(Some(&origin_sub_a), &resource_url, Some(CorpPolicy::SameOrigin)).is_err());

        // 3. Same-Site permitido para subdomínio (origin_sub_a)
        assert!(validate_corp(Some(&origin_sub_a), &resource_url, Some(CorpPolicy::SameSite)).is_ok());

        // 4. Same-Site bloqueado para domínio diferente (origin_b)
        assert!(validate_corp(Some(&origin_b), &resource_url, Some(CorpPolicy::SameSite)).is_err());

        // 5. Cross-Origin permitido para todos
        assert!(validate_corp(Some(&origin_b), &resource_url, Some(CorpPolicy::CrossOrigin)).is_ok());
    }
}
