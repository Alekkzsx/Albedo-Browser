use std::rc::Rc;
use std::cell::RefCell;
use uuid::Uuid;
use crate::engine::AceEngine;
use crate::js::JsRuntime;

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
        let id = self.collection.borrow_mut().add(url.to_string());
        
        // Properly load the initial content
        self.load_url(id, url.to_string());
    }

    pub fn switch_to_tab(&self, index: usize) -> Option<(String, bool, String, TabMode)> {
        println!("[TabManager] Switching to tab index: {}", index);
        let mut col = self.collection.borrow_mut();
        
        if let Some(tab) = col.switch_to(index) {
             println!("[TabManager] Tab switched successfully to: {}", tab.url);
             let native_content = tab.engine.render();
             return Some((tab.url.clone(), tab.show_start_page, native_content, tab.mode));
        }
        println!("[TabManager] Error: Index out of bounds!");
        None
    }

    pub fn close_tab(&self, index: usize) {
        println!("[TabManager] Closing tab index: {}", index);
        self.collection.borrow_mut().close(index);
    }

    pub fn load_url(&self, tab_id: Uuid, url: String) {
        println!("[TabManager] Loading URL: {}", url);
        let mut col = self.collection.borrow_mut();
        if let Some(pos) = col.tabs.iter().position(|t| t.id == tab_id) {
            col.tabs[pos].load_url(url);
        }
    }

    pub fn navigate(&self, _window: &slint::Window, url: &str) -> Option<(String, bool, String, TabMode)> {
        let tab_id = {
             let col = self.collection.borrow();
             col.get_active().map(|t| t.id)
        };

        if let Some(id) = tab_id {
            self.load_url(id, url.to_string());

            let col = self.collection.borrow();
            if let Some(tab) = col.get_active() {
                let rendered = tab.engine.render();
                return Some((tab.url.clone(), tab.show_start_page, rendered, tab.mode));
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

    pub fn update_tab_url(&self, id_str: &str, url: String) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(pos) = col.tabs.iter().position(|t| t.id.to_string() == id_str) {
            col.tabs[pos].url = url;
             return Some(pos) == col.active_index;
        }
        false
    }

    pub fn update_tab_title(&self, id_str: &str, title: String) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(pos) = col.tabs.iter().position(|t| t.id.to_string() == id_str) {
            col.tabs[pos].title = title;
            return true; 
        }
        false
    }

    pub fn pulse(&self) -> bool {
        let (mut needs_redraw, nav_req) = self.collection.borrow_mut().pulse();

        if let Some((id, url)) = nav_req {
             println!("JS requested navigation to: {}", url);
             self.load_url(id, url);
             needs_redraw = true;
        }
        needs_redraw
    }

    pub fn dispatch_click_to_active_tab(&self, ptr: usize) -> bool {
        self.collection.borrow_mut().dispatch_click(ptr)
    }
}

