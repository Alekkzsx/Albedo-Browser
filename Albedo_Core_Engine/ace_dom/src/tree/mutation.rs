//! # Mutações Estruturais e Manipulação de Links da Árvore DOM
//!
//! Operações atômicas de conexão e desconexão de ponteiros na `Arena<NodeData>`
//! com integração normativa à pilha de reações de ciclo de vida de Custom Elements.

use crate::custom_elements::reaction_stack::CustomElementReactionsStack;
use crate::custom_elements::CustomElementState;
use crate::error::DomError;
use crate::node::{NodeData, NodeKind};
use ace_core::arena::{Arena, ArenaId};
use ace_core::id::NodeId;

#[inline(always)]
fn to_arena_id(id: NodeId) -> Result<ArenaId<NodeData>, DomError> {
    ArenaId::<NodeData>::from_node_id(id).ok_or(DomError::InvalidNodeId(id))
}

/// Verifica se um nó está conectado à raiz de um documento ativo (WHATWG DOM §4.2.2).
pub fn is_connected_to_document(arena: &Arena<NodeData>, node_id: NodeId) -> bool {
    let mut curr = Some(node_id);
    while let Some(curr_id) = curr {
        if let Some(aid) = ArenaId::<NodeData>::from_node_id(curr_id) {
            if let Some(node) = arena.get(aid) {
                if matches!(node.kind, NodeKind::Document(_)) {
                    return true;
                }
                if node.parent.is_none() {
                    return false;
                }
                curr = node.parent;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    false
}

/// Coleta todos os `NodeId`s da subárvore em ordem de travessia pré-ordem em profundidade (Tree Order).
pub fn collect_subtree_node_ids(arena: &Arena<NodeData>, root_id: NodeId) -> Vec<NodeId> {
    let mut result = Vec::new();
    let mut stack = vec![root_id];

    while let Some(current_id) = stack.pop() {
        result.push(current_id);
        if let Some(aid) = ArenaId::<NodeData>::from_node_id(current_id) {
            if let Some(node) = arena.get(aid) {
                let mut children = Vec::new();
                let mut curr_child = node.first_child;
                while let Some(c_id) = curr_child {
                    children.push(c_id);
                    if let Some(c_aid) = ArenaId::<NodeData>::from_node_id(c_id) {
                        curr_child = arena.get(c_aid).and_then(|c| c.next_sibling);
                    } else {
                        break;
                    }
                }
                for &c_id in children.iter().rev() {
                    stack.push(c_id);
                }
            }
        }
    }

    result
}

/// Enfileira a reação `Connected` para todos os elementos customizados na subárvore em ordem da árvore.
pub fn enqueue_connected_subtree(
    arena: &Arena<NodeData>,
    root_id: NodeId,
    reactions: &mut CustomElementReactionsStack,
) {
    let nodes = collect_subtree_node_ids(arena, root_id);
    for node_id in nodes {
        if let Some(aid) = ArenaId::<NodeData>::from_node_id(node_id) {
            if let Some(node) = arena.get(aid) {
                if let Some(el) = node.as_element() {
                    if el.custom_element_state() == CustomElementState::Custom {
                        reactions.enqueue_connected(node_id);
                    }
                }
            }
        }
    }
}

/// Enfileira a reação `Disconnected` para todos os elementos customizados na subárvore em ordem da árvore.
pub fn enqueue_disconnected_subtree(
    arena: &Arena<NodeData>,
    root_id: NodeId,
    reactions: &mut CustomElementReactionsStack,
) {
    let nodes = collect_subtree_node_ids(arena, root_id);
    for node_id in nodes {
        if let Some(aid) = ArenaId::<NodeData>::from_node_id(node_id) {
            if let Some(node) = arena.get(aid) {
                if let Some(el) = node.as_element() {
                    if el.custom_element_state() == CustomElementState::Custom {
                        reactions.enqueue_disconnected(node_id);
                    }
                }
            }
        }
    }
}

/// Verifica se `ancestor` é um ancestral inclusivo de `node`.
fn is_inclusive_ancestor(arena: &Arena<NodeData>, ancestor: NodeId, node: NodeId) -> bool {
    if ancestor == node {
        return true;
    }
    let mut curr = Some(node);
    while let Some(curr_id) = curr {
        if curr_id == ancestor {
            return true;
        }
        if let Some(aid) = ArenaId::<NodeData>::from_node_id(curr_id) {
            curr = arena.get(aid).and_then(|n| n.parent);
        } else {
            break;
        }
    }
    false
}

/// Anexa `child_id` como o último filho de `parent_id`.
pub fn append_child(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    child_id: NodeId,
) -> Result<(), DomError> {
    if is_inclusive_ancestor(arena, child_id, parent_id) {
        return Err(DomError::HierarchyRequestError(
            "Cannot insert an ancestor into a descendant".into(),
        ));
    }

    let p_aid = to_arena_id(parent_id)?;
    let c_aid = to_arena_id(child_id)?;

    // Desconecta o nó de qualquer pai anterior
    let prev_parent = arena.get(c_aid).ok_or(DomError::InvalidNodeId(child_id))?.parent;
    if let Some(pp_id) = prev_parent {
        remove_child(arena, pp_id, child_id)?;
    }

    let parent_last_child = arena.get(p_aid).ok_or(DomError::InvalidNodeId(parent_id))?.last_child;

    if let Some(last_id) = parent_last_child {
        let last_aid = to_arena_id(last_id)?;
        if let Some(last_node) = arena.get_mut(last_aid) {
            last_node.next_sibling = Some(child_id);
        }
        if let Some(child_node) = arena.get_mut(c_aid) {
            child_node.parent = Some(parent_id);
            child_node.prev_sibling = Some(last_id);
            child_node.next_sibling = None;
        }
        if let Some(parent_node) = arena.get_mut(p_aid) {
            parent_node.last_child = Some(child_id);
        }
    } else {
        // Primeiro filho do pai
        if let Some(child_node) = arena.get_mut(c_aid) {
            child_node.parent = Some(parent_id);
            child_node.prev_sibling = None;
            child_node.next_sibling = None;
        }
        if let Some(parent_node) = arena.get_mut(p_aid) {
            parent_node.first_child = Some(child_id);
            parent_node.last_child = Some(child_id);
        }
    }

    Ok(())
}

/// Anexa `child_id` como o último filho de `parent_id`, enfileirando reações de ciclo de vida se conectado.
pub fn append_child_with_reactions(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    child_id: NodeId,
    reactions: &mut CustomElementReactionsStack,
) -> Result<(), DomError> {
    let was_connected_before = is_connected_to_document(arena, child_id);
    let parent_connected = is_connected_to_document(arena, parent_id);

    append_child(arena, parent_id, child_id)?;

    if !was_connected_before && parent_connected {
        enqueue_connected_subtree(arena, child_id, reactions);
    }

    Ok(())
}

/// Insere `new_child_id` imediatamente antes de `ref_child_id` sob o pai `parent_id`.
pub fn insert_before(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    new_child_id: NodeId,
    ref_child_id: Option<NodeId>,
) -> Result<(), DomError> {
    let ref_id = match ref_child_id {
        Some(id) => id,
        None => return append_child(arena, parent_id, new_child_id),
    };

    if new_child_id == ref_id || is_inclusive_ancestor(arena, new_child_id, parent_id) {
        return Err(DomError::HierarchyRequestError(
            "Cannot insert an ancestor into a descendant".into(),
        ));
    }

    let p_aid = to_arena_id(parent_id)?;
    let new_aid = to_arena_id(new_child_id)?;
    let ref_aid = to_arena_id(ref_id)?;

    // Desconecta o novo nó se já tiver pai
    let prev_parent = arena.get(new_aid).ok_or(DomError::InvalidNodeId(new_child_id))?.parent;
    if let Some(pp_id) = prev_parent {
        remove_child(arena, pp_id, new_child_id)?;
    }

    let ref_node = arena.get(ref_aid).ok_or(DomError::InvalidNodeId(ref_id))?;
    if ref_node.parent != Some(parent_id) {
        return Err(DomError::NotFoundError);
    }
    let ref_prev_sibling = ref_node.prev_sibling;

    // Atualiza o irmão anterior do reference node
    if let Some(prev_id) = ref_prev_sibling {
        let prev_aid = to_arena_id(prev_id)?;
        if let Some(prev_node) = arena.get_mut(prev_aid) {
            prev_node.next_sibling = Some(new_child_id);
        }
    } else {
        // O ref_node era o first_child
        if let Some(parent_node) = arena.get_mut(p_aid) {
            parent_node.first_child = Some(new_child_id);
        }
    }

    // Configura o new_child
    if let Some(new_node) = arena.get_mut(new_aid) {
        new_node.parent = Some(parent_id);
        new_node.prev_sibling = ref_prev_sibling;
        new_node.next_sibling = Some(ref_id);
    }

    // Atualiza o ref_node
    if let Some(ref_node) = arena.get_mut(ref_aid) {
        ref_node.prev_sibling = Some(new_child_id);
    }

    Ok(())
}

/// Insere `new_child_id` imediatamente antes de `ref_child_id`, enfileirando reações de ciclo de vida se conectado.
pub fn insert_before_with_reactions(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    new_child_id: NodeId,
    ref_child_id: Option<NodeId>,
    reactions: &mut CustomElementReactionsStack,
) -> Result<(), DomError> {
    let was_connected_before = is_connected_to_document(arena, new_child_id);
    let parent_connected = is_connected_to_document(arena, parent_id);

    insert_before(arena, parent_id, new_child_id, ref_child_id)?;

    if !was_connected_before && parent_connected {
        enqueue_connected_subtree(arena, new_child_id, reactions);
    }

    Ok(())
}

/// Remove `child_id` dos filhos de `parent_id`.
pub fn remove_child(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    child_id: NodeId,
) -> Result<(), DomError> {
    let p_aid = to_arena_id(parent_id)?;
    let c_aid = to_arena_id(child_id)?;

    let child_node = arena.get(c_aid).ok_or(DomError::InvalidNodeId(child_id))?;
    if child_node.parent != Some(parent_id) {
        return Err(DomError::NotFoundError);
    }

    let prev_s = child_node.prev_sibling;
    let next_s = child_node.next_sibling;

    // Atualiza anterior
    if let Some(prev_id) = prev_s {
        let prev_aid = to_arena_id(prev_id)?;
        if let Some(prev_node) = arena.get_mut(prev_aid) {
            prev_node.next_sibling = next_s;
        }
    } else {
        if let Some(parent_node) = arena.get_mut(p_aid) {
            parent_node.first_child = next_s;
        }
    }

    // Atualiza posterior
    if let Some(next_id) = next_s {
        let next_aid = to_arena_id(next_id)?;
        if let Some(next_node) = arena.get_mut(next_aid) {
            next_node.prev_sibling = prev_s;
        }
    } else {
        if let Some(parent_node) = arena.get_mut(p_aid) {
            parent_node.last_child = prev_s;
        }
    }

    // Limpa links do nó removido
    if let Some(child_node) = arena.get_mut(c_aid) {
        child_node.parent = None;
        child_node.prev_sibling = None;
        child_node.next_sibling = None;
    }

    Ok(())
}

/// Remove `child_id` dos filhos de `parent_id`, enfileirando reações de desconexão se estava conectado ao documento.
pub fn remove_child_with_reactions(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    child_id: NodeId,
    reactions: &mut CustomElementReactionsStack,
) -> Result<(), DomError> {
    let was_connected = is_connected_to_document(arena, parent_id);

    remove_child(arena, parent_id, child_id)?;

    if was_connected {
        enqueue_disconnected_subtree(arena, child_id, reactions);
    }

    Ok(())
}

/// Substitui `old_child_id` por `new_child_id` sob `parent_id`.
pub fn replace_child(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    new_child_id: NodeId,
    old_child_id: NodeId,
) -> Result<(), DomError> {
    if new_child_id == old_child_id {
        return Ok(());
    }
    insert_before(arena, parent_id, new_child_id, Some(old_child_id))?;
    remove_child(arena, parent_id, old_child_id)?;
    Ok(())
}

/// Substitui `old_child_id` por `new_child_id`, gerenciando as reações de desconexão e conexão de ciclo de vida.
pub fn replace_child_with_reactions(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    new_child_id: NodeId,
    old_child_id: NodeId,
    reactions: &mut CustomElementReactionsStack,
) -> Result<(), DomError> {
    if new_child_id == old_child_id {
        return Ok(());
    }
    let parent_connected = is_connected_to_document(arena, parent_id);

    insert_before(arena, parent_id, new_child_id, Some(old_child_id))?;
    remove_child(arena, parent_id, old_child_id)?;

    if parent_connected {
        enqueue_disconnected_subtree(arena, old_child_id, reactions);
        enqueue_connected_subtree(arena, new_child_id, reactions);
    }

    Ok(())
}

/// Insere `child_id` como o primeiro filho de `parent_id`.
pub fn prepend_child(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    child_id: NodeId,
) -> Result<(), DomError> {
    let p_aid = to_arena_id(parent_id)?;
    let first_child = arena.get(p_aid).ok_or(DomError::InvalidNodeId(parent_id))?.first_child;
    insert_before(arena, parent_id, child_id, first_child)
}

/// Insere `child_id` como o primeiro filho de `parent_id` com propagação de reações.
pub fn prepend_child_with_reactions(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    child_id: NodeId,
    reactions: &mut CustomElementReactionsStack,
) -> Result<(), DomError> {
    let p_aid = to_arena_id(parent_id)?;
    let first_child = arena.get(p_aid).ok_or(DomError::InvalidNodeId(parent_id))?.first_child;
    insert_before_with_reactions(arena, parent_id, child_id, first_child, reactions)
}

/// Insere `new_child_id` imediatamente após `ref_child_id` sob `parent_id`.
pub fn insert_after(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    new_child_id: NodeId,
    ref_child_id: NodeId,
) -> Result<(), DomError> {
    let ref_aid = to_arena_id(ref_child_id)?;
    let next_sibling = arena.get(ref_aid).ok_or(DomError::InvalidNodeId(ref_child_id))?.next_sibling;
    insert_before(arena, parent_id, new_child_id, next_sibling)
}

/// Insere `new_child_id` imediatamente após `ref_child_id` com propagação de reações.
pub fn insert_after_with_reactions(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    new_child_id: NodeId,
    ref_child_id: NodeId,
    reactions: &mut CustomElementReactionsStack,
) -> Result<(), DomError> {
    let ref_aid = to_arena_id(ref_child_id)?;
    let next_sibling = arena.get(ref_aid).ok_or(DomError::InvalidNodeId(ref_child_id))?.next_sibling;
    insert_before_with_reactions(arena, parent_id, new_child_id, next_sibling, reactions)
}
