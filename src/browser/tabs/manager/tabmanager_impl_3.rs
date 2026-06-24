use super::*;
use super::collection::TabCollection;
use super::tab::TabMode;
use crate::ace::engine::AceEngine;
use crate::network::resources::ResourceManager;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::mpsc;



impl TabManager {
    /// TODO: add docs
    pub fn dispatch_click_to_active_tab(&self, node_idx: usize) -> bool {
        self.with_collection(|col| {
            if let Some(tab) = col.get_active() {
                if let Some(ref rt) = tab.engine.js_runtime {
                    if let Some(ref dom) = tab.engine.dom {
                        tracing::debug!(node_idx, "Dispatching click to node");
                        rt.dispatch_event(dom.clone(), node_idx, "click");
                        return true;
                    }
                }
            }
            false
        })
    }

    /// TODO: add docs
    pub fn handle_click(&self, x: f32, y: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            let node_id = tab.engine.find_element_at_position(x, y);
            if let Some(idx) = node_id {
                // Comportamento nativo: toggle de <details> via <summary>
                if let Some(ref dom_arc) = tab.engine.dom {
                    let mut dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());

                    let is_summary = dom
                        .get_element(idx)
                        .map_or(false, |el| el.tag == "summary");

                    if is_summary {
                        let parent_idx = dom.get_node(idx).and_then(|n| n.parent);
                        if let Some(p_idx) = parent_idx {
                            let is_details = dom
                                .get_element(p_idx)
                                .map_or(false, |el| el.tag == "details");

                            if is_details {
                                let has_open = dom
                                    .get_element(p_idx)
                                    .map_or(false, |el| el.attributes.contains_key("open"));

                                if has_open {
                                    dom.remove_attribute_notify(p_idx, "open".into());
                                } else {
                                    dom.set_attribute_notify(p_idx, "open".into(), String::new());
                                }
                            }
                        }
                    }

                    // Comportamento nativo: <form method="dialog"> fecha <dialog> pai
                    let clicked_tag = dom
                        .get_element(idx)
                        .map_or_else(String::new, |el| el.tag.clone());

                    if clicked_tag == "button" || clicked_tag == "input" {
                        let button_value = dom
                            .get_element(idx)
                            .and_then(|el| el.attributes.get("value").cloned())
                            .unwrap_or_default();

                        let mut ancestor = dom.get_node(idx).and_then(|n| n.parent);
                        let mut found_form_dialog = false;
                        while let Some(a_idx) = ancestor {
                            if let Some(a_el) = dom.get_element(a_idx) {
                                if a_el.tag == "form"
                                    && a_el
                                        .attributes
                                        .get("method")
                                        .map_or(false, |m| m.eq_ignore_ascii_case("dialog"))
                                {
                                    found_form_dialog = true;
                                }
                                if found_form_dialog
                                    && a_el.tag == "dialog"
                                    && a_el.attributes.contains_key("open")
                                {
                                    if !button_value.is_empty() {
                                        dom.set_attribute_notify(
                                            a_idx,
                                            "data-return-value".into(),
                                            button_value.clone(),
                                        );
                                    }
                                    dom.remove_attribute_notify(a_idx, "open".into());
                                    dom.remove_attribute_notify(a_idx, "data-ace-modal".into());
                                    break;
                                }
                                ancestor = dom.get_node(a_idx).and_then(|n| n.parent);
                            } else {
                                break;
                            }
                        }
                    }
                }

                // Dispatch JS click event
                if let Some(ref rt) = tab.engine.js_runtime {
                    if let Some(ref dom) = tab.engine.dom {
                        tracing::debug!(x, y, node_idx = idx, "Click at position");
                        rt.dispatch_event(dom.clone(), idx, "click");
                        return true;
                    }
                }
            }
        }
        false
    }
}
