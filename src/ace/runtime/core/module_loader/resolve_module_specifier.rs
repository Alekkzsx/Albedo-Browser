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


/// Resolve um specifier de módulo ES para uma URL absoluta.
///
/// Suporta:
/// - URLs absolutas (`https://cdn.example.com/mod.js`)
/// - Caminhos absolutos (`/lib/mod.js`)
/// - Caminhos relativos (`./mod.js`, `../utils.js`)
/// - Data URIs (`data:text/javascript,...`)
/// - Bare specifiers com fallback para esm.sh CDN (`react` → `https://esm.sh/react`)
pub fn resolve_module_specifier(specifier: &str, referrer: &str, base_url: &str) -> Option<String> {
    // 1. URLs absolutas — retorna direto
    if specifier.starts_with("http://") || specifier.starts_with("https://") {
        return Some(specifier.to_string());
    }

    // 2. Data URIs — retorna direto
    if specifier.starts_with("data:") {
        return Some(specifier.to_string());
    }

    // 3. Blob URIs — retorna direto
    if specifier.starts_with("blob:") {
        return Some(specifier.to_string());
    }

    // 4. Caminhos relativos (./foo, ../bar) — resolve contra o referrer
    if specifier.starts_with("./") || specifier.starts_with("../") {
        let effective_base = if !referrer.is_empty() && referrer != "<input>" {
            referrer
        } else {
            base_url
        };

        if let Ok(base) = crate::ace::url::parse(effective_base, None) {
            if let Ok(resolved) = base.join(specifier) {
                return Some(resolved.to_string());
            }
        }
        return None;
    }

    // 5. Caminhos absolutos (/lib/foo.js) — resolve contra a origem da página
    if specifier.starts_with('/') {
        if let Ok(base) = crate::ace::url::parse(base_url, None) {
            if let Ok(resolved) = base.join(specifier) {
                return Some(resolved.to_string());
            }
        }
        return None;
    }

    // 6. Bare specifiers (ex: "react", "lodash/fp") — mapear para CDN esm.sh
    //    Isso é essencial para compatibilidade com frameworks modernos que usam
    //    import maps ou bare specifiers.
    //    TODO: Suportar import maps (<script type="importmap">) no futuro.
    Some(format!("https://esm.sh/{}", specifier))
}
