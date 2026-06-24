use super::*;
// AceDOM - Selection API Implementation
// FASE 2: Selection API completa (WHATWG Selection spec)
// Status: 100% implementado e documentado

use std::cell::RefCell;
use std::rc::Rc;
use crate::ace::engine::dom::{NodeId, Range};

/// Direção da seleção


impl Selection {
    /// Cria uma nova seleção vazia
    pub fn new() -> Self {
        Selection {
            ranges: Vec::new(),
            direction: SelectionDirection::None,
            anchor_node: None,
            anchor_offset: 0,
            focus_node: None,
            focus_offset: 0,
            collapsed: true,
            associated_range: None,
        }
    }
    
    /// Retorna o número de ranges na seleção
    pub fn range_count(&self) -> usize {
        self.ranges.len()
    }
    
    /// Retorna true se a seleção está vazia
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
    
    /// Retorna true se a seleção está colapsada (cursor sem extensão)
    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }
    
    /// Retorna o tipo de seleção
    pub fn selection_type(&self) -> SelectionType {
        if self.ranges.is_empty() {
            SelectionType::None
        } else if self.collapsed {
            SelectionType::Caret
        } else {
            SelectionType::Range
        }
    }
    
    /// Obtém um range pelo índice
    pub fn get_range_at(&self, index: usize) -> Option<Rc<RefCell<Range>>> {
        self.ranges.get(index).cloned()
    }
    
    /// Adiciona um range à seleção
    /// Se já existir, substitui ou adiciona dependendo da implementação
    pub fn add_range(&mut self, range: Rc<RefCell<Range>>) {
        // Para navegadores baseados em Gecko/Firefox: apenas 1 range permitido
        // Para WebKit/Blink: múltiplos ranges permitidos (Ctrl+click)
        
        // Implementação conservadora: 1 range máximo (compatível com maioria dos sites)
        if !self.ranges.is_empty() {
            self.remove_all_ranges();
        }
        
        let range_ref = range.borrow();
        self.anchor_node = Some(range_ref.start_container());
        self.anchor_offset = range_ref.start_offset();
        self.focus_node = Some(range_ref.end_container());
        self.focus_offset = range_ref.end_offset();
        self.collapsed = range_ref.collapsed();
        
        drop(range_ref);
        
        self.ranges.push(range);
        self.update_direction();
    }
    
    /// Remove um range específico da seleção
    pub fn remove_range(&mut self, range: &Rc<RefCell<Range>>) {
        let len = self.ranges.len();
        self.ranges.retain(|r| !Rc::ptr_eq(r, range));
        
        if self.ranges.is_empty() {
            self.clear_selection_state();
        } else if len != self.ranges.len() {
            // Atualiza estado baseado no primeiro range restante
            self.update_from_first_range();
        }
    }
    
    /// Remove todos os ranges da seleção
    pub fn remove_all_ranges(&mut self) {
        self.ranges.clear();
        self.clear_selection_state();
    }
    
    /// Seleciona todo o conteúdo de um nó
    pub fn select_all_children(&mut self, node: NodeId, dom: &crate::ace::engine::dom::AceDOM) {
        self.remove_all_ranges();
        
        if let Some(node_data) = dom.get_node(node) {
            let mut range = Range::new();
            
            // Start: antes do primeiro filho
            range.set_start(node, 0);
            
            // End: depois do último filho
            let child_count = node_data.children().len();
            range.set_end(node, child_count);
            
            let range_rc = Rc::new(RefCell::new(range));
            self.add_range(range_rc);
        }
    }
}
