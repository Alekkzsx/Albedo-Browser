use uuid::Uuid;
use crate::engine::AceEngine;
use crate::js::JsRuntime;
use crate::js::console::Console;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabMode {
    Native, // Everything is ACE now
}

pub struct Tab {
    pub id: Uuid,
    pub title: String,
    pub url: String,
    pub engine: AceEngine, // Always present
    pub js_runtime: Option<JsRuntime>,
    pub mode: TabMode,
    pub is_active: bool,
    pub show_start_page: bool,
}

impl Tab {
    pub fn new(url: String) -> Self {
        let show_start_page = url == "albedo://start";
        let title = if show_start_page { "New Tab".to_string() } else { "Loading...".to_string() };
        
        Self {
            id: Uuid::new_v4(),
            title,
            url,
            engine: AceEngine::new(),
            js_runtime: {
                match JsRuntime::new() {
                    Ok(rt) => {
                        if let Err(e) = Console::register(&rt) {
                            eprintln!("Failed to register console: {}", e);
                        }
                        if let Err(e) = rt.register_events() {
                            eprintln!("Failed to register events: {}", e);
                        }
                        Some(rt)
                    },
                    Err(e) => {
                        eprintln!("Failed to initialize JS runtime: {}", e);
                        None
                    }
                }
            },
            mode: TabMode::Native,
            is_active: false,
            show_start_page,
        }
    }
}
