use crate::ui::{AppWindow, TabData};
use crate::browser::tabs::manager::TabManager;
use crate::browser::bridge::sync_ace_visuals;
use slint::{ComponentHandle, SharedString, VecModel, Weak};
use std::rc::Rc;
use std::cell::RefCell;
use sysinfo::System;

pub fn handle_system_monitor(ui_handle: &Weak<AppWindow>, system: &Rc<RefCell<System>>) {
    if let Some(ui) = ui_handle.upgrade() {
        let mut sys = system.borrow_mut();
        sys.refresh_all();
        
        let total_ram = sys.total_memory() / 1024 / 1024;
        let used_ram = sys.used_memory() / 1024 / 1024;
        let cpu_usage = sys.global_cpu_usage();
        
        let stats = format!("RAM: {}/{} MB | CPU: {:.1}% | MODE: EFFICIENT", used_ram, total_ram, cpu_usage);
        ui.set_system_stats(stats.into());
    }
}

pub fn sync_tabs(tm: &TabManager, tabs_model: &Rc<VecModel<TabData>>) {
    let tabs_info = tm.get_tabs_info();
    let slint_tabs: Vec<TabData> = tabs_info.into_iter().map(|(title, active)| {
        TabData {
            title: title.into(),
            active,
        }
    }).collect();
    tabs_model.set_vec(slint_tabs);
}

pub fn handle_navigate(ui_handle: &Weak<AppWindow>, tm: &TabManager, url: SharedString, tabs_model: &Rc<VecModel<TabData>>) {
    let url_str = url.as_str();
    let final_url = if url_str.starts_with("file://") {
        url_str.to_string()
    } else if url_str.starts_with("/") {
        format!("file:///{}", url_str.trim_start_matches('/'))
    } else if url_str.contains(' ') || !url_str.contains('.') {
        if !url_str.starts_with("albedo://") {
            format!("https://www.google.com/search?q={}", url_str)
        } else {
            url_str.to_string()
        }
    } else if url_str.starts_with("http") || url_str.starts_with("albedo://") {
            url_str.to_string()
    } else {
            format!("https://{}", url_str)
    };

    println!("Navigating to: {}", final_url);
    if let Some(ui) = ui_handle.upgrade() {
            // Guard against infinite loops / same URL
            let current = ui.get_current_url();
            if current.as_str() == final_url {
                println!("[Navigation] Ignoring duplicate request to: {}", final_url);
                return;
            }

            tm.request_navigate(final_url);
            sync_tabs(tm, tabs_model);
    }
}

pub fn handle_new_tab(ui_handle: &Weak<AppWindow>, tm: &TabManager, tabs_model: &Rc<VecModel<TabData>>) {
    if let Some(ui) = ui_handle.upgrade() {
         tm.create_tab(ui.window(), "albedo://start");
         sync_tabs(tm, tabs_model);
    }
}

pub fn handle_switch_tab(ui_handle: &Weak<AppWindow>, tm: &TabManager, index: i32, tabs_model: &Rc<VecModel<TabData>>) {
    let result = tm.switch_to_tab(index as usize);
    sync_tabs(tm, tabs_model);
    
    if let Some((url, show_start, _native_content, _mode)) = result {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_current_url(url.into());
                ui.set_show_start_page(show_start);
                sync_ace_visuals(&ui, tm);
            }
    }
}

pub fn handle_close_tab(tm: &TabManager, index: i32, tabs_model: &Rc<VecModel<TabData>>) {
    tm.close_tab(index as usize);
    sync_tabs(tm, tabs_model);
}

pub fn handle_pointer_click(ui_handle: &Weak<AppWindow>, tm: &TabManager, x: f32, y: f32) {
    if let Some(ui) = ui_handle.upgrade() {
         if tm.handle_click(x, y) {
             sync_ace_visuals(&ui, tm);
         }
    }
}

pub fn handle_hover(ui_handle: &Weak<AppWindow>, tm: &TabManager, x: f32, y: f32) {
    if let Some(ui) = ui_handle.upgrade() {
         if tm.handle_hover(x, y) {
             sync_ace_visuals(&ui, tm);
         }
    }
}

pub fn handle_pointer_down(ui_handle: &Weak<AppWindow>, tm: &TabManager, x: f32, y: f32) {
    if let Some(ui) = ui_handle.upgrade() {
         if tm.handle_pointer_down(x, y) {
             sync_ace_visuals(&ui, tm);
         }
    }
}

pub fn handle_pointer_up(ui_handle: &Weak<AppWindow>, tm: &TabManager, x: f32, y: f32) {
    if let Some(ui) = ui_handle.upgrade() {
         if tm.handle_pointer_up(x, y) {
             sync_ace_visuals(&ui, tm);
         }
    }
}

pub fn handle_key_down(tm: &TabManager, key: SharedString, code: SharedString, ctrl: bool, shift: bool, alt: bool, meta: bool) {
    if tm.handle_key_down(key.as_str(), code.as_str(), ctrl, shift, alt, meta) {
        // Se a engine mudou algo (ex: focus), poderíamos sincronizar aqui,
        // mas o pulse timer cuidará disso se houver mudanças de estilo/mutação.
        println!("[Events] KeyDown handled: {}", key);
    }
}

pub fn handle_key_up(tm: &TabManager, key: SharedString, code: SharedString, ctrl: bool, shift: bool, alt: bool, meta: bool) {
    tm.handle_key_up(key.as_str(), code.as_str(), ctrl, shift, alt, meta);
}

pub fn handle_scroll(ui_handle: &Weak<AppWindow>, tm: &TabManager, x: f32, y: f32, delta: f32) {
    if let Some(ui) = ui_handle.upgrade() {
        if tm.handle_scroll(x, y, delta) {
            sync_ace_visuals(&ui, tm);
        }
    }
}

pub fn handle_pulse(ui_handle: &slint::Weak<AppWindow>, tm: &TabManager) {
    if let Some(ui) = ui_handle.upgrade() {
        // Processar navegação pendente
        if let Some(pending_url) = tm.take_pending_nav() {
             if let Some((url, show_start, _loading, _status)) = tm.navigate(ui.window(), &pending_url) {
                 ui.set_current_url(url.into());
                 ui.set_show_start_page(show_start);
                 sync_ace_visuals(&ui, tm);
             }
        }

        // Processar recursos assíncronos primeiro
        if tm.process_active_tab_resources() {
            sync_ace_visuals(&ui, tm);
        }

        // Animações
        if tm.process_animations() {
            sync_ace_visuals(&ui, tm);
        }

        if let Some((_, Some(mut engine))) = tm.get_active_tab_native_data() {
            // Recompilar estilos se hover/focus mudou
            if engine.styles_dirty {
                println!("[Pulse] Recomputing dirty styles...");
                engine.recompute_dirty_styles();
                sync_ace_visuals(&ui, tm);
            }

            // Pulse JS Runtime
            if let Some(ref rt) = engine.js_runtime {
                // Sincroniza scroll position para uso de instersectionObservers no event_loop
                *rt.viewport_y.lock().unwrap() = engine.viewport_y;

                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64() * 1000.0;
                
                let raf_executed = rt.run_raf_callbacks(now_ms);
                // println!("[Pulse] Running JS pending jobs...");
                let (js_executed, js_style_dirty) = rt.run_pending();
                // println!("[Pulse] JS pending jobs done.");
                
                if raf_executed || js_executed || js_style_dirty {
                    sync_ace_visuals(&ui, tm);
                }
            }

            let (mutated, style_dirty) = engine.check_mutations();
            
            if style_dirty {
                println!("[Pulse] Updating stylesheet...");
                engine.update_stylesheet();
                sync_ace_visuals(&ui, tm);
            } else if mutated {
                println!("[Pulse] Recomputing layout due to mutations...");
                engine.recompute_layout();
                println!("[Pulse] Syncing ACE visuals...");
                sync_ace_visuals(&ui, tm);
                println!("[Pulse] Layout and visual sync complete.");
            }
        }
    }
}

