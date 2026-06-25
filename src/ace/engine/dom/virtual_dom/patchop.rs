use super::*;
// AceDOM Virtual DOM & Diff/Patch Engine
// Implementação otimizada para frameworks reativos (React, Solid, Svelte)
// Objetivo: Updates 50x mais rápidos que re-renderização completa

use std::collections::HashMap;
use std::rc::Rc;
use crate::ace::engine::dom::{AceDOM, NodeId};

/// Representação leve de um nó no Virtual DOM


/// Tipos de operações de Patch
#[derive(Debug, Clone)]
pub enum PatchOp {
    /// Inserir nó em índice específico
    Insert { index: usize, node: VNode },
    /// Remover nó em índice específico
    Remove { index: usize },
    /// Substituir nó em índice específico
    Replace { index: usize, node: VNode },
    /// Atualizar atributo
    SetAttribute { index: usize, key: Rc<str>, value: Rc<str> },
    /// Remover atributo
    RemoveAttribute { index: usize, key: Rc<str> },
    /// Atualizar texto
    SetText { index: usize, text: Rc<str> },
    /// Mover nó de um índice para outro
    Move { from: usize, to: usize },
}
