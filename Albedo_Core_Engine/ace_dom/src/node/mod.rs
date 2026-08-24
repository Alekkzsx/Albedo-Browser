//! # Modelo de Nós da Árvore DOM (Node Model)
//!
//! Estrutura compacta com nós alocados na `Arena<NodeData>` e links bidirecionais
//! ocupando exatamente 40 bytes por nó através de `Option<NodeId>` (Niche Optimization).

pub mod element;
pub mod iter;
pub mod text;
pub mod token_list;

pub use element::{Attribute, ElementData, Namespace};
pub use text::{CommentData, DoctypeData, DocumentData, DocumentMode, TextData};
pub use token_list::DOMTokenList;

use ace_core::flags::NodeFlags;
use ace_core::id::NodeId;
use ace_core::intern::Atom;

/// Modo de encapsulamento da Shadow DOM (WHATWG DOM Standard).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadowMode {
    #[default]
    Open,
    Closed,
}

/// Dados específicos de uma raiz de sombra (`NodeKind::ShadowRoot`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowRootData {
    pub mode: ShadowMode,
    pub host: NodeId,
}

/// As diferentes variantes de nós suportadas na árvore DOM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Document(DocumentData),
    DocumentType(DoctypeData),
    Element(Box<ElementData>),
    Text(TextData),
    Comment(CommentData),
    DocumentFragment,
    ShadowRoot(ShadowRootData),
}

/// A estrutura primária de um nó na árvore DOM do Albedo.
///
/// Possui links bidirecionais (parent, first_child, last_child, prev_sibling, next_sibling)
/// que totalizam exatamente 40 bytes graças à elisão de discriminante (`Option<NonZeroU64>`).
#[derive(Debug, Clone)]
pub struct NodeData {
    /// Identificador único e estável do nó.
    pub id: NodeId,
    /// Pai imediato na hierarquia DOM.
    pub parent: Option<NodeId>,
    /// Primeiro filho.
    pub first_child: Option<NodeId>,
    /// Último filho.
    pub last_child: Option<NodeId>,
    /// Irmão imediatamente anterior.
    pub prev_sibling: Option<NodeId>,
    /// Irmão imediatamente posterior.
    pub next_sibling: Option<NodeId>,
    /// Flags de estado e dirty-tracking (recalc style, reflow layout, repaint).
    pub flags: NodeFlags,
    /// Dados específicos do tipo do nó.
    pub kind: NodeKind,
}

impl NodeData {
    /// Cria uma nova instância de `NodeData` isolada (sem conexões na árvore).
    pub fn new(id: NodeId, kind: NodeKind) -> Self {
        Self {
            id,
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            flags: NodeFlags::empty(),
            kind,
        }
    }

    /// Retorna `true` se este nó for um elemento (`NodeKind::Element`).
    #[inline]
    pub fn is_element(&self) -> bool {
        matches!(self.kind, NodeKind::Element(_))
    }

    /// Retorna `true` se este nó for um nó de texto (`NodeKind::Text`).
    #[inline]
    pub fn is_text(&self) -> bool {
        matches!(self.kind, NodeKind::Text(_))
    }

    /// Retorna `true` se este nó for o documento raiz (`NodeKind::Document`).
    #[inline]
    pub fn is_document(&self) -> bool {
        matches!(self.kind, NodeKind::Document(_))
    }

    /// Retorna uma referência a `ElementData` caso o nó seja um Elemento.
    #[inline]
    pub fn as_element(&self) -> Option<&ElementData> {
        match &self.kind {
            NodeKind::Element(data) => Some(data),
            _ => None,
        }
    }

    /// Retorna uma referência mutável a `ElementData` caso o nó seja um Elemento.
    #[inline]
    pub fn as_element_mut(&mut self) -> Option<&mut ElementData> {
        match &mut self.kind {
            NodeKind::Element(data) => Some(data),
            _ => None,
        }
    }

    /// Retorna o nome da tag do elemento se for um Elemento.
    #[inline]
    pub fn tag_name(&self) -> Option<&Atom> {
        self.as_element().map(|el| &el.tag_name)
    }

    /// Retorna o conteúdo textual do nó se for Texto ou Comentário.
    #[inline]
    pub fn text_content(&self) -> Option<&str> {
        match &self.kind {
            NodeKind::Text(t) => Some(t.data.as_str()),
            NodeKind::Comment(c) => Some(c.data.as_str()),
            _ => None,
        }
    }
}
