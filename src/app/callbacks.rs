use crate::ui::{AppWindow, TabData};
use crate::tab_manager::TabManager;
use crate::app::sync::sync_ace_visuals;
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
    let final_url = if url_str.contains(' ') || !url_str.contains('.') {
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

            if let Some((url, show_start, _native_content, _mode)) = tm.navigate(ui.window(), &final_url) {
            ui.set_current_url(url.into());
            ui.set_show_start_page(show_start);
            sync_ace_visuals(&ui, tm);
            }
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

pub fn handle_pulse(ui_handle: &Weak<AppWindow>, tm: &TabManager) {
    if tm.pulse() {
        if let Some(ui) = ui_handle.upgrade() {
            sync_ace_visuals(&ui, tm);
        }
    }
}

pub fn handle_click(ui_handle: &Weak<AppWindow>, tm: &TabManager, ptr_str: SharedString) {
    let ptr = if ptr_str.starts_with("0x") {
        usize::from_str_radix(&ptr_str[2..], 16).unwrap_or(0)
    } else {
        ptr_str.parse::<usize>().unwrap_or(0)
    };

    if ptr != 0 {
        if let Some(ui) = ui_handle.upgrade() {
             if tm.dispatch_click_to_active_tab(ptr) {
                 sync_ace_visuals(&ui, tm);
             }
        }
    }
}
