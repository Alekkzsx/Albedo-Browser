use uuid::Uuid;
use crate::engine::AceEngine;
use crate::js::JsRuntime;
use crate::js::console::Console;

pub mod collection;

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
    pub fn handle_pulse(&mut self) -> (bool, Option<String>) {
        let mut needs_redraw = false;
        let mut navigation_request = None;

        if let Some(rt) = &self.js_runtime {
            let (mutated, stylesheet_dirty) = rt.run_pending();
            if stylesheet_dirty {
                self.engine.update_stylesheet();
            }
            if let Some(url) = rt.get_pending_navigation() {
               navigation_request = Some(url);
            }
             if mutated || stylesheet_dirty {
                needs_redraw = true;
            }
        }
        (needs_redraw, navigation_request)
    }

    pub fn load_url(&mut self, url: String) {
        println!("[Tab] Loading URL: {}", url);
        self.url = url.clone();
        self.show_start_page = false;

        let html_content = if url.starts_with("albedo://") {
            if let Some((title, content)) = crate::engine::internal_pages::get_internal_page(&url) {
                println!("[Tab] Loading internal Albedo page: {}", title);
                self.title = title;
                content
            } else {
                 self.title = "404 Not Found".to_string();
                 "<h1>404</h1><p>Internal page not found.</p>".to_string()
            }
        } else {
            let (title, content) = crate::services::fetcher::fetch_url(&url);
            self.title = title;
            content
        };

        let mut engine = AceEngine::new();
        println!("[Tab] Handing HTML to AceEngine...");
        engine.load_html(&html_content);
        
        self.js_runtime = crate::js::init::init_js_for_url(&url, &engine);
        self.engine = engine;
        
        if let Some(dom) = &self.engine.dom {
            if let Some(el) = dom.root.select("title").ok().and_then(|mut i| i.next()) {
                self.title = el.text_contents();
            }
        }
    }

    pub fn dispatch_click(&mut self, ptr: usize) -> bool {
        if let Some(rt) = &self.js_runtime {
            // Safe to cast back to NodeRef because we know it came from there
            // In a real browser we'd use a unique ID map, but for now this is "safe enough" for a demo
            let node_ptr = ptr as *const kuchiki::NodeRef;
            let node = unsafe { &*node_ptr }.clone(); 
            rt.dispatch_event(node, "click");
            return true;
        }
        false
    }
}
