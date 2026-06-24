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
    /// TODO: add docs
    pub fn new(base_url: &str) -> Self {
        Self {
            sources: Arc::new(Mutex::new(HashMap::new())),
            evaluated: Arc::new(Mutex::new(std::collections::HashSet::new())),
            base_url: Arc::new(Mutex::new(base_url.to_string())),
        }
    }

    /// Registra um módulo inline no cache (ex: conteúdo de <script type="module">)
    pub fn register_inline(&self, name: &str, source: String) {
        let mut sources = self.sources.lock().unwrap_or_else(|e| e.into_inner());
        sources.insert(name.to_string(), source);
    }

    /// Registra o código de um módulo externo já baixado
    pub fn register_external(&self, url: &str, source: String) {
        let mut sources = self.sources.lock().unwrap_or_else(|e| e.into_inner());
        sources.insert(url.to_string(), source);
    }

    /// Verifica se o módulo já foi avaliado
    pub fn is_evaluated(&self, url: &str) -> bool {
        let evaluated = self.evaluated.lock().unwrap_or_else(|e| e.into_inner());
        evaluated.contains(url)
    }

    /// Marca um módulo como avaliado
    pub fn mark_evaluated(&self, url: &str) {
        let mut evaluated = self.evaluated.lock().unwrap_or_else(|e| e.into_inner());
        evaluated.insert(url.to_string());
    }

    /// Obtém o código-fonte de um módulo do cache
    pub fn get_source(&self, url: &str) -> Option<String> {
        let sources = self.sources.lock().unwrap_or_else(|e| e.into_inner());
        sources.get(url).cloned()
    }

    /// Resolve um specifier de módulo para uma URL absoluta
    pub fn resolve_specifier(&self, specifier: &str, referrer: &str) -> Option<String> {
        resolve_module_specifier(specifier, referrer, &self.base_url.lock().unwrap_or_else(|e| e.into_inner()))
    }
}
