use crate::tab::{Tab, TabMode};
use uuid::Uuid;

pub struct TabCollection {
    pub tabs: Vec<Tab>,
    pub active_index: Option<usize>,
}

impl TabCollection {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_index: None,
        }
    }

    pub fn add(&mut self, url: String) -> Uuid {
        let new_tab = Tab::new(url);
        let id = new_tab.id;
        self.tabs.push(new_tab);
        if self.tabs.len() == 1 {
            self.active_index = Some(0);
        }
        id
    }

    pub fn get_active(&self) -> Option<&Tab> {
        if let Some(idx) = self.active_index {
            self.tabs.get(idx)
        } else {
            None
        }
    }

    pub fn get_active_mut(&mut self) -> Option<&mut Tab> {
        if let Some(idx) = self.active_index {
            self.tabs.get_mut(idx)
        } else {
            None
        }
    }

    pub fn switch_to(&mut self, index: usize) -> Option<&Tab> {
        if index < self.tabs.len() {
            self.active_index = Some(index);
            self.tabs.get(index)
        } else {
            None
        }
    }

    pub fn close(&mut self, index: usize) {
        if index >= self.tabs.len() { return; }
        self.tabs.remove(index);
        
        if let Some(curr) = self.active_index {
            if self.tabs.is_empty() {
                self.active_index = None;
            } else if index <= curr {
                let new_index = 0.min(self.tabs.len().saturating_sub(1));
                self.active_index = Some(new_index);
            }
        }
    }

    pub fn pulse(&mut self) -> (bool, Option<(Uuid, String)>) {
        if let Some(tab) = self.get_active_mut() {
             let (redraw, nav) = tab.handle_pulse();
             let nav_req = nav.map(|url| (tab.id, url));
             return (redraw, nav_req);
        }
        (false, None)
    }

    pub fn dispatch_click(&mut self, ptr: usize) -> bool {
        if let Some(tab) = self.get_active_mut() {
            return tab.dispatch_click(ptr);
        }
        false
    }
}
