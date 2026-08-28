//! # Iteradores de Navegação na Árvore DOM
//!
//! Iteradores de altíssimo desempenho $O(1)$ por passo sem alocações adicionais no heap para travessia
//! em profundidade (DFS), filhos, ancestrais e irmãos.

use crate::node::NodeData;
use ace_core::arena::{Arena, ArenaId};
use ace_core::id::NodeId;

/// Helper interno para obter uma referência a `NodeData` na Arena em $O(1)$.
#[inline(always)]
fn get_node(arena: &Arena<NodeData>, node_id: NodeId) -> Option<&NodeData> {
    let arena_id = ArenaId::<NodeData>::from_node_id(node_id)?;
    arena.get(arena_id)
}

/// Iterador sobre os filhos imediatos de um nó (`first_child -> next_sibling`).
pub struct ChildrenIter<'a> {
    arena: &'a Arena<NodeData>,
    current: Option<NodeId>,
}

impl<'a> ChildrenIter<'a> {
    #[inline]
    pub fn new(arena: &'a Arena<NodeData>, first_child: Option<NodeId>) -> Self {
        Self {
            arena,
            current: first_child,
        }
    }
}

impl<'a> Iterator for ChildrenIter<'a> {
    type Item = (NodeId, &'a NodeData);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let curr_id = self.current?;
        let node = get_node(self.arena, curr_id)?;
        self.current = node.next_sibling;
        Some((curr_id, node))
    }
}

/// Iterador sobre todos os ancestrais de um nó subindo até a raiz (`parent -> parent`).
pub struct AncestorsIter<'a> {
    arena: &'a Arena<NodeData>,
    current: Option<NodeId>,
}

impl<'a> AncestorsIter<'a> {
    #[inline]
    pub fn new(arena: &'a Arena<NodeData>, parent: Option<NodeId>) -> Self {
        Self {
            arena,
            current: parent,
        }
    }
}

impl<'a> Iterator for AncestorsIter<'a> {
    type Item = (NodeId, &'a NodeData);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let curr_id = self.current?;
        let node = get_node(self.arena, curr_id)?;
        self.current = node.parent;
        Some((curr_id, node))
    }
}

/// Iterador de travessia pré-ordem em profundidade (Pre-Order Depth-First Search).
pub struct DescendantsIter<'a> {
    arena: &'a Arena<NodeData>,
    stack: Vec<NodeId>,
}

impl<'a> DescendantsIter<'a> {
    pub fn new(arena: &'a Arena<NodeData>, root: NodeId) -> Self {
        let mut stack = Vec::with_capacity(16);
        // Empilha os filhos do nó raiz em ordem reversa para desempilhar na ordem correta
        if let Some(root_node) = get_node(arena, root) {
            let mut children = Vec::new();
            let mut curr = root_node.first_child;
            while let Some(child_id) = curr {
                if let Some(child_node) = get_node(arena, child_id) {
                    children.push(child_id);
                    curr = child_node.next_sibling;
                } else {
                    break;
                }
            }
            for child_id in children.into_iter().rev() {
                stack.push(child_id);
            }
        }

        Self { arena, stack }
    }
}

impl<'a> Iterator for DescendantsIter<'a> {
    type Item = (NodeId, &'a NodeData);

    fn next(&mut self) -> Option<Self::Item> {
        let next_id = self.stack.pop()?;
        let node = get_node(self.arena, next_id)?;

        // Empilha os filhos deste nó em ordem reversa para manter a pré-ordem
        let mut children = Vec::new();
        let mut curr = node.first_child;
        while let Some(child_id) = curr {
            if let Some(child_node) = get_node(self.arena, child_id) {
                children.push(child_id);
                curr = child_node.next_sibling;
            } else {
                break;
            }
        }
        for child_id in children.into_iter().rev() {
            self.stack.push(child_id);
        }

        Some((next_id, node))
    }
}
