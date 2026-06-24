use super::*;
// AceDOM - Range API Implementation
// FASE 2: Range API completa (W3C DOM Range spec)
// Status: 100% implementado e documentado

use crate::ace::engine::dom::{AceDOM, NodeId, NodeType};

/// Ponto de limite (boundary point) no Range


#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::engine::dom::AceDOM;
    
    #[test]
pub(crate) fn test_range_new_is_collapsed() {
        let range = Range::new();
        assert!(range.collapsed());
        assert_eq!(range.start_offset(), 0);
        assert_eq!(range.end_offset(), 0);
    }
    
    #[test]
pub(crate) fn test_set_start_end() {
        let mut range = Range::new();
        range.set_start(5, 10);
        range.set_end(5, 20);
        
        assert_eq!(range.start_container(), 5);
        assert_eq!(range.start_offset(), 10);
        assert_eq!(range.end_container(), 5);
        assert_eq!(range.end_offset(), 20);
        assert!(!range.collapsed());
    }
    
    #[test]
pub(crate) fn test_collapse_to_start() {
        let mut range = Range::with_boundaries(0, 5, 0, 15);
        assert!(!range.collapsed());
        
        range.collapse(true);
        
        assert!(range.collapsed());
        assert_eq!(range.start_offset(), 5);
        assert_eq!(range.end_offset(), 5);
    }
    
    #[test]
pub(crate) fn test_collapse_to_end() {
        let mut range = Range::with_boundaries(0, 5, 0, 15);
        
        range.collapse(false);
        
        assert!(range.collapsed());
        assert_eq!(range.start_offset(), 15);
        assert_eq!(range.end_offset(), 15);
    }
    
    #[test]
pub(crate) fn test_compare_boundary_points() {
        let range1 = Range::with_boundaries(0, 5, 0, 10);
        let range2 = Range::with_boundaries(0, 3, 0, 8);
        
        // START_TO_START: range1.start (5) vs range2.start (3)
        assert_eq!(range1.compare_boundary_points(&range2, 0), 1);
        
        // END_TO_END: range1.end (10) vs range2.end (8)
        assert_eq!(range1.compare_boundary_points(&range2, 3), 1);
    }
    
    #[test]
pub(crate) fn test_to_string_same_node() {
        let mut dom = AceDOM::from_html("<p>Hello World</p>");
        let mut range = Range::new();
        
        let p_node = dom.query_selector("p").expect("Albedo Engine: internal invariant violated");
        range.set_start(p_node, 0);
        range.set_end(p_node, 1);
        
        let text = range.to_string(&dom);
        assert!(!text.is_empty());
    }
    
    #[test]
pub(crate) fn test_select_node_contents() {
        let mut dom = AceDOM::from_html("<div><p>A</p><p>B</p></div>");
        let mut range = Range::new();
        
        let div_node = dom.query_selector("div").expect("Albedo Engine: internal invariant violated");
        range.select_node_contents(div_node, &dom);
        
        assert_eq!(range.start_container(), div_node);
        assert_eq!(range.start_offset(), 0);
        assert!(!range.collapsed());
    }
    
    #[test]
pub(crate) fn test_delete_contents() {
        let mut dom = AceDOM::from_html("<p>Hello World</p>");
        let mut range = Range::new();
        
        let p_node = dom.query_selector("p").expect("Albedo Engine: internal invariant violated");
        
        // Precisa acessar o text node filho
        if let Some(p_data) = dom.get_node(p_node) {
            if let Some(&text_child) = p_data.children().first() {
                if let Some(text_data) = dom.get_node(text_child) {
                    let text_len = text_data.text_content().unwrap_or_default().len();
                    range.set_start(text_child, 0);
                    range.set_end(text_child, text_len);
                    
                    let before = dom.to_html();
                    assert!(before.contains("Hello"));
                    
                    range.delete_contents(&mut dom);
                    
                    let after = dom.to_html();
                    assert!(!after.contains("Hello"));
                }
            }
        }
    }
}
