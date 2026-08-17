//! # Utilitários para Configuração de Features
//!
//! Funções para parsing de flags de linha de comando (`--enable-features`) e overrides de configuração.

use super::Feature;

/// Analisa uma lista de features separadas por vírgula (ex: `"CssSubgrid,-WebAssembly"`).
///
/// Retorna uma lista de tuplas `(Feature, bool)` indicando se cada feature deve ser ativada (`true`) ou desativada (`false`).
pub fn parse_feature_overrides(input: &str) -> Vec<(Feature, bool)> {
    let mut overrides = Vec::new();

    for item in input.split(',') {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (name, enabled) = if let Some(stripped) = trimmed.strip_prefix('-') {
            (stripped, false)
        } else if let Some(stripped) = trimmed.strip_prefix('+') {
            (stripped, true)
        } else {
            (trimmed, true)
        };

        if let Some(feature) = feature_from_name(name) {
            overrides.push((feature, enabled));
        }
    }

    overrides
}

fn feature_from_name(name: &str) -> Option<Feature> {
    match name.to_ascii_lowercase().as_str() {
        "cssflexbox" | "flexbox" => Some(Feature::CssFlexbox),
        "cssgrid" | "grid" => Some(Feature::CssGrid),
        "csssubgrid" | "subgrid" => Some(Feature::CssSubgrid),
        "webassembly" | "wasm" => Some(Feature::WebAssembly),
        "webgl" => Some(Feature::WebGl),
        "offscreencanvas" => Some(Feature::OffscreenCanvas),
        "fetchapi" | "fetch" => Some(Feature::FetchApi),
        "localstorage" => Some(Feature::LocalStorage),
        "indexeddb" => Some(Feature::IndexedDb),
        "devtools" => Some(Feature::DevTools),
        "websocket" => Some(Feature::WebSocket),
        "serviceworker" => Some(Feature::ServiceWorker),
        _ => None,
    }
}
