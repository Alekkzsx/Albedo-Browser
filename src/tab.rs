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
    pub engine: AceEngine,
    pub mode: TabMode,
    pub show_start_page: bool,
    pub resource_rx: Option<tokio::sync::mpsc::UnboundedReceiver<crate::services::resource_manager::ResourceResponse>>,
}

impl Tab {
    pub fn new(id: String, url: String) -> Self {
        Self {
            id,
            title: "Nova Aba".into(),
            url,
            engine: AceEngine::new(),
            mode: TabMode::Native,
            show_start_page: false,
            resource_rx: None,
        }
    }

    pub fn load_url(&mut self, url: String) {
        println!("[Tab] Loading URL: {}", url);
        self.url = url.clone();
        self.engine.load_url(&url);
    }
}
