use std::rc::Rc;
use std::cell::RefCell;
use crate::engine::AceEngine;
use crate::tab::{Tab, TabMode};
use crate::AppWindow;
use crate::tab::collection::TabCollection;

#[derive(Clone)]
pub struct TabManager {
    collection: Rc<RefCell<TabCollection>>,
    ui_handle: slint::Weak<AppWindow>,
}

impl TabManager {
    pub fn new(ui_handle: slint::Weak<AppWindow>) -> Self {
        Self {
            collection: Rc::new(RefCell::new(TabCollection::new())),
            ui_handle,
        }
    }

    pub fn create_tab(&self, _window: &slint::Window, url: &str) {
        println!("[TabManager] Creating new tab for URL: {}", url);
        // collection.add returns String now
        let id = self.collection.borrow_mut().add(url.to_string());
        self.load_url(id, url.to_string());
    }

    pub fn load_url(&self, tab_id: String, url: String) {
        println!("[TabManager] Loading URL: {}", url);
        let mut col = self.collection.borrow_mut();
        // find tab by id (String)
        if let Some(pos) = col.tabs.iter().position(|t| t.id == tab_id) {
            col.tabs[pos].load_url(url);
        }
    }

    pub fn navigate(&self, _window: &slint::Window, url: &str) -> Option<(String, bool, bool, String)> {
        // ... lógica de pegar aba ...
        let tab_id = {
             let col = self.collection.borrow();
             col.get_active().map(|t| t.id.clone())
        };

        if let Some(id) = tab_id {
            // This is synchronous now
            self.load_url(id, url.to_string()); 

            let col = self.collection.borrow();
            if let Some(_tab) = col.get_active() {
                // Return what user requested. 
                // Note: user snippet had logic that returns immediately after update.
                // "return Some((url.to_string(), false, false, "EFFICIENT".into()));"
                // The third bool might be "is_loading" or similar? 
                // The fourth string is... "EFFICIENT". Use that.
                
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

    // Helper to get tabs info for the sidebar/topbar
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
             // Returning dummy content for now as rendering is handled via sync_ace_visuals pulling from engine
             return Some((tab.url.clone(), tab.show_start_page, "".to_string(), tab.mode));
        }
        None
    }

    pub fn close_tab(&self, index: usize) {
        self.collection.borrow_mut().close(index);
    }

    pub fn pulse(&self) -> bool {
        false
    }

    pub fn dispatch_click_to_active_tab(&self, _ptr: usize) -> bool {
        false
    }
}
