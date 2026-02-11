use std::rc::Rc;
use std::cell::RefCell;
use wry::{WebView, WebViewBuilder};
use raw_window_handle::HasWindowHandle;
// use slint::ComponentHandle;
use uuid::Uuid;

pub struct Tab {
    pub id: Uuid,
    pub title: String,
    pub url: String,
    pub webview: Option<WebView>,
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
            webview: None, // Initialized later
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

    pub fn create_tab(&self, window: &slint::Window, url: &str) {
        let mut tabs = self.tabs.borrow_mut();
        let mut new_tab = Tab::new(url.to_string());
        
        // Initialize WebView only if not start page
        if !new_tab.show_start_page {
             #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
            {
                let handle = window.window_handle();
                if let Ok(_) = handle.window_handle() {
                    let ui_handle_load = self.ui_handle.clone();
                    let ui_handle_title = self.ui_handle.clone();
                    let tab_id = new_tab.id.to_string();
                    let tab_id_clone = tab_id.clone();
                    
                    let result = WebViewBuilder::new()
                        .with_url(url)
                        .with_visible(false) // Start hidden
                        .with_on_page_load_handler(move |_event, url| {
                            let url_clone = url.clone();
                            let id_clone = tab_id.clone();
                            let _ = ui_handle_load.upgrade_in_event_loop(move |ui| {
                                ui.invoke_url_updated(id_clone.into(), url_clone.into());
                            });
                        })
                        .with_document_title_changed_handler(move |title| {
                            let title_clone = title.clone();
                            let id_clone = tab_id_clone.clone();
                             let _ = ui_handle_title.upgrade_in_event_loop(move |ui| {
                                ui.invoke_title_updated(id_clone.into(), title_clone.into());
                            });
                        })
                        .build(&handle);
    
                    match result {
                        Ok(wv) => {
                            println!("Tab WebView created successfully!");
                            new_tab.webview = Some(wv);
                        },
                        Err(e) => println!("Failed to create Tab WebView: {:?}", e),
                    }
                }
            }
        }

        tabs.push(new_tab);
        
        // If this is the only tab, make it active
        if tabs.len() == 1 {
            drop(tabs); // Release borrow
            self.switch_to_tab(0);
        }
    }

    pub fn switch_to_tab(&self, index: usize) -> Option<(String, bool)> {
        let mut tabs = self.tabs.borrow_mut();
        if index >= tabs.len() { return None; }

        // Hide current active tab
        if let Some(current_idx) = *self.active_tab_index.borrow() {
            if let Some(tab) = tabs.get(current_idx) {
                if let Some(wv) = &tab.webview {
                    let _ = wv.set_visible(false);
                }
            }
        }

        // Show new tab
        let mut result = None;
        if let Some(tab) = tabs.get_mut(index) {
            *self.active_tab_index.borrow_mut() = Some(index);
            
            if !tab.show_start_page {
                 if let Some(wv) = &tab.webview {
                    let _ = wv.set_visible(true);
                    let _ = wv.focus();
                }
            }
            
            result = Some((tab.url.clone(), tab.show_start_page));
        }
        
        result
    }

    pub fn close_tab(&self, index: usize) {
        let mut tabs = self.tabs.borrow_mut();
        if index >= tabs.len() { return; }

        // Remove the tab (WebView is dropped automatically)
        tabs.remove(index);

        // Update active index
        let mut active = self.active_tab_index.borrow_mut();
        if let Some(curr) = *active {
            if tabs.is_empty() {
                *active = None;
            } else if index <= curr {
                // If we closed a tab before or at the current link, adjust logic needed
                // For MVP, just switch to the last available tab or 0
                let new_index = 0.min(tabs.len().saturating_sub(1));
                drop(active); // Release borrow to call switch
                drop(tabs); 
                self.switch_to_tab(new_index);
                return;
            }
        }
    }

    // Proxy methods for the active tab
    pub fn navigate(&self, window: &slint::Window, url: &str) {
        let mut tabs = self.tabs.borrow_mut(); // Need mutable to update state/create webview
        if let Some(idx) = *self.active_tab_index.borrow() {
            if let Some(tab) = tabs.get_mut(idx) {
                let final_url = if url.starts_with("http") { url.to_string() } else { format!("https://{}", url) };
                
                tab.url = final_url.clone();
                tab.show_start_page = false;
                tab.title = final_url.clone(); // Temp title update

                // If WebView doesn't exist yet (was Start Page), create it
                if tab.webview.is_none() {
                     #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
                    {
                        let handle = window.window_handle();
                         if let Ok(_) = handle.window_handle() {
                            let ui_handle_load = self.ui_handle.clone();
                            let ui_handle_title = self.ui_handle.clone();
                            let tab_id = tab.id.to_string();
                            let tab_id_clone = tab_id.clone();
                             
                            let result = WebViewBuilder::new()
                                .with_url(&final_url)
                                .with_visible(true)
                                .with_on_page_load_handler(move |_event, url| {
                                    let url_clone = url.clone();
                                    let id_clone = tab_id.clone();
                                    let _ = ui_handle_load.upgrade_in_event_loop(move |ui| {
                                        ui.invoke_url_updated(id_clone.into(), url_clone.into());
                                    });
                                })
                                .with_document_title_changed_handler(move |title| {
                                    let title_clone = title.clone();
                                    let id_clone = tab_id_clone.clone();
                                     let _ = ui_handle_title.upgrade_in_event_loop(move |ui| {
                                        ui.invoke_title_updated(id_clone.into(), title_clone.into());
                                    });
                                })
                                .build(&handle);
                            
                            if let Ok(wv) = result {
                                tab.webview = Some(wv);
                            }
                         }
                    }
                } else if let Some(wv) = &tab.webview {
                    let _ = wv.load_url(&final_url);
                    let _ = wv.set_visible(true);
                }
            }
        }
    }

    pub fn resize(&self, window: &slint::Window, top_offset: i32) {
        let tabs = self.tabs.borrow();
        if let Some(idx) = *self.active_tab_index.borrow() {
            if let Some(tab) = tabs.get(idx) {
                if let Some(wv) = &tab.webview {
                    let size = window.size();
                    let scale = window.scale_factor();
                    
                    let window_width = size.width;
                    let window_height = size.height;
                    let webview_height = (window_height as i32) - top_offset;
                    
                    if webview_height > 0 {
                        let _ = wv.set_bounds(wry::Rect {
                            position: wry::dpi::PhysicalPosition::new(0, top_offset).into(),
                            size: wry::dpi::PhysicalSize::new(window_width, webview_height as u32).into(),
                        });
                    }
                }
            }
        }
    }
    
    pub fn set_visible(&self, visible: bool) {
         let tabs = self.tabs.borrow();
         if let Some(idx) = *self.active_tab_index.borrow() {
             if let Some(tab) = tabs.get(idx) {
                 if let Some(wv) = &tab.webview {
                     let _ = wv.set_visible(visible);
                 }
             }
         }
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
            // Return true if this is the active tab
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
