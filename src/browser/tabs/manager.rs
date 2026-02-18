use std::rc::Rc;
use std::cell::RefCell;
use crate::engine::AceEngine;
use super::tab::TabMode;
use super::collection::TabCollection;
use crate::network::resources::ResourceManager;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct TabManager {
    collection: Rc<RefCell<TabCollection>>,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            collection: Rc::new(RefCell::new(TabCollection::new())),
        }
    }

    pub fn create_tab(&self, _window: &slint::Window, url: &str) {
        println!("[TabManager] Creating new tab for URL: {}", url);
        
        // Criar canal para esta aba
        let (tx, rx) = mpsc::unbounded_channel();
        let rm = ResourceManager::new(tx);
        
        let id = self.collection.borrow_mut().add(url.to_string());
        
        {
            let mut col = self.collection.borrow_mut();
            if let Some(pos) = col.tabs.iter().position(|t| t.id == id) {
                let tab = &mut col.tabs[pos];
                tab.resource_rx = Some(rx);
                tab.engine.set_resource_manager(rm);
            }
        }
        
        self.load_url(id, url.to_string());
    }

    pub fn load_url(&self, tab_id: String, url: String) {
        println!("[TabManager] Loading URL: {}", url);
        let mut col = self.collection.borrow_mut();
        if let Some(pos) = col.tabs.iter().position(|t| t.id == tab_id) {
            col.tabs[pos].load_url(url);
        }
    }

    pub fn navigate(&self, _window: &slint::Window, url: &str) -> Option<(String, bool, bool, String)> {
        let tab_id = {
             let col = self.collection.borrow();
             col.get_active().map(|t| t.id.clone())
        };

        if let Some(id) = tab_id {
            self.load_url(id, url.to_string()); 

            let col = self.collection.borrow();
            if let Some(_tab) = col.get_active() {
                return Some((url.to_string(), false, false, "EFFICIENT".into())); 
            }
        }
        None
    }

    pub fn get_active_tab_native_data(&self) -> Option<(String, Option<AceEngine>)> {
        let col = self.collection.borrow();
        if let Some(tab) = col.get_active() {
             return Some((tab.url.clone(), Some(tab.engine.clone())));
        }
        None
    }

    pub fn get_tabs_info(&self) -> Vec<(String, bool)> {
        let col = self.collection.borrow();
        let active_idx = col.active_index;
        
        col.tabs.iter().enumerate().map(|(i, tab)| {
            (tab.title.clone(), Some(i) == active_idx)
        }).collect()
    }

    pub fn switch_to_tab(&self, index: usize) -> Option<(String, bool, String, TabMode)> {
        println!("[TabManager] Switching to tab index: {}", index);
        let mut col = self.collection.borrow_mut();
        
        if let Some(tab) = col.switch_to(index) {
             println!("[TabManager] Tab switched successfully to: {}", tab.url);
             return Some((tab.url.clone(), tab.show_start_page, "".to_string(), tab.mode));
        }
        None
    }

    pub fn close_tab(&self, index: usize) {
        self.collection.borrow_mut().close(index);
    }

    pub fn process_active_tab_resources(&self) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            // Se tivermos um receiver, tentar ler mensagens sem bloquear
            if let Some(mut rx) = tab.resource_rx.take() {
                let mut did_update = false;
                
                // Ler até o canal estar vazio ou limite de mensagens
                let mut count = 0;
                while let Ok(response) = rx.try_recv() {
                    if tab.engine.handle_resource_response(response) {
                        did_update = true;
                    }
                    count += 1;
                    if count > 50 { break; } // Limite por frame
                }
                
                tab.resource_rx = Some(rx);
                return did_update;
            }
        }
        false
    }

    pub fn dispatch_click_to_active_tab(&self, node_idx: usize) -> bool {
        let col = self.collection.borrow();
        if let Some(tab) = col.get_active() {
            if let Some(ref rt) = tab.engine.js_runtime {
                if let Some(ref dom) = tab.engine.dom {
                    println!("[TabManager] Dispatching click to node index: {}", node_idx);
                    rt.dispatch_event(dom.clone(), node_idx, "click");
                    return true;
                }
            }
        }
        false
    }

    pub fn handle_click(&self, x: f32, y: f32) -> bool {
        let col = self.collection.borrow();
        if let Some(tab) = col.get_active() {
            let node_id = tab.engine.find_element_at_position(x, y);
            if let Some(idx) = node_id {
                if let Some(ref rt) = tab.engine.js_runtime {
                    if let Some(ref dom) = tab.engine.dom {
                        println!("[TabManager] Click at ({}, {}) -> Node {}", x, y, idx);
                        rt.dispatch_event(dom.clone(), idx, "click");
                        return true;
                    }
                }
            }
        }
        false
    }

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
                             rt.dispatch_event(dom.clone(), old_idx, "mouseout");
                         }
                     }
                 }
                 
                 // Disparar mouseover no novo elemento
                 if let Some(new_idx) = node_id {
                     if let Some(ref rt) = tab.engine.js_runtime {
                         if let Some(ref dom) = tab.engine.dom {
                             rt.dispatch_event(dom.clone(), new_idx, "mouseover");
                         }
                     }
                 }
                 
                 tab.engine.set_hover(node_id);
                 return true;
             }
        }
        false
    }

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

    pub fn handle_key_down(&self, key: &str, code: &str, ctrl: bool, shift: bool, alt: bool, meta: bool) -> bool {
        let col = self.collection.borrow();
        if let Some(tab) = col.get_active() {
            if let Some(ref rt) = tab.engine.js_runtime {
                if let Some(focused_idx) = tab.engine.focused_element {
                    rt.dispatch_keyboard_event(focused_idx, "keydown", key, code, ctrl, shift, alt, meta);
                    return true;
                }
            }
        }
        false
    }

    pub fn handle_key_up(&self, key: &str, code: &str, ctrl: bool, shift: bool, alt: bool, meta: bool) -> bool {
        let col = self.collection.borrow();
        if let Some(tab) = col.get_active() {
            if let Some(ref rt) = tab.engine.js_runtime {
                if let Some(focused_idx) = tab.engine.focused_element {
                    rt.dispatch_keyboard_event(focused_idx, "keyup", key, code, ctrl, shift, alt, meta);
                    return true;
                }
            }
        }
        false
    }

    pub fn handle_scroll(&self, _x: f32, _y: f32, delta: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            // No Slint, viewport-y costuma ser negativo para scroll down
            // delta vindo do mouse wheel (positiva para cima, negativa para baixo)
            // Se delta > 0 (scroll up), queremos incrementar viewport_y (em direção a 0)
            // Se delta < 0 (scroll down), queremos decrementar viewport_y (mais negativo)
            
            let new_y = tab.engine.viewport_y + delta;
            
            // Limit scroll (0 to -content_height + window_height)
            // Para simplificar agora, vamos apenas impedir que suba acima de 0
            tab.engine.viewport_y = new_y.min(0.0);
            return true;
        }
        false
    }

    pub fn process_animations(&self) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
             let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64();
             if tab.engine.tick(now) {
                 return true;
             }
        }
        false
    }

    pub fn request_navigate(&self, url: String) {
        let mut col = self.collection.borrow_mut();
        col.pending_nav = Some(url);
    }

    pub fn take_pending_nav(&self) -> Option<String> {
        let mut col = self.collection.borrow_mut();
        col.pending_nav.take()
    }
}
