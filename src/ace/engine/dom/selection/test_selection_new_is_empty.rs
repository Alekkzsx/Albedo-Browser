use super::*;
// AceDOM - Selection API Implementation
// FASE 2: Selection API completa (WHATWG Selection spec)
// Status: 100% implementado e documentado

use std::cell::RefCell;
use std::rc::Rc;
use crate::ace::engine::dom::{NodeId, Range};

/// Direção da seleção


#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::engine::dom::AceDOM;
    
    #[test]
pub(crate) fn test_selection_new_is_empty() {
        let selection = Selection::new();
        assert!(selection.is_empty());
        assert_eq!(selection.range_count(), 0);
        assert_eq!(selection.selection_type(), SelectionType::None);
    }
    
    #[test]
pub(crate) fn test_add_range_updates_state() {
        let mut dom = AceDOM::from_html("<p>Hello <strong>World</strong></p>");
        let mut selection = Selection::new();
        
        let mut range = Range::new();
        // Seleciona "Hello "
        let p_node = dom.query_selector("p").expect("Albedo Engine: internal invariant violated");
        range.set_start(p_node, 0);
        range.set_end(p_node, 1);
        
        let range_rc = Rc::new(RefCell::new(range));
        selection.add_range(range_rc);
        
        assert_eq!(selection.range_count(), 1);
        assert!(!selection.is_collapsed());
        assert_eq!(selection.selection_type(), SelectionType::Range);
    }
    
    #[test]
pub(crate) fn test_remove_range_clears_if_last() {
        let mut selection = Selection::new();
        let mut range = Range::new();
        range.set_start(0, 0);
        range.set_end(0, 5);
        let range_rc = Rc::new(RefCell::new(range));
        
        selection.add_range(range_rc.clone());
        assert!(!selection.is_empty());
        
        selection.remove_range(&range_rc);
        assert!(selection.is_empty());
        assert_eq!(selection.selection_type(), SelectionType::None);
    }
    
    #[test]
pub(crate) fn test_collapse_to_start() {
        let mut dom = AceDOM::from_html("<div><p>Hello World</p></div>");
        let mut selection = Selection::new();
        
        let mut range = Range::new();
        let p_node = dom.query_selector("p").expect("Albedo Engine: internal invariant violated");
        range.set_start(p_node, 0);
        range.set_end(p_node, 5); // "Hello"
        
        let range_rc = Rc::new(RefCell::new(range));
        selection.add_range(range_rc);
        
        assert!(!selection.is_collapsed());
        
        selection.collapse_to_start();
        
        assert!(selection.is_collapsed());
        assert_eq!(selection.selection_type(), SelectionType::Caret);
    }
    
    #[test]
pub(crate) fn test_delete_from_document() {
        let mut dom = AceDOM::from_html("<p>Hello <strong>World</strong>!</p>");
        let mut selection = Selection::new();
        
        let mut range = Range::new();
        let p_node = dom.query_selector("p").expect("Albedo Engine: internal invariant violated");
        range.set_start(p_node, 0);
        range.set_end(p_node, 1); // Seleciona todo o conteúdo do parágrafo
        
        let range_rc = Rc::new(RefCell::new(range));
        selection.add_range(range_rc);
        
        let before_html = dom.to_html();
        assert!(before_html.contains("Hello"));
        
        selection.delete_from_document(&mut dom);
        
        let after_html = dom.to_html();
        // Conteúdo deve ter sido removido
        assert!(!after_html.contains("Hello"));
    }
    
    #[test]
pub(crate) fn test_select_all_children() {
        let mut dom = AceDOM::from_html("<div><p>A</p><p>B</p><p>C</p></div>");
        let mut selection = Selection::new();
        
        let div_node = dom.query_selector("div").expect("Albedo Engine: internal invariant violated");
        selection.select_all_children(div_node, &dom);
        
        assert_eq!(selection.range_count(), 1);
        assert!(!selection.is_collapsed());
        
        let text = selection.to_string(&dom);
        assert!(text.contains("A"));
        assert!(text.contains("B"));
        assert!(text.contains("C"));
    }
    
    #[test]
pub(crate) fn test_extend_selection() {
        let mut dom = AceDOM::from_html("<p>Hello World Foo Bar</p>");
        let mut selection = Selection::new();
        
        // Começa com seleção colapsada em "Hello"
        selection.collapse(0, 0);
        
        // Estende para "World"
        selection.extend(0, 2, &dom);
        
        assert!(!selection.is_collapsed());
        assert_eq!(selection.selection_type(), SelectionType::Range);
    }
    
    #[test]
pub(crate) fn test_to_string_multiple_ranges() {
        // Nota: implementação atual só suporta 1 range, mas estrutura permite múltiplos
        let mut dom = AceDOM::from_html("<p>Line 1</p><p>Line 2</p>");
        let mut selection = Selection::new();
        
        let mut range = Range::new();
        let p1 = dom.query_selector_all("p")[0];
        range.set_start(p1, 0);
        range.set_end(p1, 1);
        
        let range_rc = Rc::new(RefCell::new(range));
        selection.add_range(range_rc);
        
        let text = selection.to_string(&dom);
        assert!(!text.is_empty());
    }
}
