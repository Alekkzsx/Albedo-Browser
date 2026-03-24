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
#[derive(Clone, Debug, Default)]
pub struct ModuleRegistry {
    /// Módulos cujo fonte já foi resolvido (inline ou fetched).
    pub sources: Arc<Mutex<HashMap<String, String>>>,
    /// URLs de módulos já avaliados (evitar re-avaliação).
    pub evaluated: Arc<Mutex<std::collections::HashSet<String>>>,
    /// URL base da página (para resolver caminhos relativos).
    pub base_url: Arc<Mutex<String>>,
}

impl ModuleRegistry {
    pub fn new(base_url: &str) -> Self {
        Self {
            sources: Arc::new(Mutex::new(HashMap::new())),
            evaluated: Arc::new(Mutex::new(std::collections::HashSet::new())),
            base_url: Arc::new(Mutex::new(base_url.to_string())),
        }
    }

    /// Registra um módulo inline no cache (ex: conteúdo de <script type="module">)
    pub fn register_inline(&self, name: &str, source: String) {
        let mut sources = self.sources.lock().unwrap();
        sources.insert(name.to_string(), source);
    }

    /// Registra o código de um módulo externo já baixado
    pub fn register_external(&self, url: &str, source: String) {
        let mut sources = self.sources.lock().unwrap();
        sources.insert(url.to_string(), source);
    }

    /// Verifica se o módulo já foi avaliado
    pub fn is_evaluated(&self, url: &str) -> bool {
        let evaluated = self.evaluated.lock().unwrap();
        evaluated.contains(url)
    }

    /// Marca um módulo como avaliado
    pub fn mark_evaluated(&self, url: &str) {
        let mut evaluated = self.evaluated.lock().unwrap();
        evaluated.insert(url.to_string());
    }

    /// Obtém o código-fonte de um módulo do cache
    pub fn get_source(&self, url: &str) -> Option<String> {
        let sources = self.sources.lock().unwrap();
        sources.get(url).cloned()
    }

    /// Resolve um specifier de módulo para uma URL absoluta
    pub fn resolve_specifier(&self, specifier: &str, referrer: &str) -> Option<String> {
        resolve_module_specifier(specifier, referrer, &self.base_url.lock().unwrap())
    }
}

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

/// Faz download síncrono de um módulo JS via HTTP(S).
/// Retorna `None` se o download falhar.
fn fetch_module_source(url: &str) -> Option<String> {
    println!("[ESModules] Fetching module: {}", url);

    // Data URI handling
    if url.starts_with("data:") {
        if let Some(comma_pos) = url.find(',') {
            let data = &url[comma_pos + 1..];
            // Check if base64 encoded
            let prefix = &url[..comma_pos];
            if prefix.contains(";base64") {
                if let Ok(decoded) =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
                {
                    return String::from_utf8(decoded).ok();
                }
            } else {
                return Some(crate::ace::url::percent_encoding::decode(data));
            }
        }
        return None;
    }

    // HTTP fetch blocking
    match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Albedo/1.0")
        .build()
    {
        Ok(client) => match client.get(url).send() {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text() {
                        Ok(text) => {
                            println!("[ESModules] Fetched {} bytes from {}", text.len(), url);
                            Some(text)
                        }
                        Err(e) => {
                            eprintln!(
                                "[ESModules] Failed to read response body from {}: {}",
                                url, e
                            );
                            None
                        }
                    }
                } else {
                    eprintln!("[ESModules] HTTP {} for module {}", response.status(), url);
                    None
                }
            }
            Err(e) => {
                eprintln!("[ESModules] Network error fetching {}: {}", url, e);
                None
            }
        },
        Err(e) => {
            eprintln!("[ESModules] Failed to create HTTP client: {}", e);
            None
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// rquickjs Resolver & Loader Implementation
// ═══════════════════════════════════════════════════════════════════════

/// Resolver de módulos ES para o QuickJS.
/// Converte specifiers de import em nomes canônicos (URLs absolutas).
pub struct AlbedoModuleResolver {
    pub registry: ModuleRegistry,
}

impl rquickjs::loader::Resolver for AlbedoModuleResolver {
    fn resolve<'js>(
        &mut self,
        _ctx: &rquickjs::Ctx<'js>,
        base: &str,
        name: &str,
    ) -> rquickjs::Result<String> {
        println!(
            "[ESModules::Resolver] Resolving '{}' from base '{}'",
            name, base
        );

        let base_url = self.registry.base_url.lock().unwrap().clone();

        match resolve_module_specifier(name, base, &base_url) {
            Some(resolved) => {
                println!("[ESModules::Resolver] Resolved to: {}", resolved);
                Ok(resolved)
            }
            None => {
                eprintln!(
                    "[ESModules::Resolver] Failed to resolve '{}' from '{}'",
                    name, base
                );
                Err(rquickjs::Error::new_resolving(base, name))
            }
        }
    }
}

/// Loader de módulos ES para o QuickJS.
/// Busca o código-fonte dos módulos (do cache ou via HTTP).
pub struct AlbedoModuleLoader {
    pub registry: ModuleRegistry,
}

impl rquickjs::loader::Loader for AlbedoModuleLoader {
    fn load<'js>(
        &mut self,
        ctx: &rquickjs::Ctx<'js>,
        name: &str,
    ) -> rquickjs::Result<rquickjs::Module<'js, rquickjs::module::Declared>> {
        println!("[ESModules::Loader] Loading module: {}", name);

        // 1. Verificar se já existe no cache do registry
        if let Some(source) = self.registry.get_source(name) {
            println!(
                "[ESModules::Loader] Found in cache: {} ({} bytes)",
                name,
                source.len()
            );
            return rquickjs::Module::declare(ctx.clone(), name, source);
        }

        // 2. Tentar fazer fetch externo
        if name.starts_with("http://") || name.starts_with("https://") || name.starts_with("data:")
        {
            if let Some(source) = fetch_module_source(name) {
                // Registrar no cache para evitar re-download
                self.registry.register_external(name, source.clone());
                return rquickjs::Module::declare(ctx.clone(), name, source);
            }
        }

        eprintln!("[ESModules::Loader] Failed to load module: {}", name);
        Err(rquickjs::Error::new_loading(name))
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Script Descriptor (representação de <script> tags do DOM)
// ═══════════════════════════════════════════════════════════════════════

/// Descreve um `<script>` encontrado no DOM durante o parsing HTML.
#[derive(Debug, Clone)]
pub struct ScriptDescriptor {
    /// Indica se é `<script type="module">` vs script clássico
    pub is_module: bool,
    /// URL do script externo (`src` attribute), se houver
    pub src: Option<String>,
    /// Conteúdo inline do script (texto entre as tags)
    pub inline_content: Option<String>,
    /// Script assíncrono (`async` attribute)
    pub is_async: bool,
    /// Script deferido (`defer` attribute)
    pub is_defer: bool,
    /// `nomodule` — ignorar em navegadores que suportam módulos
    pub is_nomodule: bool,
    /// Posição (order) do script no DOM
    pub dom_order: usize,
    /// Tipo MIME bruto (ex: "module", "text/javascript", "importmap")
    pub type_attr: Option<String>,
    /// `crossorigin` attribute
    pub crossorigin: Option<String>,
    /// `integrity` attribute (SRI hash)
    pub integrity: Option<String>,
    /// Índice do nó no DOM (para referência)
    pub node_index: usize,
}

impl ScriptDescriptor {
    /// Verifica se é um import map
    pub fn is_import_map(&self) -> bool {
        self.type_attr.as_deref() == Some("importmap")
    }
}

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
                    || scope_prefix.len() > best_scope_match.unwrap().0.len()
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

    fn match_specifier(&self, specifier: &str, map: &HashMap<String, String>) -> Option<String> {
        // Match exato
        if let Some(url) = map.get(specifier) {
            return Some(url.clone());
        }

        // Match por prefixo (ex: "lodash/" mapeia "lodash/fp" → "...")
        let mut best_match: Option<(&str, &str)> = None;
        for (pattern, target) in map {
            if pattern.ends_with('/') && specifier.starts_with(pattern.as_str()) {
                if best_match.is_none() || pattern.len() > best_match.unwrap().0.len() {
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

/// Public API para download de scripts/módulos externos.
/// Usado pelo `engine/mod.rs` para baixar scripts clássicos com `src` attribute.
pub fn fetch_module_source_public(url: &str) -> Option<String> {
    fetch_module_source(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_absolute_url() {
        let result =
            resolve_module_specifier("https://cdn.example.com/mod.js", "", "https://example.com");
        assert_eq!(result, Some("https://cdn.example.com/mod.js".to_string()));
    }

    #[test]
    fn test_resolve_relative_specifier() {
        let result = resolve_module_specifier(
            "./utils.js",
            "https://example.com/app/main.js",
            "https://example.com",
        );
        assert_eq!(result, Some("https://example.com/app/utils.js".to_string()));
    }

    #[test]
    fn test_resolve_parent_relative() {
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
    fn test_resolve_absolute_path() {
        let result =
            resolve_module_specifier("/lib/mod.js", "", "https://example.com/app/index.html");
        assert_eq!(result, Some("https://example.com/lib/mod.js".to_string()));
    }

    #[test]
    fn test_resolve_bare_specifier() {
        let result = resolve_module_specifier("react", "", "https://example.com");
        assert_eq!(result, Some("https://esm.sh/react".to_string()));
    }

    #[test]
    fn test_resolve_bare_specifier_with_path() {
        let result = resolve_module_specifier("lodash/fp", "", "https://example.com");
        assert_eq!(result, Some("https://esm.sh/lodash/fp".to_string()));
    }

    #[test]
    fn test_data_uri() {
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
    fn test_import_map_parse() {
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

        let map = ImportMap::parse(json).unwrap();
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
