use crate::engine::AceEngine;

pub mod collection;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabMode {
    Native, // Everything is ACE now
}

pub struct Tab {
    pub id: String,
    pub title: String,
    pub url: String,
    pub engine: AceEngine, // Garanta que isso usa a struct nova
    pub mode: TabMode,
    pub is_active: bool,
    pub show_start_page: bool,
}

impl Tab {
    pub fn new(id: String, url: String) -> Self {
        Self {
            id,
            title: "Nova Aba".into(),
            url,
            engine: AceEngine::new(), // Inicia motor simples
            mode: TabMode::Native,
            is_active: false,
            show_start_page: false,
        }
    }

    pub fn load_url(&mut self, url: String) {
        println!("[Tab] Loading URL: {}", url);
        self.url = url.clone();
        self.engine.load_url(&url);
    }
}
