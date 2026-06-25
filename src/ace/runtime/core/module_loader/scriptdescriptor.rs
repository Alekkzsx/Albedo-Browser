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
