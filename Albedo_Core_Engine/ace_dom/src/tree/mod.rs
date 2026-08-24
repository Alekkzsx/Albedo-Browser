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
        let node = NodeData::new(dummy_id, NodeKind::Element(Box::new(el_data)));
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

    /// Cria um novo nó de Fragmento de Documento (`DocumentFragment`).
    pub fn create_document_fragment(&mut self) -> NodeId {
        let dummy_id = NodeId::new();
        let node = NodeData::new(dummy_id, NodeKind::DocumentFragment);
        let arena_id = self.arena.alloc(node);
        let real_id = arena_id.to_node_id();
        if let Some(n) = self.arena.get_mut(arena_id) {
            n.id = real_id;
        }
        real_id
    }

    /// Conecta uma Shadow DOM a um elemento hospedeiro (WHATWG DOM Standard §4.2.2).
    pub fn attach_shadow(
        &mut self,
        host_id: NodeId,
        mode: crate::node::ShadowMode,
    ) -> Result<NodeId, DomError> {
        let host_node = self.get_node(host_id).ok_or(DomError::InvalidNodeId(host_id))?;
        let el_data = host_node
            .as_element()
            .ok_or_else(|| DomError::HierarchyRequestError("Apenas Elementos podem hospedar Shadow DOM".into()))?;

        if el_data.shadow_root.is_some() {
            return Err(DomError::HierarchyRequestError(
                "O elemento já possui uma ShadowRoot anexada".into(),
            ));
        }

        let dummy_id = NodeId::new();
        let shadow_data = crate::node::ShadowRootData { mode, host: host_id };
        let shadow_node = NodeData::new(dummy_id, NodeKind::ShadowRoot(shadow_data));
        let arena_id = self.arena.alloc(shadow_node);
        let shadow_root_id = arena_id.to_node_id();
        if let Some(n) = self.arena.get_mut(arena_id) {
            n.id = shadow_root_id;
        }

        if let Some(host_mut) = self.get_node_mut(host_id) {
            if let Some(el_mut) = host_mut.as_element_mut() {
                el_mut.shadow_root = Some(shadow_root_id);
            }
        }

        Ok(shadow_root_id)
    }

    /// Obtém o `NodeId` da ShadowRoot associada a um elemento, se existir e for acessível.
    pub fn get_shadow_root(&self, host_id: NodeId) -> Option<NodeId> {
        self.get_node(host_id)
            .and_then(|n| n.as_element())
            .and_then(|el| el.shadow_root)
    }

    /// Clona um nó existente no documento. Se `deep == true`, clona recursivamente todos os descendentes.
    pub fn clone_node(&mut self, node_id: NodeId, deep: bool) -> Result<NodeId, DomError> {
        let original_node = self.get_node(node_id).ok_or(DomError::InvalidNodeId(node_id))?;
        let new_kind = original_node.kind.clone();

        let dummy_id = NodeId::new();
        let mut new_node = NodeData::new(dummy_id, new_kind);
        new_node.flags = original_node.flags;

        let arena_id = self.arena.alloc(new_node);
        let new_id = arena_id.to_node_id();
        if let Some(n) = self.arena.get_mut(arena_id) {
            n.id = new_id;
        }

        if deep {
            let mut children_to_clone = Vec::new();
            for (child_id, _) in self.children(node_id) {
                children_to_clone.push(child_id);
            }

            for child_id in children_to_clone {
                let cloned_child = self.clone_node(child_id, true)?;
                self.append_child(new_id, cloned_child)?;
            }
        }

        Ok(new_id)
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

    /// Retorna a lista de nós filhos imediatos na visão achatada (*Flat Tree / Composed Tree*).
    pub fn flat_tree_children(&self, node_id: NodeId) -> Vec<NodeId> {
        crate::node::FlatTreeResolver::flat_tree_children(self, node_id)
    }

    /// Serializa o nó e seus descendentes em uma string HTML normativa (`outerHTML`).
    pub fn outer_html(&self, node_id: NodeId) -> String {
        crate::serializer::serialize_node(self, node_id)
    }

    /// Serializa os filhos do nó em uma string HTML normativa (`innerHTML`).
    pub fn inner_html(&self, node_id: NodeId) -> String {
        crate::serializer::serialize_inner_html(self, node_id)
    }

    /// Retorna o número total de nós vivos alocados na arena.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.arena.len()
    }
}
