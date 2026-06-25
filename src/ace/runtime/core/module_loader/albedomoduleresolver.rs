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


// ═══════════════════════════════════════════════════════════════════════
// rquickjs Resolver & Loader Implementation
// ═══════════════════════════════════════════════════════════════════════

/// Resolver de módulos ES para o QuickJS.
/// Converte specifiers de import em nomes canônicos (URLs absolutas).
pub struct AlbedoModuleResolver {
    pub registry: ModuleRegistry,
}

impl rquickjs::loader::Resolver for AlbedoModuleResolver {
pub(crate) fn resolve<'js>(
        &mut self,
        _ctx: &rquickjs::Ctx<'js>,
        base: &str,
        name: &str,
    ) -> rquickjs::Result<String> {
        tracing::debug!(name = %name, base = %base, "Resolving module");

        let base_url = self.registry.base_url.lock().unwrap_or_else(|e| e.into_inner()).clone();

        match resolve_module_specifier(name, base, &base_url) {
            Some(resolved) => {
                tracing::debug!(resolved = %resolved, "Module resolved");
                Ok(resolved)
            }
            None => {
                tracing::error!(name = %name, base = %base, "Failed to resolve module");
                Err(rquickjs::Error::new_resolving(base, name))
            }
        }
    }
}
