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


impl EventPath {
    /// Computa o path completo do evento através de shadow DOMs
    pub fn compute_path(dom: &AceDOM, target_idx: usize) -> Vec<usize> {
        let mut path = Vec::new();
        let mut current = Some(target_idx);
        
        while let Some(idx) = current {
            path.push(idx);
            
            // Verifica se estamos dentro de um shadow root
            if let Some(node) = dom.get_node(idx) {
                // Se este nó é um shadow root, precisamos ajustar o path
                if node.node_type == AceNodeType::ShadowRoot {
                    // Adiciona o host ao path em vez dos ancestrais no shadow
                    if let Some(host_idx) = node.parent {
                        // Continua do host, não dos ancestrais no shadow
                        current = Some(host_idx);
                        continue;
                    }
                }
                
                current = node.parent;
            } else {
                break;
            }
        }
        
        path.reverse();
        path
    }
    
    /// Retorna o target retargeted para um observador fora do shadow DOM
    pub fn retarget_target(dom: &AceDOM, original_target: usize, observer_outside_shadow: bool) -> usize {
        if !observer_outside_shadow {
            return original_target;
        }
        
        // Encontra o limite do shadow DOM mais próximo
        let mut current = Some(original_target);
        let mut last_boundary_host = original_target;
        
        while let Some(idx) = current {
            if let Some(node) = dom.get_node(idx) {
                if node.node_type == AceNodeType::ShadowRoot {
                    // Este é um boundary - o target deve ser o host
                    if let Some(host_idx) = node.parent {
                        last_boundary_host = host_idx;
                    }
                }
                current = node.parent;
            } else {
                break;
            }
        }
        
        last_boundary_host
    }
}
