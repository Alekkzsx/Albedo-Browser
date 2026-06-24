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

pub mod moduleregistry; pub use moduleregistry::*;
pub mod resolve_module_specifier; pub use resolve_module_specifier::*;
pub mod fetch_module_source; pub use fetch_module_source::*;
pub mod albedomoduleresolver; pub use albedomoduleresolver::*;
pub mod albedomoduleloader; pub use albedomoduleloader::*;
pub mod scriptdescriptor; pub use scriptdescriptor::*;
pub mod importmap; pub use importmap::*;
pub mod fetch_module_source_public; pub use fetch_module_source_public::*;
pub mod test_resolve_absolute_url; pub use test_resolve_absolute_url::*;
