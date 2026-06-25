//! ES Module Loader & Resolver para o Albedo Browser
//!
//! Implementa as traits `rquickjs::loader::Resolver` e `rquickjs::loader::Loader`
//! para suportar `<script type="module">`, `import`/`export` e `import()` dinâmico.
//!
//! ## Arquitetura
//! - `AlbedoModuleResolver`: Resolve specifiers de módulos contra a URL base da página.
//!   Suporta URLs absolutas, relativas (`./`, `../`), bare specifiers (`lodash`), e
//!   data URIs.
//! - `AlbedoModuleLoader`: Busca o código-fonte dos módulos. Primeiro verifica o
//!   cache interno (`module_cache`), depois faz download HTTP síncrono.
//! - `ModuleRegistry`: Cache compartilhado que armazena (url → código_fonte) para
//!   evitar re-downloads e re-avaliações.

use super::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Cache compartilhado de módulos já baixados/registrados.
/// Chave: URL absolutizada do módulo.
/// Valor: Código-fonte JS do módulo.


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
pub(crate) fn test_resolve_absolute_url() {
        let result =
            resolve_module_specifier("https://cdn.example.com/mod.js", "", "https://example.com");
        assert_eq!(result, Some("https://cdn.example.com/mod.js".to_string()));
    }

    #[test]
pub(crate) fn test_resolve_relative_specifier() {
        let result = resolve_module_specifier(
            "./utils.js",
            "https://example.com/app/main.js",
            "https://example.com",
        );
        assert_eq!(result, Some("https://example.com/app/utils.js".to_string()));
    }

    #[test]
pub(crate) fn test_resolve_parent_relative() {
        let result = resolve_module_specifier(
            "../lib/helper.js",
            "https://example.com/app/main.js",
            "https://example.com",
        );
        assert_eq!(
            result,
            Some("https://example.com/lib/helper.js".to_string())
        );
    }

    #[test]
pub(crate) fn test_resolve_absolute_path() {
        let result =
            resolve_module_specifier("/lib/mod.js", "", "https://example.com/app/index.html");
        assert_eq!(result, Some("https://example.com/lib/mod.js".to_string()));
    }

    #[test]
pub(crate) fn test_resolve_bare_specifier() {
        let result = resolve_module_specifier("react", "", "https://example.com");
        assert_eq!(result, Some("https://esm.sh/react".to_string()));
    }

    #[test]
pub(crate) fn test_resolve_bare_specifier_with_path() {
        let result = resolve_module_specifier("lodash/fp", "", "https://example.com");
        assert_eq!(result, Some("https://esm.sh/lodash/fp".to_string()));
    }

    #[test]
pub(crate) fn test_data_uri() {
        let result = resolve_module_specifier(
            "data:text/javascript,export default 42",
            "",
            "https://example.com",
        );
        assert_eq!(
            result,
            Some("data:text/javascript,export default 42".to_string())
        );
    }

    #[test]
pub(crate) fn test_import_map_parse() {
        let json = r#"{
            "imports": {
                "react": "https://esm.sh/react@18",
                "lodash/": "https://esm.sh/lodash/"
            },
            "scopes": {
                "/app/": {
                    "react": "https://esm.sh/react@17"
                }
            }
        }"#;

        let map = ImportMap::parse(json).expect("Invalid ImportMap JSON");
        assert_eq!(
            map.imports.get("react"),
            Some(&"https://esm.sh/react@18".to_string())
        );
        assert_eq!(
            map.resolve("react", "https://example.com/index.js"),
            Some("https://esm.sh/react@18".to_string())
        );
        // Specifier com escopo deve usar a versão do escopo
        assert_eq!(
            map.resolve("react", "/app/main.js"),
            Some("https://esm.sh/react@17".to_string())
        );
        // Prefix match
        assert_eq!(
            map.resolve("lodash/fp", "https://example.com/index.js"),
            Some("https://esm.sh/lodash/fp".to_string())
        );
    }
}
