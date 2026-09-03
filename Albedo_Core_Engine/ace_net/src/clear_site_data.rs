//! # W3C Clear-Site-Data (RFC 8879)
//!
//! Permite que servidores em conexões seguras instruam o navegador a purgar
//! deterministicamente dados locais da origem (cache de rede, cookies, storage ou todos).

use http::HeaderValue;

/// Ações de limpeza solicitadas pelo cabeçalho `Clear-Site-Data`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClearSiteDataAction {
    pub clear_cache: bool,
    pub clear_cookies: bool,
    pub clear_storage: bool,
    pub clear_execution_contexts: bool,
}

impl ClearSiteDataAction {
    /// Analisa o valor bruto do cabeçalho `Clear-Site-Data`.
    /// Exemplo: `Clear-Site-Data: "cache", "cookies"` ou `Clear-Site-Data: "*"`
    pub fn parse(header_val: &HeaderValue) -> Self {
        let s = match header_val.to_str() {
            Ok(v) => v,
            Err(_) => return Self::default(),
        };

        let mut action = Self::default();

        for part in s.split(',') {
            let directive = part.trim().trim_matches('"').trim();
            match directive {
                "cache" => action.clear_cache = true,
                "cookies" => action.clear_cookies = true,
                "storage" => action.clear_storage = true,
                "executionContexts" => action.clear_execution_contexts = true,
                "*" => {
                    action.clear_cache = true;
                    action.clear_cookies = true;
                    action.clear_storage = true;
                    action.clear_execution_contexts = true;
                }
                _ => {}
            }
        }

        action
    }

    /// Verifica se qualquer ação de purga foi acionada.
    pub fn is_empty(&self) -> bool {
        !self.clear_cache
            && !self.clear_cookies
            && !self.clear_storage
            && !self.clear_execution_contexts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clear_site_data_directives() {
        let val = HeaderValue::from_static("\"cache\", \"cookies\"");
        let action = ClearSiteDataAction::parse(&val);

        assert!(action.clear_cache);
        assert!(action.clear_cookies);
        assert!(!action.clear_storage);
        assert!(!action.clear_execution_contexts);
    }

    #[test]
    fn test_parse_clear_site_data_wildcard() {
        let val = HeaderValue::from_static("\"*\"");
        let action = ClearSiteDataAction::parse(&val);

        assert!(action.clear_cache);
        assert!(action.clear_cookies);
        assert!(action.clear_storage);
        assert!(action.clear_execution_contexts);
    }
}
