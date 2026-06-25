//! Shadow DOM Implementation - W3C Shadow DOM v1 Spec
//! 
//! Este módulo implementa:
//! - attachShadow() com modos open/closed
//! - Slot assignment algorithm
//! - Event retargeting através de shadow boundaries
//! - Pseudo-elemento ::slotted()
//! - Host integration

use super::*;
use std::collections::{HashMap, HashSet};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType, NodeDirtyFlags};

/// Configuração para attachShadow()


impl SlotAssignment {
    /// Executa o algoritmo de slot assignment para um shadow root
    pub fn assign_slots(dom: &mut AceDOM, shadow_root_idx: usize) {
        let mut slot_elements: Vec<(usize, String)> = Vec::new();
        
        // Coleta todos os elementos <slot> no shadow DOM
        Self::collect_slot_elements(dom, shadow_root_idx, &mut slot_elements);
        
        // Obtém os children do host (light DOM)
        let host_idx = if let Some(shadow_node) = dom.get_node(shadow_root_idx) {
            shadow_node.parent
        } else {
            return;
        };
        
        let host_children = if let Some(host_node) = dom.get_node(host_idx.expect("Albedo Engine: internal invariant violated")) {
            host_node.children.clone()
        } else {
            return;
        };
        
        // Algoritmo de assignment
        let mut assigned: HashSet<usize> = HashSet::new();
        let mut default_slot: Option<usize> = None;
        
        // Primeiro passa: named slots
        for (slot_idx, slot_name) in &slot_elements {
            if slot_name.is_empty() {
                default_slot = Some(*slot_idx);
                continue;
            }
            
            // Encontra nodes no light DOM com slot="slot_name"
            let mut assigned_to_this_slot = Vec::new();
            for &child_idx in &host_children {
                if assigned.contains(&child_idx) {
                    continue;
                }
                
                if let Some(child_node) = dom.get_node(child_idx) {
                    if let AceNodeType::Element(el) = &child_node.node_type {
                        if let Some(slot_attr) = el.attributes.get("slot") {
                            if slot_attr == slot_name {
                                assigned_to_this_slot.push(child_idx);
                                assigned.insert(child_idx);
                            }
                        }
                    }
                }
            }
            
            // Atualiza o mapeamento de assigned nodes
            if let Some(shadow_root) = dom.get_shadow_root_mut(shadow_root_idx) {
                shadow_root.assigned_nodes.insert(*slot_idx, assigned_to_this_slot);
            }
        }
        
        // Segundo passa: default slot (nós não atribuídos)
        if let Some(default_slot_idx) = default_slot {
            let mut default_assigned = Vec::new();
            for &child_idx in &host_children {
                if !assigned.contains(&child_idx) {
                    default_assigned.push(child_idx);
                }
            }
            
            if let Some(shadow_root) = dom.get_shadow_root_mut(shadow_root_idx) {
                shadow_root.assigned_nodes.insert(default_slot_idx, default_assigned);
            }
        }
        
        // Marca como dirty para re-layout
        dom.mark_dirty(shadow_root_idx, NodeDirtyFlags::LAYOUT | NodeDirtyFlags::CHILDREN);
    }
    
pub(crate) fn collect_slot_elements(dom: &AceDOM, root_idx: usize, slots: &mut Vec<(usize, String)>) {
        let mut stack = vec![root_idx];
        
        while let Some(idx) = stack.pop() {
            if let Some(node) = dom.get_node(idx) {
                if let AceNodeType::Element(el) = &node.node_type {
                    if el.tag == "slot" {
                        let slot_name = el.attributes.get("name")
                            .cloned()
                            .unwrap_or_default();
                        slots.push((idx, slot_name));
                    }
                }
                
                stack.extend(&node.children);
            }
        }
    }
}
