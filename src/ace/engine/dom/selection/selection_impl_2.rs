use super::*;
// AceDOM - Selection API Implementation
// FASE 2: Selection API completa (WHATWG Selection spec)
// Status: 100% implementado e documentado

use std::cell::RefCell;
use std::rc::Rc;
use crate::ace::engine::dom::{NodeId, Range};

/// Direção da seleção


impl Selection {
    
    /// Seleciona um nó inteiro (incluindo o próprio nó)
    pub fn select_all(&mut self, node: NodeId, dom: &crate::ace::engine::dom::AceDOM) {
        self.remove_all_ranges();
        
        if let Some(parent) = dom.get_node(node).and_then(|n| n.parent()) {
            let parent_id = parent.id();
            let mut range = Range::new();
            
            // Encontra o índice deste nó entre os filhos do parent
            let parent_data = dom.get_node(parent_id).expect("Albedo Engine: internal invariant violated");
            let index = parent_data.children()
                .iter()
                .position(|&child_id| child_id == node)
                .unwrap_or(0);
            
            range.set_start(parent_id, index);
            range.set_end(parent_id, index + 1);
            
            let range_rc = Rc::new(RefCell::new(range));
            self.add_range(range_rc);
        }
    }
    
    /// Deleta o conteúdo selecionado
    pub fn delete_from_document(&mut self, dom: &mut crate::ace::engine::dom::AceDOM) {
        for range_rc in &self.ranges {
            let mut range = range_rc.borrow_mut();
            range.delete_contents(dom);
        }
        
        // Após deletar, colapsa a seleção
        if !self.ranges.is_empty() {
            self.collapse_to_start();
        }
    }
    
    /// Colapsa a seleção para o início (anchor)
    pub fn collapse_to_start(&mut self) {
        if self.ranges.is_empty() {
            return;
        }
        
        let first_range = &self.ranges[0];
        let mut range = first_range.borrow_mut();
        range.collapse(true); // true = para start
        drop(range);
        self.update_from_first_range();
    }
    
    /// Colapsa a seleção para o fim (focus)
    pub fn collapse_to_end(&mut self) {
        if self.ranges.is_empty() {
            return;
        }
        
        let first_range = &self.ranges[0];
        let mut range = first_range.borrow_mut();
        range.collapse(false); // false = para end
        drop(range);
        self.update_from_first_range();
    }
    
    /// Colapsa a seleção para um ponto específico
    pub fn collapse(&mut self, node: NodeId, offset: usize) {
        self.remove_all_ranges();
        
        let mut range = Range::new();
        range.set_start(node, offset);
        range.set_end(node, offset);
        
        let range_rc = Rc::new(RefCell::new(range));
        self.add_range(range_rc);
    }
    
    /// Estende a seleção até um nó e offset
    pub fn extend(&mut self, node: NodeId, offset: usize, _dom: &crate::ace::engine::dom::AceDOM) {
        if self.ranges.is_empty() {
            // Se não há seleção, cria uma nova a partir do ponto
            self.collapse(node, offset);
            return;
        }
        
        let first_range = &self.ranges[0];
        let mut range = first_range.borrow_mut();
        
        // Mantém anchor, move focus
        let anchor_node = self.anchor_node.expect("Albedo Engine: internal invariant violated");
        let anchor_offset = self.anchor_offset;
        
        range.set_start_and_end(anchor_node, anchor_offset, node, offset);
        
        drop(range);
        self.update_from_first_range();
    }
    
    /// Define a direção da seleção explicitamente
    pub fn set_direction(&mut self, direction: SelectionDirection) {
        self.direction = direction;
    }
}
