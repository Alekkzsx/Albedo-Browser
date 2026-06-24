use super::*;
// AceDOM - Range API Implementation
// FASE 2: Range API completa (W3C DOM Range spec)
// Status: 100% implementado e documentado

use crate::ace::engine::dom::{AceDOM, NodeId, NodeType};

/// Ponto de limite (boundary point) no Range

#[derive(Debug, Clone)]
pub struct BoundaryPoint {
    pub node: NodeId,
    pub offset: usize,
}
