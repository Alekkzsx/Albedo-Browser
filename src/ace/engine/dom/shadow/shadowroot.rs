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


/// ShadowRoot - raiz da árvore shadow DOM
#[derive(Clone, Debug)]
pub struct ShadowRoot {
    pub host_idx: usize,
    pub root_idx: usize,
    pub mode: ShadowRootMode,
    pub delegates_focus: bool,
    pub slots: HashMap<String, usize>, // slot name -> slot element index
    pub assigned_nodes: HashMap<usize, Vec<usize>>, // slot idx -> assigned nodes
}

impl ShadowRoot {
    /// TODO: add docs
    pub fn new(host_idx: usize, root_idx: usize, init: ShadowRootInit) -> Self {
        Self {
            host_idx,
            root_idx,
            mode: init.mode,
            delegates_focus: init.delegates_focus,
            slots: HashMap::new(),
            assigned_nodes: HashMap::new(),
        }
    }
    
    /// Acessa o shadow root apenas se mode for open
    pub fn get_root_idx(&self) -> Option<usize> {
        match self.mode {
            ShadowRootMode::Open => Some(self.root_idx),
            ShadowRootMode::Closed => None,
        }
    }
}
