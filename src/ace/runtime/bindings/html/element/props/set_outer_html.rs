use super::*;
use super::{mark_mutation, Element};
use crate::ace::engine::dom::{AceDOM, AceNode, AceNodeType};
use rquickjs::{Class, Ctx, Result, Value};



/// TODO: add docs
pub fn set_outer_html(el: &Element, html: String) {
    // outerHTML setter is more complex: it replaces the element itself.
    // Spec: "On setting, the element will be replaced by a fragment which is created by parsing the given string."
    if let Ok(mut dom) = el.dom.lock() {
        let parent_idx = dom.get_node(el.index).and_then(|n| n.parent);
        if let Some(p_idx) = parent_idx {
            let ref_idx = dom.get_node(el.index).and_then(|n| n.next_sibling);

            // Remove old node
            dom.remove_node_from_parent(el.index);

            // Insert new nodes from fragment
            for child_idx in dom.import_html_fragment(&html, Some(p_idx)) {
                dom.insert_before(p_idx, child_idx, ref_idx);
            }
        }
    }
    mark_mutation(el);
}
