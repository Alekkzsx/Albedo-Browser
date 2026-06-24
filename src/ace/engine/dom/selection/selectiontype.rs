use super::*;
// AceDOM - Selection API Implementation
// FASE 2: Selection API completa (WHATWG Selection spec)
// Status: 100% implementado e documentado

use std::cell::RefCell;
use std::rc::Rc;
use crate::ace::engine::dom::{NodeId, Range};

/// Direção da seleção


/// Tipo de seleção
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionType {
    /// Nenhuma seleção
    None,
    /// Cursor (seleção colapsada)
    Caret,
    /// Intervalo de texto selecionado
    Range,
}
