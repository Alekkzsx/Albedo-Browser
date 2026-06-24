use crate::ace::engine::AceEngine;

// pub mod collection;

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
    pub resource_rx:
        Option<tokio::sync::mpsc::UnboundedReceiver<crate::network::resources::ResourceResponse>>,
    pub is_loading: bool,
    pub loading_progress: f32,
    pub favicon_data: Option<slint::SharedPixelBuffer<slint::Rgba8Pixel>>,
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
            is_loading: false,
            loading_progress: 0.0,
            favicon_data: None,
        }
    }

    pub fn load_url(&mut self, url: String) {
        tracing::info!(url = %url, "Loading URL");
        self.url = url.clone();
        self.is_loading = true;
        self.loading_progress = 0.1; // Iniciou requisição
        self.engine.load_url(url);
    }
}
