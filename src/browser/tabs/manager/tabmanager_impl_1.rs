use super::*;
use super::collection::TabCollection;
use super::tab::TabMode;
use crate::ace::engine::AceEngine;
use crate::network::resources::ResourceManager;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::mpsc;



impl TabManager {
    /// TODO: add docs
    pub fn new() -> Self {
        Self {
            collection: Rc::new(RefCell::new(TabCollection::new())),
        }
    }

pub(crate) fn with_collection<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&TabCollection) -> R,
    {
        f(&self.collection.borrow())
    }

pub(crate) fn with_collection_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut TabCollection) -> R,
    {
        f(&mut *self.collection.borrow_mut())
    }

    /// TODO: add docs
    pub fn create_tab(&self, _window: &slint::Window, url: &str) {
        tracing::info!(url = %url, "Creating new tab");

        // Criar canal para esta aba
        let (tx, rx) = mpsc::unbounded_channel();
        let rm = ResourceManager::new(tx);

        let id = self.collection.borrow_mut().add(url.to_string());

        {
            let mut col = self.collection.borrow_mut();
            if let Some(pos) = col.tabs.iter().position(|t| t.id == id) {
                let tab = &mut col.tabs[pos];
                tab.resource_rx = Some(rx);
                tab.engine.set_resource_manager(rm);
            }
        }

        self.load_url(id, url.to_string());
    }

    /// TODO: add docs
    pub fn load_url(&self, tab_id: String, url: String) {
        tracing::info!(url = %url, "Loading URL");
        let mut col = self.collection.borrow_mut();
        if let Some(pos) = col.tabs.iter().position(|t| t.id == tab_id) {
            col.tabs[pos].load_url(url.clone());
            col.tabs[pos].favicon_data = None; // Reset favicon

            // Requisita o favicon hardcoded pelo Google Service
            if let Ok(parsed) = crate::ace::url::parse(&url, None) {
                if let Some(host) = parsed.host_str() {
                    let favicon_url =
                        format!("https://www.google.com/s2/favicons?domain={}&sz=64", host);
                    if let Some(rm) = col.tabs[pos].engine.resource_manager.as_ref() {
                        rm.fetch(
                            favicon_url,
                            crate::network::resources::ResourceType::Image,
                            None,
                        );
                    }
                }
            }
        }
    }

    /// TODO: add docs
    pub fn navigate(
        &self,
        _window: &slint::Window,
        url: &str,
    ) -> Option<(String, bool, bool, String)> {
        let tab_id = self.with_collection(|col| col.get_active().map(|t| t.id.clone()));

        if let Some(id) = tab_id {
            self.load_url(id, url.to_string());

            return self.with_collection(|col| {
                if col.get_active().is_some() {
                    Some((url.to_string(), false, false, "EFFICIENT".into()))
                } else {
                    None
                }
            });
        }
        None
    }

    /// TODO: add docs
    pub fn get_active_tab_native_data(&self) -> Option<(String, AceEngine, f32)> {
        self.with_collection(|col| {
            col.get_active().map(|tab| {
                (
                    tab.url.clone(),
                    tab.engine.clone(),
                    tab.loading_progress,
                )
            })
        })
    }

    /// TODO: add docs
    pub fn get_tabs_info(&self) -> Vec<(String, bool, bool, slint::Image)> {
        self.with_collection(|col| {
            let active_idx = col.active_index;
            let default_image = slint::Image::default();
            col.tabs
                .iter()
                .enumerate()
                .map(|(i, tab)| {
                    let img = match &tab.favicon_data {
                        Some(buf) => slint::Image::from_rgba8(buf.clone()),
                        None => default_image.clone(),
                    };
                    (
                        tab.title.clone(),
                        Some(i) == active_idx,
                        tab.is_loading,
                        img,
                    )
                })
                .collect()
        })
    }
}
