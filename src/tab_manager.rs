use std::rc::Rc;
use std::cell::RefCell;
// use raw_window_handle::HasWindowHandle; // Unused
// use slint::ComponentHandle;
use uuid::Uuid;
use reqwest; // Added for fetching external URLs

use crate::engine::AceEngine;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabMode {
    Native, // Everything is ACE now
}

pub struct Tab {
    pub id: Uuid,
    pub title: String,
    pub url: String,
    pub engine: AceEngine, // Always present
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
            mode: TabMode::Native,
            is_active: false,
            show_start_page,
        }
    }
}

use crate::AppWindow;

#[derive(Clone)]
pub struct TabManager {
    tabs: Rc<RefCell<Vec<Tab>>>,
    active_tab_index: Rc<RefCell<Option<usize>>>,
    ui_handle: slint::Weak<AppWindow>,
}

impl TabManager {
    pub fn new(ui_handle: slint::Weak<AppWindow>) -> Self {
        Self {
            tabs: Rc::new(RefCell::new(Vec::new())),
            active_tab_index: Rc::new(RefCell::new(None)),
            ui_handle,
        }
    }

    pub fn create_tab(&self, _window: &slint::Window, url: &str) {
        let mut tabs = self.tabs.borrow_mut();
        let mut new_tab = Tab::new(url.to_string());
        
        tabs.push(new_tab);
        
        // If this is the only tab, make it active
        if tabs.len() == 1 {
            drop(tabs); // Release borrow
            self.switch_to_tab(0);
        }
    }

    pub fn switch_to_tab(&self, index: usize) -> Option<(String, bool, String, TabMode)> {
        let mut tabs = self.tabs.borrow_mut();
        if index >= tabs.len() { return None; }

        // Update active index
        *self.active_tab_index.borrow_mut() = Some(index);
        
        if let Some(tab) = tabs.get_mut(index) {
             let native_content = tab.engine.render();
             return Some((tab.url.clone(), tab.show_start_page, native_content, tab.mode));
        }
        None
    }

    pub fn close_tab(&self, index: usize) {
        let mut tabs = self.tabs.borrow_mut();
        if index >= tabs.len() { return; }

        tabs.remove(index);

        let mut active = self.active_tab_index.borrow_mut();
        if let Some(curr) = *active {
            if tabs.is_empty() {
                *active = None;
            } else if index <= curr {
                let new_index = 0.min(tabs.len().saturating_sub(1));
                drop(active);
                drop(tabs); 
                self.switch_to_tab(new_index);
                return;
            }
        }
    }

    pub fn navigate(&self, _window: &slint::Window, url: &str) -> Option<(String, bool, String, TabMode)> {
        let mut tabs = self.tabs.borrow_mut();
        if let Some(idx) = *self.active_tab_index.borrow() {
            if let Some(tab) = tabs.get_mut(idx) {
                tab.url = url.to_string();
                tab.show_start_page = false;

                let html_content = if url.starts_with("albedo://") {
                    tab.title = format!("Albedo - {}", &url[9..]);
                    match url {
                        "albedo://about" => "<h1>About Albedo</h1><p style='color: blue;'>The pure Rust browser.</p>".to_string(),
                        "albedo://engine" => "<h1>ACE v0.1</h1><p style='color: red;'>Running natively.</p>".to_string(),
                        "albedo://flex" => "<h1>Flexbox Demo</h1><div style='display: flex; flex-direction: row; justify-content: space-between;'><div style='background-color: red; width: 50px; height: 50px;'></div><div style='background-color: blue; width: 50px; height: 50px;'></div></div>".to_string(),
                         "albedo://images" => "<h1>Images Demo</h1><p>Displaying images via ACE:</p><img src='https://www.rust-lang.org/static/images/rust-logo-blk.svg' width='150' height='150'><p>Local image:</p><img src='assets/icon.png' width='50' height='50'>".to_string(),
                    "albedo://css" => "
                        <style>
                            .title { color: purple; font-size: 40px; }
                            #special { color: red; background-color: yellow; }
                            p { color: green; }
                            .box { display: flex; flex-direction: row; justify-content: space-around; background-color: #eee; }
                            .item { color: white; background-color: blue; font-size: 20px; }
                        </style>
                        <h1 class='title'>CSS Engine Demo</h1>
                        <p id='special'>This is a special ID-styled paragraph.</p>
                        <p>This is a normal paragraph styled by tag selector.</p>
                        <div class='box'>
                            <div class='item'>Item 1</div>
                            <div class='item'>Item 2</div>
                            <div class='item'>Item 3</div>
                        </div>
                    ".to_string(),
                        "albedo://links" => "<h1>Links Demo</h1>
                        <p>Click the link below:</p>
                        <a href='albedo://flex'>Go to Flexbox Demo</a>
                        <br/>
                        <div style='background-color: #eee; padding: 10px;'>
                            <a href='albedo://about'>Go to About</a>
                        </div>".to_string(),
                        "albedo://scroll" => "<h1>Scroll Demo</h1>
                        <p>This page should scroll.</p>
                        <div style='height: 200px; background-color: red;'>Item 1</div>
                        <div style='height: 200px; background-color: blue;'>Item 2</div>
                        <div style='height: 200px; background-color: green;'>Item 3</div>
                        <div style='height: 200px; background-color: yellow;'>Item 4</div>
                        <div style='height: 200px; background-color: purple;'>Item 5</div>
                        <p>End of page.</p>".to_string(),
                        _ => "<h1>404</h1><p>Internal page not found.</p>".to_string(),
                    }
                } else {
                    // External URL - Fetch via Reqwest
                     tab.title = url.to_string();
                     match reqwest::blocking::get(url) {
                        Ok(resp) => {
                            match resp.text() {
                                Ok(text) => text,
                                Err(e) => format!("<h1>Encoding Error</h1><p>{}</p>", e),
                            }
                        },
                        Err(e) => format!("<h1>Connection Error</h1><p>{}</p>", e),
                    }
                };

                tab.engine.load_html(&html_content);
                let rendered = tab.engine.render();
                return Some((tab.url.clone(), tab.show_start_page, rendered, tab.mode));
            }
        }
        None
    }

    pub fn resize(&self, _window: &slint::Window, _top_offset: i32) {}

    pub fn set_visible(&self, _visible: bool) {}

    pub fn get_active_tab_native_data(&self) -> Option<(String, Option<AceEngine>)> {
        let tabs = self.tabs.borrow();
        if let Some(idx) = *self.active_tab_index.borrow() {
            if let Some(tab) = tabs.get(idx) {
                return Some((tab.url.clone(), Some(tab.engine.clone())));
            }
        }
        None
    }

    pub fn get_tabs_info(&self) -> Vec<(String, bool)> {
        let tabs = self.tabs.borrow();
        let active_idx = *self.active_tab_index.borrow();
        
        tabs.iter().enumerate().map(|(i, tab)| {
            (tab.title.clone(), Some(i) == active_idx)
        }).collect()
    }

    pub fn update_tab_url(&self, id_str: &str, url: String) -> bool {
        let mut tabs = self.tabs.borrow_mut();
        if let Some(pos) = tabs.iter().position(|t| t.id.to_string() == id_str) {
            tabs[pos].url = url;
             let active_idx = *self.active_tab_index.borrow();
             return Some(pos) == active_idx;
        }
        false
    }

    pub fn update_tab_title(&self, id_str: &str, title: String) -> bool {
        let mut tabs = self.tabs.borrow_mut();
        if let Some(pos) = tabs.iter().position(|t| t.id.to_string() == id_str) {
            tabs[pos].title = title;
            return true; 
        }
        false
    }
}
