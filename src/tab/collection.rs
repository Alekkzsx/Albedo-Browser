use crate::tab::Tab;
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

    pub fn add(&mut self, url: String) -> String {
        let id = Uuid::new_v4().to_string();
        let new_tab = Tab::new(id.clone(), url);
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
}
