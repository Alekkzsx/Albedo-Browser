use super::collection::TabCollection;
use super::tab::TabMode;
use crate::ace::engine::AceEngine;
use crate::network::resources::ResourceManager;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct TabManager {
    collection: Rc<RefCell<TabCollection>>,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            collection: Rc::new(RefCell::new(TabCollection::new())),
        }
    }

    fn with_collection<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&TabCollection) -> R,
    {
        f(&self.collection.borrow())
    }

    fn with_collection_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut TabCollection) -> R,
    {
        f(&mut *self.collection.borrow_mut())
    }

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

    pub fn close_tab(&self, index: usize) {
        self.with_collection_mut(|col| col.close(index));
    }

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
                                .lock()
                                .unwrap()
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
                            .lock()
                            .unwrap()
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

    pub fn dispatch_click_to_active_tab(&self, node_idx: usize) -> bool {
        self.with_collection(|col| {
            if let Some(tab) = col.get_active() {
                if let Some(ref rt) = tab.engine.js_runtime {
                    if let Some(ref dom) = tab.engine.dom {
                        tracing::debug!(node_idx, "Dispatching click to node");
                        rt.dispatch_event(dom.clone(), node_idx, "click");
                        return true;
                    }
                }
            }
            false
        })
    }

    pub fn handle_click(&self, x: f32, y: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            let node_id = tab.engine.find_element_at_position(x, y);
            if let Some(idx) = node_id {
                // Comportamento nativo: toggle de <details> via <summary>
                if let Some(ref dom_arc) = tab.engine.dom {
                    let mut dom = dom_arc.lock().unwrap();

                    let is_summary = dom
                        .get_element(idx)
                        .map_or(false, |el| el.tag == "summary");

                    if is_summary {
                        let parent_idx = dom.get_node(idx).and_then(|n| n.parent);
                        if let Some(p_idx) = parent_idx {
                            let is_details = dom
                                .get_element(p_idx)
                                .map_or(false, |el| el.tag == "details");

                            if is_details {
                                let has_open = dom
                                    .get_element(p_idx)
                                    .map_or(false, |el| el.attributes.contains_key("open"));

                                if has_open {
                                    dom.remove_attribute_notify(p_idx, "open".into());
                                } else {
                                    dom.set_attribute_notify(p_idx, "open".into(), String::new());
                                }
                            }
                        }
                    }

                    // Comportamento nativo: <form method="dialog"> fecha <dialog> pai
                    let clicked_tag = dom
                        .get_element(idx)
                        .map_or_else(String::new, |el| el.tag.clone());

                    if clicked_tag == "button" || clicked_tag == "input" {
                        let button_value = dom
                            .get_element(idx)
                            .and_then(|el| el.attributes.get("value").cloned())
                            .unwrap_or_default();

                        let mut ancestor = dom.get_node(idx).and_then(|n| n.parent);
                        let mut found_form_dialog = false;
                        while let Some(a_idx) = ancestor {
                            if let Some(a_el) = dom.get_element(a_idx) {
                                if a_el.tag == "form"
                                    && a_el
                                        .attributes
                                        .get("method")
                                        .map_or(false, |m| m.eq_ignore_ascii_case("dialog"))
                                {
                                    found_form_dialog = true;
                                }
                                if found_form_dialog
                                    && a_el.tag == "dialog"
                                    && a_el.attributes.contains_key("open")
                                {
                                    if !button_value.is_empty() {
                                        dom.set_attribute_notify(
                                            a_idx,
                                            "data-return-value".into(),
                                            button_value.clone(),
                                        );
                                    }
                                    dom.remove_attribute_notify(a_idx, "open".into());
                                    dom.remove_attribute_notify(a_idx, "data-ace-modal".into());
                                    break;
                                }
                                ancestor = dom.get_node(a_idx).and_then(|n| n.parent);
                            } else {
                                break;
                            }
                        }
                    }
                }

                // Dispatch JS click event
                if let Some(ref rt) = tab.engine.js_runtime {
                    if let Some(ref dom) = tab.engine.dom {
                        tracing::debug!(x, y, node_idx = idx, "Click at position");
                        rt.dispatch_event(dom.clone(), idx, "click");
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn handle_hover(&self, x: f32, y: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            let node_id = tab.engine.find_element_at_position(x, y);
            let old_hover = tab.engine.hovered_element;

            if old_hover != node_id {
                // Disparar mouseout no elemento anterior
                if let Some(old_idx) = old_hover {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        if let Some(ref dom) = tab.engine.dom {
                            rt.dispatch_pointer_event(
                                old_idx,
                                "pointerout",
                                x,
                                y,
                                0,
                                1,
                                "mouse",
                                true,
                            );
                            rt.dispatch_event(dom.clone(), old_idx, "mouseout");
                        }
                    }
                }

                // Disparar mouseover no novo elemento
                if let Some(new_idx) = node_id {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        if let Some(ref dom) = tab.engine.dom {
                            rt.dispatch_pointer_event(
                                new_idx,
                                "pointerover",
                                x,
                                y,
                                0,
                                1,
                                "mouse",
                                true,
                            );
                            rt.dispatch_event(dom.clone(), new_idx, "mouseover");
                        }
                    }
                }

                tab.engine.set_hover(node_id);
                return true;
            } else {
                // Disparar mousemove/pointermove no elemento atual se a posição mudou e continua nele
                if let Some(idx) = node_id {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        rt.dispatch_touch_event(idx, "touchmove", x, y, 1);
                        rt.dispatch_pointer_event(idx, "pointermove", x, y, 0, 1, "mouse", true);
                        // rt.dispatch_event(dom.clone(), idx, "mousemove"); // mousemove pode ser implementado depois
                    }
                }
                return true;
            }
        }
        false
    }

    pub fn handle_pointer_down(&self, x: f32, y: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            let node_id = tab.engine.find_element_at_position(x, y);
            // Set active only if clicked on something
            // Standard behavior: clicking on empty space usually clears active state of others?
            // Actually, active state is usually per-element. If I click empty active becomes None.
            if node_id != tab.engine.active_element {
                tab.engine.set_active(node_id);

                // Atualizar foco também quando clica em um elemento
                let old_focused = tab.engine.focused_element;
                tab.engine.set_focused(node_id);

                // Disparar eventos blur/focus se o foco mudou
                if old_focused != node_id {
                    if let Some(old_idx) = old_focused {
                        if let Some(ref rt) = tab.engine.js_runtime {
                            if let Some(ref dom) = tab.engine.dom {
                                rt.dispatch_event(dom.clone(), old_idx, "blur");
                            }
                        }
                    }
                    if let Some(new_idx) = node_id {
                        if let Some(ref rt) = tab.engine.js_runtime {
                            if let Some(ref dom) = tab.engine.dom {
                                rt.dispatch_event(dom.clone(), new_idx, "focus");
                            }
                        }
                    }
                }

                if let Some(idx) = node_id {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        rt.dispatch_touch_event(idx, "touchstart", x, y, 1);
                        rt.dispatch_pointer_event(idx, "pointerdown", x, y, 0, 1, "mouse", true);
                        if let Some(ref dom) = tab.engine.dom {
                            rt.dispatch_event(dom.clone(), idx, "mousedown");
                        }
                    }
                }
                return true;
            }
        }
        false
    }

    pub fn handle_pointer_up(&self, x: f32, y: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            // On mouse up, we generally clear active state of the current active element
            if tab.engine.active_element.is_some() {
                let old_active = tab.engine.active_element;
                tab.engine.set_active(None);

                // Dispatch mouseup. Usually to the element under cursor OR the active element.
                // For CSS :active, it clears when mouse is released.
                if let Some(idx) = old_active {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        rt.dispatch_touch_event(idx, "touchend", x, y, 1);
                        rt.dispatch_pointer_event(idx, "pointerup", x, y, 0, 1, "mouse", true);
                        if let Some(ref dom) = tab.engine.dom {
                            rt.dispatch_event(dom.clone(), idx, "mouseup");
                        }
                    }
                }
                return true;
            }
        }
        false
    }

    pub fn handle_pointer_cancel(&self, x: f32, y: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            // On interaction cancel (e.g., system alert, drag cancelled)
            if tab.engine.active_element.is_some() {
                let old_active = tab.engine.active_element;
                tab.engine.set_active(None);

                // Dispatch pointercancel. Only pointercancel is fired, no mouse counterpart.
                if let Some(idx) = old_active {
                    if let Some(ref rt) = tab.engine.js_runtime {
                        rt.dispatch_touch_event(idx, "touchcancel", x, y, 1);
                        rt.dispatch_pointer_event(idx, "pointercancel", x, y, 0, 1, "mouse", true);
                    }
                }
                return true;
            }
        }
        false
    }

    pub fn handle_key_down(
        &self,
        key: &str,
        code: &str,
        ctrl: bool,
        shift: bool,
        alt: bool,
        meta: bool,
    ) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            // Comportamento nativo: ESC fecha dialog modal
            if key == "Escape" {
                if let Some(ref dom_arc) = tab.engine.dom {
                    let mut dom = dom_arc.lock().unwrap();
                    let mut modal_idx = None;
                    for (i, node) in dom.nodes.iter().enumerate() {
                        if let crate::ace::engine::dom::AceNodeType::Element(el) = &node.node_type {
                            if el.tag == "dialog"
                                && el.attributes.contains_key("open")
                                && el.attributes.contains_key("data-ace-modal")
                            {
                                modal_idx = Some(i);
                            }
                        }
                    }

                    if let Some(idx) = modal_idx {
                        // Disparar evento "cancel" (cancelável pela spec, mas simplificado aqui)
                        // Fechar o dialog
                        dom.remove_attribute_notify(idx, "open".into());
                        dom.remove_attribute_notify(idx, "data-ace-modal".into());

                        // Drop dom lock before trying to mutate tab
                        drop(dom);

                        // Cancelar pointers ativos também
                        if let Some(idx_active) = tab.engine.active_element {
                            if let Some(ref rt) = tab.engine.js_runtime {
                                rt.dispatch_touch_event(idx_active, "touchcancel", 0.0, 0.0, 1);
                                rt.dispatch_pointer_event(
                                    idx_active,
                                    "pointercancel",
                                    0.0,
                                    0.0,
                                    0,
                                    1,
                                    "mouse",
                                    true,
                                );
                            }
                            tab.engine.set_active(None);
                        }

                        return true;
                    }
                }
            }

            // Dispatch JS keydown event
            if let Some(ref rt) = tab.engine.js_runtime {
                if let Some(focused_idx) = tab.engine.focused_element {
                    rt.dispatch_keyboard_event(
                        focused_idx,
                        "keydown",
                        key,
                        code,
                        ctrl,
                        shift,
                        alt,
                        meta,
                    );
                    return true;
                }
            }
        }
        false
    }

    pub fn handle_key_up(
        &self,
        key: &str,
        code: &str,
        ctrl: bool,
        shift: bool,
        alt: bool,
        meta: bool,
    ) -> bool {
        self.with_collection(|col| {
            if let Some(tab) = col.get_active() {
                if let Some(ref rt) = tab.engine.js_runtime {
                    if let Some(focused_idx) = tab.engine.focused_element {
                        rt.dispatch_keyboard_event(
                            focused_idx,
                            "keyup",
                            key,
                            code,
                            ctrl,
                            shift,
                            alt,
                            meta,
                        );
                        return true;
                    }
                }
            }
            false
        })
    }

    pub fn handle_scroll(&self, x: f32, y: f32, delta: f32) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            // Verificar elemento sob o cursor
            let node_id = tab.engine.find_element_at_position(x, y);

            if let Some(idx) = node_id {
                if let Some(ref dom_arc) = tab.engine.dom {
                    let dom = dom_arc.lock().unwrap();
                    let geometry = tab.engine.element_geometry.lock().unwrap();
                    let mut scroll_map = tab.engine.element_scroll.lock().unwrap();

                    let mut current = Some(idx);
                    while let Some(current_idx) = current {
                        if let Some(geom) = geometry.get(&current_idx) {
                            if (geom.overflow_y
                                == crate::ace::engine::style::css_values::CssOverflow::Scroll
                                || geom.overflow_y
                                    == crate::ace::engine::style::css_values::CssOverflow::Auto)
                                && geom.content_height > geom.height
                            {
                                // Encontramos um container scrollável internamente
                                let current_scroll =
                                    scroll_map.get(&current_idx).copied().unwrap_or((0.0, 0.0));

                                // delta > 0 (scroll up) diminui o offset Y; delta < 0 (scroll down) aumenta o offset Y
                                let mut new_sy = current_scroll.1 - delta;
                                let max_scroll = geom.content_height - geom.height;

                                new_sy = new_sy.clamp(0.0, max_scroll);
                                scroll_map.insert(current_idx, (current_scroll.0, new_sy));

                                // Marcar para redesenho
                                drop(scroll_map);
                                drop(geometry);
                                drop(dom);
                                return true;
                            }
                        }
                        current = dom.get_node(current_idx).and_then(|n| n.parent);
                    }
                }
            }

            // Fallback: scroll do viewport principal
            let new_y = tab.engine.viewport_y + delta;
            tab.engine.viewport_y = new_y.min(0.0);
            return true;
        }
        false
    }

    pub fn process_animations(&self) -> bool {
        let mut col = self.collection.borrow_mut();
        if let Some(tab) = col.get_active_mut() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            if tab.engine.tick(now) {
                return true;
            }
        }
        false
    }

    pub fn request_navigate(&self, url: String) {
        self.with_collection_mut(|col| col.pending_nav = Some(url));
    }

    pub fn take_pending_nav(&self) -> Option<String> {
        self.with_collection_mut(|col| col.pending_nav.take())
    }
}
