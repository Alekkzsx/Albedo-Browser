mod tab;
mod tab_manager;
mod engine;
mod layout;
mod js;
mod services;

use slint::ComponentHandle;
use tab_manager::TabManager;

mod app;
mod ui;
use ui::*;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Custom panic hook for better debugging
    app::setup::set_panic_hook();

    println!("[Main] Starting Albedo Browser...");
    let ui = app::setup::create_window()?;
    let ui_handle = ui.as_weak();

    // Initialize Tab Manager
    let tab_manager = TabManager::new(ui_handle.clone());

    // System Monitor
    let system = std::rc::Rc::new(std::cell::RefCell::new(sysinfo::System::new_all()));
    let sys_clone = system.clone();
    let ui_handle_clone = ui_handle.clone();
    
    let system_timer = slint::Timer::default();
    system_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(2), move || {
        app::callbacks::handle_system_monitor(&ui_handle_clone, &sys_clone);
    });
    
    // Tab Sync Logic (Initialization only now)
    let tabs_model = std::rc::Rc::new(slint::VecModel::default());
    ui.set_tabs_model(tabs_model.clone().into());
    
    // Initial Tab Creation
    let ui_handle_clone = ui_handle.clone();
    let tm_clone = tab_manager.clone();
    let tabs_model_clone = tabs_model.clone();
    
    // Create first tab immediately
    if let Some(ui) = ui_handle_clone.upgrade() {
        tm_clone.create_tab(ui.window(), "albedo://start");
        app::sync::sync_ace_visuals(&ui, &tm_clone);
        app::callbacks::sync_tabs(&tm_clone, &tabs_model_clone);
        ui.set_current_url("".into());
        ui.set_show_start_page(true);
    }

    // Callbacks
    let tm_clone = tab_manager.clone();
    let ui_handle_clone = ui_handle.clone();
    let tabs_model_clone = tabs_model.clone();
    
    ui.on_navigate(move |url| {
        app::callbacks::handle_navigate(&ui_handle_clone, &tm_clone, url, &tabs_model_clone);
    });

    let tm_clone = tab_manager.clone();
    let tabs_model_clone = tabs_model.clone();
    let ui_handle_clone = ui_handle.clone();
    
    ui.on_request_new_tab(move || {
        app::callbacks::handle_new_tab(&ui_handle_clone, &tm_clone, &tabs_model_clone);
    });

    let tm_clone = tab_manager.clone();
    let tabs_model_clone = tabs_model.clone();
    let ui_handle_clone = ui_handle.clone();
    
    ui.on_request_switch_tab(move |index| {
        app::callbacks::handle_switch_tab(&ui_handle_clone, &tm_clone, index, &tabs_model_clone);
    });

    let tm_clone = tab_manager.clone();
    let tabs_model_clone = tabs_model.clone();
    ui.on_request_close_tab(move |index| {
        app::callbacks::handle_close_tab(&tm_clone, index, &tabs_model_clone);
    });

    /*
    // JS Pulse Timer
    let tm_pulse = tab_manager.clone();
    let ui_pulse = ui_handle.clone();
    let pulse_timer = slint::Timer::default();
    pulse_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(16), move || {
        app::callbacks::handle_pulse(&ui_pulse, &tm_pulse);
    });
    */

    let tm_click = tab_manager.clone();
    let ui_click = ui_handle.clone();
    ui.on_box_clicked(move |ptr_str| {
        app::callbacks::handle_click(&ui_click, &tm_click, ptr_str);
    });

    ui.run()?;
    Ok(())
}
