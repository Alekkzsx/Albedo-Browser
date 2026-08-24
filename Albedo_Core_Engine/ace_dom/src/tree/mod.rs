//! # Estrutura Principal do Documento e Árvore DOM
//!
//! Gerenciamento centralizado da `Arena<NodeData>`, ciclo de vida e mutações.

pub mod mutation;

use crate::error::DomError;
use crate::node::iter::{AncestorsIter, ChildrenIter, DescendantsIter};
use crate::node::{
    CommentData, DoctypeData, DocumentData, DocumentMode, ElementData, Namespace, NodeData,
    NodeKind, TextData,
};
use ace_core::arena::{Arena, ArenaId};
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use smol_str::SmolStr;

/// A raiz de uma árvore DOM completa gerida em memória contígua na `Arena<NodeData>`.
#[derive(Debug, Clone)]
pub struct Document {
    arena: Arena<NodeData>,
    root: NodeId,
    pub doctype: Option<NodeId>,
    pub document_element: Option<NodeId>,
    pub head: Option<NodeId>,
    pub body: Option<NodeId>,
    pub mode: DocumentMode,
}

impl Default for Document {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Document {
    /// Inicializa um novo `Document` com uma URL opcional.
    pub fn new(url: Option<&str>) -> Self {
        let mut arena = Arena::with_capacity(256);
        let root_node = NodeData {
            id: NodeId::new(),
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            flags: ace_core::flags::NodeFlags::empty(),
            kind: NodeKind::Document(DocumentData {
                mode: DocumentMode::NoQuirks,
                title: None,
                url: url.map(String::from),
            }),
        };

        let arena_id = arena.alloc(root_node);
        let root_id = arena_id.to_node_id();
        // Sincroniza o ID dentro da struct
        if let Some(node) = arena.get_mut(arena_id) {
            node.id = root_id;
        }

        Self {
            arena,
            root: root_id,
            doctype: None,
            document_element: None,
            head: None,
            body: None,
            mode: DocumentMode::NoQuirks,
        }
    }

    /// Retorna o `NodeId` do nó raiz do documento.
    #[inline]
    pub const fn root(&self) -> NodeId {
        self.root
    }

    /// Cria um novo nó Elemento na arena do documento.
    pub fn create_element(&mut self, tag_name: impl Into<Atom>, namespace: Namespace) -> NodeId {
        let dummy_id = NodeId::new();
        let el_data = ElementData::new(tag_name, namespace);
        let node = NodeData::new(dummy_id, NodeKind::Element(el_data));
        let arena_id = self.arena.alloc(node);
        let real_id = arena_id.to_node_id();
        if let Some(n) = self.arena.get_mut(arena_id) {
            n.id = real_id;
        }
        real_id
    }

    /// Cria um novo nó de Texto na arena do documento.
    pub fn create_text_node(&mut self, data: impl Into<SmolStr>) -> NodeId {
        let dummy_id = NodeId::new();
        let node = NodeData::new(dummy_id, NodeKind::Text(TextData::new(data)));
        let arena_id = self.arena.alloc(node);
        let real_id = arena_id.to_node_id();
        if let Some(n) = self.arena.get_mut(arena_id) {
            n.id = real_id;
        }
        real_id
    }

    /// Cria um novo nó de Comentário na arena do documento.
    pub fn create_comment(&mut self, data: impl Into<SmolStr>) -> NodeId {
        let dummy_id = NodeId::new();
        let node = NodeData::new(dummy_id, NodeKind::Comment(CommentData::new(data)));
        let arena_id = self.arena.alloc(node);
        let real_id = arena_id.to_node_id();
        if let Some(n) = self.arena.get_mut(arena_id) {
            n.id = real_id;
        }
        real_id
    }

    /// Cria um nó Doctype na arena do documento.
    pub fn create_doctype(
        &mut self,
        name: impl Into<SmolStr>,
        public_id: Option<SmolStr>,
        system_id: Option<SmolStr>,
        force_quirks: bool,
    ) -> NodeId {
        let dummy_id = NodeId::new();
        let doctype_data = DoctypeData {
            name: name.into(),
            public_id,
            system_id,
            force_quirks,
        };
        let node = NodeData::new(dummy_id, NodeKind::DocumentType(doctype_data));
        let arena_id = self.arena.alloc(node);
        let real_id = arena_id.to_node_id();
        if let Some(n) = self.arena.get_mut(arena_id) {
            n.id = real_id;
        }
        real_id
    }

    /// Obtém uma referência imutável a um nó da árvore em $O(1)$.
    #[inline]
    pub fn get_node(&self, id: NodeId) -> Option<&NodeData> {
        let arena_id = ArenaId::<NodeData>::from_node_id(id)?;
        self.arena.get(arena_id)
    }

    /// Obtém uma referência mutável a um nó da árvore em $O(1)$.
    #[inline]
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut NodeData> {
        let arena_id = ArenaId::<NodeData>::from_node_id(id)?;
        self.arena.get_mut(arena_id)
    }

    /// Anexa um filho ao final da lista de filhos de um pai.
    #[inline]
    pub fn append_child(&mut self, parent_id: NodeId, child_id: NodeId) -> Result<(), DomError> {
        mutation::append_child(&mut self.arena, parent_id, child_id)
    }

    /// Insere um novo filho imediatamente antes de um nó de referência.
    #[inline]
    pub fn insert_before(
        &mut self,
        parent_id: NodeId,
        new_child_id: NodeId,
        ref_child_id: Option<NodeId>,
    ) -> Result<(), DomError> {
        mutation::insert_before(&mut self.arena, parent_id, new_child_id, ref_child_id)
    }

    /// Remove um filho do nó pai.
    #[inline]
    pub fn remove_child(&mut self, parent_id: NodeId, child_id: NodeId) -> Result<(), DomError> {
        mutation::remove_child(&mut self.arena, parent_id, child_id)
    }

    /// Retorna um iterador sobre os filhos imediatos de um nó.
    pub fn children(&self, parent_id: NodeId) -> ChildrenIter<'_> {
        let first_child = self.get_node(parent_id).and_then(|n| n.first_child);
        ChildrenIter::new(&self.arena, first_child)
    }

    /// Retorna um iterador sobre os ancestrais de um nó.
    pub fn ancestors(&self, node_id: NodeId) -> AncestorsIter<'_> {
        let parent = self.get_node(node_id).and_then(|n| n.parent);
        AncestorsIter::new(&self.arena, parent)
    }

    /// Retorna um iterador de travessia pré-ordem em profundidade (DFS) a partir do nó raiz informado.
    pub fn descendants(&self, root_id: NodeId) -> DescendantsIter<'_> {
        DescendantsIter::new(&self.arena, root_id)
    }

    /// Retorna o número total de nós vivos alocados na arena.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.arena.len()
    }
}
