use super::*;
// AceDOM - Range API Implementation
// FASE 2: Range API completa (W3C DOM Range spec)
// Status: 100% implementado e documentado

use crate::ace::engine::dom::{AceDOM, NodeId, NodeType};

/// Ponto de limite (boundary point) no Range


impl Range {
    /// Cria um novo range colapsado no início do documento
    pub fn new() -> Self {
        Range {
            start_container: 0,
            start_offset: 0,
            end_container: 0,
            end_offset: 0,
            collapsed: true,
            common_ancestor: Some(0),
        }
    }
    
    /// Cria um range a partir de boundary points específicos
    pub fn with_boundaries(start_node: NodeId, start_offset: usize, 
                           end_node: NodeId, end_offset: usize) -> Self {
        let mut range = Range::new();
        range.set_start_and_end(start_node, start_offset, end_node, end_offset);
        range
    }
    
    /// Retorna o container de início
    pub fn start_container(&self) -> NodeId {
        self.start_container
    }
    
    /// Retorna o offset de início
    pub fn start_offset(&self) -> usize {
        self.start_offset
    }
    
    /// Retorna o container de fim
    pub fn end_container(&self) -> NodeId {
        self.end_container
    }
    
    /// Retorna o offset de fim
    pub fn end_offset(&self) -> usize {
        self.end_offset
    }
    
    /// Retorna true se o range está colapsado
    pub fn collapsed(&self) -> bool {
        self.collapsed
    }
    
    /// Retorna o common ancestor container
    pub fn common_ancestor_container(&self) -> Option<NodeId> {
        self.common_ancestor
    }
    
    /// Define o start boundary point
    pub fn set_start(&mut self, node: NodeId, offset: usize) {
        self.start_container = node;
        self.start_offset = offset;
        
        // Verifica se colapsou
        self.collapsed = (self.start_container == self.end_container) && 
                         (self.start_offset == self.end_offset);
        
        self.update_common_ancestor();
    }
    
    /// Define o end boundary point
    pub fn set_end(&mut self, node: NodeId, offset: usize) {
        self.end_container = node;
        self.end_offset = offset;
        
        // Verifica se colapsou
        self.collapsed = (self.start_container == self.end_container) && 
                         (self.start_offset == self.end_offset);
        
        self.update_common_ancestor();
    }
    
    /// Define start e end simultaneamente
    pub fn set_start_and_end(&mut self, 
                             start_node: NodeId, start_offset: usize,
                             end_node: NodeId, end_offset: usize) {
        self.start_container = start_node;
        self.start_offset = start_offset;
        self.end_container = end_node;
        self.end_offset = end_offset;
        
        self.collapsed = (start_node == end_node) && (start_offset == end_offset);
        self.update_common_ancestor();
    }
    
    /// Colapsa o range para start ou end
    pub fn collapse(&mut self, to_start: bool) {
        if to_start {
            self.end_container = self.start_container;
            self.end_offset = self.start_offset;
        } else {
            self.start_container = self.end_container;
            self.start_offset = self.end_offset;
        }
        self.collapsed = true;
    }
    
}
