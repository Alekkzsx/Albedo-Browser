use super::*;
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

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Cache compartilhado de módulos já baixados/registrados.
/// Chave: URL absolutizada do módulo.
/// Valor: Código-fonte JS do módulo.


/// Representa um Import Map (`<script type="importmap">`)
#[derive(Debug, Clone, Default)]
pub struct ImportMap {
    /// Mapeamentos diretos (ex: "react" → "https://esm.sh/react@18")
    pub imports: HashMap<String, String>,
    /// Mapeamentos com escopo (ex: "/app/" → { "lodash" → "..." })
    pub scopes: HashMap<String, HashMap<String, String>>,
}

impl ImportMap {
    /// Faz parse do JSON de um import map
    pub fn parse(json_str: &str) -> Option<Self> {
        let parsed: crate::ace::json::JsonValue = crate::ace::json::parse(json_str).ok()?;
        let mut import_map = ImportMap::default();

        if let Some(imports) = parsed.get("imports").and_then(|v| v.as_object()) {
            for (key, value) in imports {
                if let Some(url_str) = value.as_string() {
                    import_map.imports.insert(key.clone(), url_str.to_string());
                }
            }
        }

        if let Some(scopes) = parsed.get("scopes").and_then(|v| v.as_object()) {
            for (scope_key, scope_value) in scopes {
                if let Some(scope_map) = scope_value.as_object() {
                    let mut map = HashMap::new();
                    for (key, value) in scope_map {
                        if let Some(url_str) = value.as_string() {
                            map.insert(key.clone(), url_str.to_string());
                        }
                    }
                    import_map.scopes.insert(scope_key.clone(), map);
                }
            }
        }

        Some(import_map)
    }

    /// Resolve um specifier usando o import map.
    /// Retorna Some(url) se encontrar, None caso contrário.
    pub fn resolve(&self, specifier: &str, referrer: &str) -> Option<String> {
        // 1. Escopo mais específico primeiro
        let mut best_scope_match: Option<(&str, &HashMap<String, String>)> = None;
        for (scope_prefix, scope_map) in &self.scopes {
            if referrer.starts_with(scope_prefix.as_str()) {
                if best_scope_match.is_none()
                    || scope_prefix.len() > best_scope_match.as_ref().expect("checked is_some").0.len()
                {
                    best_scope_match = Some((scope_prefix, scope_map));
                }
            }
        }

        // Verificar match no escopo
        if let Some((_, scope_map)) = best_scope_match {
            if let Some(resolved) = self.match_specifier(specifier, scope_map) {
                return Some(resolved);
            }
        }

        // 2. Imports globais
        self.match_specifier(specifier, &self.imports)
    }

pub(crate) fn match_specifier(&self, specifier: &str, map: &HashMap<String, String>) -> Option<String> {
        // Match exato
        if let Some(url) = map.get(specifier) {
            return Some(url.clone());
        }

        // Match por prefixo (ex: "lodash/" mapeia "lodash/fp" → "...")
        let mut best_match: Option<(&str, &str)> = None;
        for (pattern, target) in map {
            if pattern.ends_with('/') && specifier.starts_with(pattern.as_str()) {
                if best_match.is_none() || pattern.len() > best_match.as_ref().expect("checked is_some").0.len() {
                    best_match = Some((pattern, target));
                }
            }
        }

        if let Some((pattern, target)) = best_match {
            let suffix = &specifier[pattern.len()..];
            return Some(format!("{}{}", target, suffix));
        }

        None
    }
}
