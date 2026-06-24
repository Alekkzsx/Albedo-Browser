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
    pub fn switch_to_tab(&self, index: usize) -> Option<(String, bool, String, TabMode)> {
        tracing::debug!(index, "Switching to tab");
        self.with_collection_mut(|col| {
            if let Some(tab) = col.switch_to(index) {
                tracing::debug!(url = %tab.url, "Tab switched");
                return Some((
                    tab.url.clone(),
                    tab.show_start_page,
                    "".to_string(),
                    tab.mode,
                ));
            }
            None
        })
    }

    /// TODO: add docs
    pub fn close_tab(&self, index: usize) {
        self.with_collection_mut(|col| col.close(index));
    }

    /// TODO: add docs
    pub fn process_active_tab_resources(&self) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            // Se tivermos um receiver, tentar ler mensagens sem bloquear
            if let Some(mut rx) = tab.resource_rx.take() {
                let mut did_update = false;

                // Ler até o canal estar vazio ou limite de mensagens
                let mut count = 0;
                while let Ok(response) = rx.try_recv() {
                    let mut is_html = false;
                    let mut is_favicon = false;

                    if let crate::network::resources::ResourceType::Html = response.resource_type {
                        if response.url == tab.url || response.url == tab.engine.current_url {
                            is_html = true;
                        }
                    } else if response.url.contains("favicon")
                        || response.url.contains(".png")
                        || response.url.contains(".ico")
                    {
                        is_favicon = true;
                    }

                    if is_html {
                        tab.is_loading = false;
                        tab.loading_progress = 1.0;
                    }

                    if is_favicon && tab.favicon_data.is_none() {
                        if let Some((width, height, ref rgba_data)) = response.decoded_image {
                            let buffer =
                                slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                                    rgba_data, width, height,
                                );
                            tab.favicon_data = Some(buffer.clone());

                            // Também adicionar globalmente
                            let slint_image = slint::Image::from_rgba8(buffer);
                            tab.engine
                                .image_cache
                                .lock().unwrap_or_else(|e| e.into_inner())
                                .insert(response.url.clone(), slint_image);
                            tab.engine.mark_styles_dirty();

                            did_update = true;
                        }
                    } else if let Some((width, height, ref rgba_data)) = response.decoded_image {
                        // Imagens genéricas processadas em background
                        let buffer =
                            slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                                rgba_data, width, height,
                            );
                        let slint_image = slint::Image::from_rgba8(buffer);
                        tab.engine
                            .image_cache
                            .lock().unwrap_or_else(|e| e.into_inner())
                            .insert(response.url.clone(), slint_image);
                        tab.engine.mark_styles_dirty();
                        did_update = true;
                    }

                    if tab.engine.handle_resource_response(response) {
                        did_update = true;
                    }
                    count += 1;
                    if count > 50 {
                        break;
                    } // Limite por frame
                }

                tab.resource_rx = Some(rx);
                return did_update;
            }
        }
        false
    }

}
