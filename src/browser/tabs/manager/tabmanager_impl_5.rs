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
    pub fn handle_pointer_up(&self, x: f32, y: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            // On mouse up, we generally clear active state of the current active element
            if tab.engine.active_element.is_some() {
                let old_active = tab.engine.active_element;
                tab.engine.set_active(None);

                // Dispatch mouseup. Usually to the element under cursor OR the active element.
                // For CSS :active, it clears when mouse is released.
                if let Some(idx) = old_active {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        rt.dispatch_touch_event(idx, "touchend", x, y, 1);
                        rt.dispatch_pointer_event(idx, "pointerup", x, y, 0, 1, "mouse", true);
                        if let Some(ref dom) = tab.engine.dom {
                            rt.dispatch_event(dom.clone(), idx, "mouseup");
                        }
                    }
                }
                return true;
            }
        }
        false
    }

    /// TODO: add docs
    pub fn handle_pointer_cancel(&self, x: f32, y: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            // On interaction cancel (e.g., system alert, drag cancelled)
            if tab.engine.active_element.is_some() {
                let old_active = tab.engine.active_element;
                tab.engine.set_active(None);

                // Dispatch pointercancel. Only pointercancel is fired, no mouse counterpart.
                if let Some(idx) = old_active {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        rt.dispatch_touch_event(idx, "touchcancel", x, y, 1);
                        rt.dispatch_pointer_event(idx, "pointercancel", x, y, 0, 1, "mouse", true);
                    }
                }
                return true;
            }
        }
        false
    }

    /// TODO: add docs
    pub fn handle_key_down(
        &self,
        key: &str,
        code: &str,
        ctrl: bool,
        shift: bool,
        alt: bool,
        meta: bool,
    ) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            // Comportamento nativo: ESC fecha dialog modal
            if key == "Escape" {
                if let Some(ref dom_arc) = tab.engine.dom {
                    let mut dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());
                    let mut modal_idx = None;
                    for (i, node) in dom.nodes.iter().enumerate() {
                        if let crate::ace::engine::dom::AceNodeType::Element(el) = &node.node_type {
                            if el.tag == "dialog"
                                && el.attributes.contains_key("open")
                                && el.attributes.contains_key("data-ace-modal")
                            {
                                modal_idx = Some(i);
                            }
                        }
                    }

                    if let Some(idx) = modal_idx {
                        // Disparar evento "cancel" (cancelável pela spec, mas simplificado aqui)
                        // Fechar o dialog
                        dom.remove_attribute_notify(idx, "open".into());
                        dom.remove_attribute_notify(idx, "data-ace-modal".into());

                        // Drop dom lock before trying to mutate tab
                        drop(dom);

                        // Cancelar pointers ativos também
                        if let Some(idx_active) = tab.engine.active_element {
                            if let Some(ref rt) = tab.engine.js_runtime {
                                rt.dispatch_touch_event(idx_active, "touchcancel", 0.0, 0.0, 1);
                                rt.dispatch_pointer_event(
                                    idx_active,
                                    "pointercancel",
                                    0.0,
                                    0.0,
                                    0,
                                    1,
                                    "mouse",
                                    true,
                                );
                            }
                            tab.engine.set_active(None);
                        }

                        return true;
                    }
                }
            }

            // Dispatch JS keydown event
            if let Some(ref rt) = tab.engine.js_runtime {
                if let Some(focused_idx) = tab.engine.focused_element {
                    rt.dispatch_keyboard_event(
                        focused_idx,
                        "keydown",
                        key,
                        code,
                        ctrl,
                        shift,
                        alt,
                        meta,
                    );
                    return true;
                }
            }
        }
        false
    }
}
