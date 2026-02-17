#![allow(warnings)]

mod browser;
mod engine;
mod runtime;
mod network;
mod ui;

use slint::ComponentHandle;
use browser::tabs::manager::TabManager;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    browser::setup::set_panic_hook();

    println!("[Main] Starting Albedo Browser...");
    let ui = browser::setup::create_window()?;
    let ui_handle = ui.as_weak();

    let tab_manager = TabManager::new();

    // System Monitor
    let system = std::rc::Rc::new(std::cell::RefCell::new(sysinfo::System::new_all()));
    let sys_clone = system.clone();
    let ui_handle_clone = ui_handle.clone();
    
    let system_timer = slint::Timer::default();
    system_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(2), move || {
        browser::events::handle_system_monitor(&ui_handle_clone, &sys_clone);
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
        browser::bridge::sync_ace_visuals(&ui, &tm_clone);
        browser::events::sync_tabs(&tm_clone, &tabs_model_clone);
        ui.set_current_url("".into());
        ui.set_show_start_page(true);
    }

    // Callbacks
    let tm_clone = tab_manager.clone();
    let ui_handle_clone = ui_handle.clone();
    let tabs_model_clone = tabs_model.clone();
    
    ui.on_navigate(move |url| {
        browser::events::handle_navigate(&ui_handle_clone, &tm_clone, url, &tabs_model_clone);
    });

    let tm_clone = tab_manager.clone();
    let tabs_model_clone = tabs_model.clone();
    let ui_handle_clone = ui_handle.clone();
    
    ui.on_request_new_tab(move || {
        browser::events::handle_new_tab(&ui_handle_clone, &tm_clone, &tabs_model_clone);
    });

    let tm_clone = tab_manager.clone();
    let tabs_model_clone = tabs_model.clone();
    let ui_handle_clone = ui_handle.clone();
    
    ui.on_request_switch_tab(move |index| {
        browser::events::handle_switch_tab(&ui_handle_clone, &tm_clone, index, &tabs_model_clone);
    });

    let tm_clone = tab_manager.clone();
    let tabs_model_clone = tabs_model.clone();
    ui.on_request_close_tab(move |index| {
        browser::events::handle_close_tab(&tm_clone, index, &tabs_model_clone);
    });

    // JS Pulse Timer
    let tm_pulse = tab_manager.clone();
    let ui_pulse = ui_handle.clone();
    let pulse_timer = slint::Timer::default();
    pulse_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(16), move || {
        browser::events::handle_pulse(&ui_pulse, &tm_pulse);
    });


    let tm_click = tab_manager.clone();
    let ui_click = ui_handle.clone();
    ui.on_pointer_click(move |x, y| {
        browser::events::handle_pointer_click(&ui_click, &tm_click, x, y); 
    });

    let tm_move = tab_manager.clone();
    let ui_move = ui_handle.clone();
    ui.on_pointer_move(move |x, y| {
        browser::events::handle_hover(&ui_move, &tm_move, x, y);
    });

    let tm_down = tab_manager.clone();
    let ui_down = ui_handle.clone();
    ui.on_pointer_down(move |x, y| {
        browser::events::handle_pointer_down(&ui_down, &tm_down, x, y);
    });

    let tm_up = tab_manager.clone();
    let ui_up = ui_handle.clone();
    ui.on_pointer_up(move |x, y| {
        browser::events::handle_pointer_up(&ui_up, &tm_up, x, y);
    });

    ui.run()?;
    Ok(())
}
