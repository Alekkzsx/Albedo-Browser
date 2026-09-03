//! # W3C Fetch Metadata Request Headers
//!
//! Implementa os cabeçalhos de segurança padronizados pelo W3C que fornecem aos servidores
//! contexto exato sobre a origem e o propósito de cada requisição:
//! - `Sec-Fetch-Site`: Relação entre a origem solicitante e o destino (`same-origin`, `same-site`, `cross-site`, `none`).
//! - `Sec-Fetch-Mode`: Modo de busca (`navigate`, `cors`, `no-cors`, `same-origin`).
//! - `Sec-Fetch-Dest`: Destino do recurso (`document`, `style`, `script`, `font`, `image`, etc.).
//! - `Sec-Fetch-User`: Presente (`?1`) caso a requisição decorra de ativação do usuário (clique, envio de formulário).

use crate::request::{RequestDestination, RequestMode};
use ace_core::security::origin::Origin;
use http::header::HeaderMap;
use http::{HeaderName, HeaderValue};
use url::Url;

/// Relação de origem para o cabeçalho `Sec-Fetch-Site`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecFetchSite {
    /// Mesma origem (esquema, host e porta idênticos).
    SameOrigin,
    /// Mesmo site registrável (mesmo eTLD+1).
    SameSite,
    /// Origens e sites totalmente distintos.
    CrossSite,
    /// Navegação direta disparada pelo usuário (ex: barra de endereços, favoritos).
    None,
}

impl SecFetchSite {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SameOrigin => "same-origin",
            Self::SameSite => "same-site",
            Self::CrossSite => "cross-site",
            Self::None => "none",
        }
    }

    pub fn to_header_value(self) -> HeaderValue {
        HeaderValue::from_static(self.as_str())
    }

    /// Calcula a relação de site entre o contexto iniciador e o recurso alvo.
    pub fn compute(initiator: Option<&Origin>, target_url: &Url) -> Self {
        let initiator_origin = match initiator {
            Some(o) => o,
            None => return Self::None,
        };

        if initiator_origin.is_opaque() {
            return Self::CrossSite;
        }

        let target_origin = match Origin::parse(target_url.as_str()) {
            Ok(o) => o,
            Err(_) => return Self::CrossSite,
        };

        if initiator_origin == &target_origin {
            return Self::SameOrigin;
        }

        // Verifica se compartilham o mesmo eTLD+1 via SchemefulSite do ace_core
        if initiator_origin.is_same_site(&target_origin) {
            return Self::SameSite;
        }

        Self::CrossSite
    }
}

/// Modo de requisição para o cabeçalho `Sec-Fetch-Mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecFetchMode {
    Navigate,
    Cors,
    NoCors,
    SameOrigin,
}

impl SecFetchMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Navigate => "navigate",
            Self::Cors => "cors",
            Self::NoCors => "no-cors",
            Self::SameOrigin => "same-origin",
        }
    }

    pub fn to_header_value(self) -> HeaderValue {
        HeaderValue::from_static(self.as_str())
    }

    pub fn from_request_mode(mode: RequestMode) -> Self {
        match mode {
            RequestMode::Navigate => Self::Navigate,
            RequestMode::Cors => Self::Cors,
            RequestMode::NoCors => Self::NoCors,
            RequestMode::SameOrigin => Self::SameOrigin,
        }
    }
}

/// Destino do recurso para o cabeçalho `Sec-Fetch-Dest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecFetchDest {
    Document,
    Style,
    Script,
    Font,
    Image,
    Audio,
    Video,
    Empty,
}

impl SecFetchDest {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Style => "style",
            Self::Script => "script",
            Self::Font => "font",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Empty => "empty",
        }
    }

    pub fn to_header_value(self) -> HeaderValue {
        HeaderValue::from_static(self.as_str())
    }

    pub fn from_destination(dest: RequestDestination) -> Self {
        match dest {
            RequestDestination::Document => Self::Document,
            RequestDestination::Style => Self::Style,
            RequestDestination::Script => Self::Script,
            RequestDestination::Font => Self::Font,
            RequestDestination::Image => Self::Image,
            RequestDestination::Media => Self::Video,
            RequestDestination::Fetch | RequestDestination::Other => Self::Empty,
        }
    }
}

/// Injeta automaticamente o conjunto completo de cabeçalhos W3C Fetch Metadata no `HeaderMap`.
pub fn inject_fetch_metadata(
    headers: &mut HeaderMap,
    site: SecFetchSite,
    mode: SecFetchMode,
    dest: SecFetchDest,
    is_user_activated: bool,
) {
    let site_header = HeaderName::from_static("sec-fetch-site");
    let mode_header = HeaderName::from_static("sec-fetch-mode");
    let dest_header = HeaderName::from_static("sec-fetch-dest");
    let user_header = HeaderName::from_static("sec-fetch-user");

    headers.insert(site_header, site.to_header_value());
    headers.insert(mode_header, mode.to_header_value());
    headers.insert(dest_header, dest.to_header_value());

    if is_user_activated && mode == SecFetchMode::Navigate {
        headers.insert(user_header, HeaderValue::from_static("?1"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_metadata_site_computation() {
        let origin_a = Origin::parse("https://example.com").unwrap();
        let target_same_origin = Url::parse("https://example.com/api/data").unwrap();
        let target_same_site = Url::parse("https://cdn.example.com/style.css").unwrap();
        let target_cross_site = Url::parse("https://malicious.org/tracker.js").unwrap();

        assert_eq!(SecFetchSite::compute(Some(&origin_a), &target_same_origin), SecFetchSite::SameOrigin);
        assert_eq!(SecFetchSite::compute(Some(&origin_a), &target_same_site), SecFetchSite::SameSite);
        assert_eq!(SecFetchSite::compute(Some(&origin_a), &target_cross_site), SecFetchSite::CrossSite);
        assert_eq!(SecFetchSite::compute(None, &target_same_origin), SecFetchSite::None);
    }

    #[test]
    fn test_fetch_metadata_injection() {
        let mut headers = HeaderMap::new();
        inject_fetch_metadata(
            &mut headers,
            SecFetchSite::SameOrigin,
            SecFetchMode::Navigate,
            SecFetchDest::Document,
            true,
        );

        assert_eq!(headers.get("sec-fetch-site").unwrap(), "same-origin");
        assert_eq!(headers.get("sec-fetch-mode").unwrap(), "navigate");
        assert_eq!(headers.get("sec-fetch-dest").unwrap(), "document");
        assert_eq!(headers.get("sec-fetch-user").unwrap(), "?1");
    }
}
