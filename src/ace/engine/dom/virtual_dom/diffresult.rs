use super::*;
//! AceDOM Virtual DOM & Diff/Patch Engine
//! Implementação otimizada para frameworks reativos (React, Solid, Svelte)
//! Objetivo: Updates 50x mais rápidos que re-renderização completa

use std::collections::HashMap;
use std::rc::Rc;
use crate::ace::engine::dom::{AceDOM, NodeId};

/// Representação leve de um nó no Virtual DOM


/// Resultado do algoritmo de Diff
pub struct DiffResult {
    pub patches: Vec<PatchOp>,
}
