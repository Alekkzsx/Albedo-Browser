use super::*;
// AceDOM Virtual DOM & Diff/Patch Engine
// Implementação otimizada para frameworks reativos (React, Solid, Svelte)
// Objetivo: Updates 50x mais rápidos que re-renderização completa

use std::collections::HashMap;
use std::rc::Rc;
use crate::ace::engine::dom::{AceDOM, NodeId};

/// Representação leve de um nó no Virtual DOM


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
pub(crate) fn test_diff_text_change() {
        let old = VNode::text("Hello");
        let new = VNode::text("World");
        
        let result = VirtualDom::diff(&old, &new);
        assert_eq!(result.patches.len(), 1);
        assert!(matches!(result.patches[0], PatchOp::SetText { .. }));
    }

    #[test]
pub(crate) fn test_diff_attr_change() {
        let old = VNode::element("div", [("class", "old")].into_iter().collect(), vec![]);
        let new = VNode::element("div", [("class", "new")].into_iter().collect(), vec![]);
        
        let result = VirtualDom::diff(&old, &new);
        assert_eq!(result.patches.len(), 1);
    }

    #[test]
pub(crate) fn test_diff_add_child() {
        let old = VNode::element("ul", HashMap::new(), vec![
            VNode::element("li", HashMap::new(), vec![VNode::text("Item 1")])
        ]);
        
        let new = VNode::element("ul", HashMap::new(), vec![
            VNode::element("li", HashMap::new(), vec![VNode::text("Item 1")]),
            VNode::element("li", HashMap::new(), vec![VNode::text("Item 2")])
        ]);
        
        let result = VirtualDom::diff(&old, &new);
        assert!(result.patches.iter().any(|p| matches!(p, PatchOp::Insert { .. })));
    }
}
