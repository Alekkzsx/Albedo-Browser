//! # Mutações Estruturais e Manipulação de Links da Árvore DOM
//!
//! Operações atômicas de conexão e desconexão de ponteiros na `Arena<NodeData>`.

use crate::error::DomError;
use crate::node::NodeData;
use ace_core::arena::{Arena, ArenaId};
use ace_core::id::NodeId;

#[inline(always)]
fn to_arena_id(id: NodeId) -> Result<ArenaId<NodeData>, DomError> {
    ArenaId::<NodeData>::from_node_id(id).ok_or(DomError::InvalidNodeId(id))
}

/// Anexa `child_id` como o último filho de `parent_id`.
pub fn append_child(
    arena: &mut Arena<NodeData>,
    parent_id: NodeId,
    child_id: NodeId,
) -> Result<(), DomError> {
    if parent_id == child_id {
        return Err(DomError::HierarchyRequestError(
            "Um nó não pode ser filho de si mesmo".into(),
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

    if new_child_id == ref_id || parent_id == new_child_id {
        return Err(DomError::HierarchyRequestError(
            "Hierarquia de inserção inválida".into(),
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
