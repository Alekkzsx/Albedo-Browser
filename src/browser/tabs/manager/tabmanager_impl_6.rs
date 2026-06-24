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
    pub fn handle_key_up(
        &self,
        key: &str,
        code: &str,
        ctrl: bool,
        shift: bool,
        alt: bool,
        meta: bool,
    ) -> bool {
        self.with_collection(|col| {
            if let Some(tab) = col.get_active() {
                if let Some(ref rt) = tab.engine.js_runtime {
                    if let Some(focused_idx) = tab.engine.focused_element {
                        rt.dispatch_keyboard_event(
                            focused_idx,
                            "keyup",
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
        })
    }

    /// TODO: add docs
    pub fn handle_scroll(&self, x: f32, y: f32, delta: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            // Verificar elemento sob o cursor
            let node_id = tab.engine.find_element_at_position(x, y);

            if let Some(idx) = node_id {
                if let Some(ref dom_arc) = tab.engine.dom {
                    let dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());
                    let geometry = tab.engine.element_geometry.lock().unwrap_or_else(|e| e.into_inner());
                    let mut scroll_map = tab.engine.element_scroll.lock().unwrap_or_else(|e| e.into_inner());

                    let mut current = Some(idx);
                    while let Some(current_idx) = current {
                        if let Some(geom) = geometry.get(&current_idx) {
                            if (geom.overflow_y
                                == crate::ace::engine::style::css_values::CssOverflow::Scroll
                                || geom.overflow_y
                                    == crate::ace::engine::style::css_values::CssOverflow::Auto)
                                && geom.content_height > geom.height
                            {
                                // Encontramos um container scrollável internamente
                                let current_scroll =
                                    scroll_map.get(&current_idx).copied().unwrap_or((0.0, 0.0));

                                // delta > 0 (scroll up) diminui o offset Y; delta < 0 (scroll down) aumenta o offset Y
                                let mut new_sy = current_scroll.1 - delta;
                                let max_scroll = geom.content_height - geom.height;

                                new_sy = new_sy.clamp(0.0, max_scroll);
                                scroll_map.insert(current_idx, (current_scroll.0, new_sy));

                                // Marcar para redesenho
                                drop(scroll_map);
                                drop(geometry);
                                drop(dom);
                                return true;
                            }
                        }
                        current = dom.get_node(current_idx).and_then(|n| n.parent);
                    }
                }
            }

            // Fallback: scroll do viewport principal
            let new_y = tab.engine.viewport_y + delta;
            tab.engine.viewport_y = new_y.min(0.0);
            return true;
        }
        false
    }

    /// TODO: add docs
    pub fn process_animations(&self) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            if tab.engine.tick(now) {
                return true;
            }
        }
        false
    }

    /// TODO: add docs
    pub fn request_navigate(&self, url: String) {
        self.with_collection_mut(|col| col.pending_nav = Some(url));
    }
}
