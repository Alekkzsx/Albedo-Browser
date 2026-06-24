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
    pub fn handle_hover(&self, x: f32, y: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            let node_id = tab.engine.find_element_at_position(x, y);
            let old_hover = tab.engine.hovered_element;

            if old_hover != node_id {
                // Disparar mouseout no elemento anterior
                if let Some(old_idx) = old_hover {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        if let Some(ref dom) = tab.engine.dom {
                            rt.dispatch_pointer_event(
                                old_idx,
                                "pointerout",
                                x,
                                y,
                                0,
                                1,
                                "mouse",
                                true,
                            );
                            rt.dispatch_event(dom.clone(), old_idx, "mouseout");
                        }
                    }
                }

                // Disparar mouseover no novo elemento
                if let Some(new_idx) = node_id {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        if let Some(ref dom) = tab.engine.dom {
                            rt.dispatch_pointer_event(
                                new_idx,
                                "pointerover",
                                x,
                                y,
                                0,
                                1,
                                "mouse",
                                true,
                            );
                            rt.dispatch_event(dom.clone(), new_idx, "mouseover");
                        }
                    }
                }

                tab.engine.set_hover(node_id);
                return true;
            } else {
                // Disparar mousemove/pointermove no elemento atual se a posição mudou e continua nele
                if let Some(idx) = node_id {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        rt.dispatch_touch_event(idx, "touchmove", x, y, 1);
                        rt.dispatch_pointer_event(idx, "pointermove", x, y, 0, 1, "mouse", true);
                        // rt.dispatch_event(dom.clone(), idx, "mousemove"); // mousemove pode ser implementado depois
                    }
                }
                return true;
            }
        }
        false
    }

    /// TODO: add docs
    pub fn handle_pointer_down(&self, x: f32, y: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            let node_id = tab.engine.find_element_at_position(x, y);
            // Set active only if clicked on something
            // Standard behavior: clicking on empty space usually clears active state of others?
            // Actually, active state is usually per-element. If I click empty active becomes None.
            if node_id != tab.engine.active_element {
                tab.engine.set_active(node_id);

                // Atualizar foco também quando clica em um elemento
                let old_focused = tab.engine.focused_element;
                tab.engine.set_focused(node_id);

                // Disparar eventos blur/focus se o foco mudou
                if old_focused != node_id {
                    if let Some(old_idx) = old_focused {
                        if let Some(ref rt) = tab.engine.js_runtime {
                            if let Some(ref dom) = tab.engine.dom {
                                rt.dispatch_event(dom.clone(), old_idx, "blur");
                            }
                        }
                    }
                    if let Some(new_idx) = node_id {
                        if let Some(ref rt) = tab.engine.js_runtime {
                            if let Some(ref dom) = tab.engine.dom {
                                rt.dispatch_event(dom.clone(), new_idx, "focus");
                            }
                        }
                    }
                }

                if let Some(idx) = node_id {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        rt.dispatch_touch_event(idx, "touchstart", x, y, 1);
                        rt.dispatch_pointer_event(idx, "pointerdown", x, y, 0, 1, "mouse", true);
                        if let Some(ref dom) = tab.engine.dom {
                            rt.dispatch_event(dom.clone(), idx, "mousedown");
                        }
                    }
                }
                return true;
            }
        }
        false
    }
}
