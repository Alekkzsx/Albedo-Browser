//! # Filtros de Nós DOM (NodeFilter — WHATWG DOM §6)

bitflags::bitflags! {
    /// Máscara de bits para filtragem de tipos de nós durante a travessia.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NodeFilter: u32 {
        const SHOW_ALL = 0xFFFFFFFF;
        const SHOW_ELEMENT = 0x00000001;
        const SHOW_TEXT = 0x00000004;
        const SHOW_COMMENT = 0x00000080;
        const SHOW_DOCUMENT = 0x00000100;
        const SHOW_DOCUMENT_TYPE = 0x00000200;
        const SHOW_DOCUMENT_FRAGMENT = 0x00000400;
        const SHOW_SHADOW_ROOT = 0x00000800;
    }
}

/// Resultado de decisão de um filtro sobre um determinado nó.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterResult {
    /// Aceita o nó e o inclui na iteração.
    Accept,
    /// Rejeita o nó e todos os seus filhos (para TreeWalker).
    Reject,
    /// Pula este nó, mas continua avaliando seus filhos.
    Skip,
}
