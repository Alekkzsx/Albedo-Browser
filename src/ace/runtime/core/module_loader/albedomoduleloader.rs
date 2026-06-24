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


/// Loader de módulos ES para o QuickJS.
/// Busca o código-fonte dos módulos (do cache ou via HTTP).
pub struct AlbedoModuleLoader {
    pub registry: ModuleRegistry,
}

impl rquickjs::loader::Loader for AlbedoModuleLoader {
pub(crate) fn load<'js>(
        &mut self,
        ctx: &rquickjs::Ctx<'js>,
        name: &str,
    ) -> rquickjs::Result<rquickjs::Module<'js, rquickjs::module::Declared>> {
        tracing::info!(name = %name, "Loading module");

        // 1. Verificar se já existe no cache do registry
        if let Some(source) = self.registry.get_source(name) {
            tracing::debug!(name = %name, len = source.len(), "Found module in cache");
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

        tracing::error!(name = %name, "Failed to load module");
        Err(rquickjs::Error::new_loading(name))
    }
}
