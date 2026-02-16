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
            if let Some(mut rx) = tab.resource_rx.take() {
                let needs_sync = tab.engine.process_resource_responses(&mut rx);
                tab.resource_rx = Some(rx);
                return needs_sync;
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
}
