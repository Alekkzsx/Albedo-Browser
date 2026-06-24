use albedo::utils::sysinfo::AceSysInfo;
use albedo::browser;
use albedo::ui;

use albedo::browser::tabs::manager::TabManager;
use slint::ComponentHandle;

/// TODO: add docs
fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .init();
}

/// TODO: add docs
fn parse_start_url() -> String {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if args[1].starts_with("http")
            || args[1].starts_with("albedo:")
            || args[1].starts_with("file:")
        {
            args[1].clone()
        } else {
            let path = std::env::current_dir().expect("Albedo Engine: internal invariant violated").join(&args[1]);
            format!("file://{}", path.display())
        }
    } else {
        "albedo://start".to_string()
    }
}

/// TODO: add docs
fn create_initial_tab(
    ui: &slint::Weak<ui::AppWindow>,
    tab_manager: &TabManager,
    tabs_model: &slint::VecModel<ui::TabData>,
) {
    let start_url = parse_start_url();
    if let Some(ui) = ui.upgrade() {
        tab_manager.create_tab(ui.window(), &start_url);
        browser::bridge::sync_ace_visuals(&ui, tab_manager);
        browser::events::sync_tabs(tab_manager, tabs_model);
        ui.set_current_url(start_url.into());
        ui.set_show_start_page(true);
    }
}

/// TODO: add docs
fn setup_timers(
    ui_handle: &slint::Weak<ui::AppWindow>,
    tab_manager: &TabManager,
) {
    // System Monitor
    let system = std::rc::Rc::new(std::cell::RefCell::new(AceSysInfo::new()));
    let sys_clone = system.clone();
    let ui_handle_clone = ui_handle.clone();

    let system_timer = slint::Timer::default();
    system_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(2),
        move || {
            browser::events::handle_system_monitor(&ui_handle_clone, &sys_clone);
        },
    );

    // JS Pulse Timer
    let tm_pulse = tab_manager.clone();
    let ui_pulse = ui_handle.clone();
    let pulse_timer = slint::Timer::default();
    pulse_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(16),
        move || {
            browser::events::handle_pulse(&ui_pulse, &tm_pulse);
        },
    );
}

/// TODO: add docs
fn register_callbacks(
    ui: &ui::AppWindow,
    tab_manager: &TabManager,
    ui_handle: &slint::Weak<ui::AppWindow>,
    tabs_model: &slint::VecModel<ui::TabData>,
) {
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

    let tm_key = tab_manager.clone();
    ui.on_key_down(move |key, code, ctrl, shift, alt, meta| {
        browser::events::handle_key_down(&tm_key, key, code, ctrl, shift, alt, meta);
    });

    let tm_key_up = tab_manager.clone();
    ui.on_key_up(move |key, code, ctrl, shift, alt, meta| {
        browser::events::handle_key_up(&tm_key_up, key, code, ctrl, shift, alt, meta);
    });

    let tm_scroll = tab_manager.clone();
    let ui_scroll = ui_handle.clone();
    ui.on_scroll(move |x, y, delta| {
        browser::events::handle_scroll(&ui_scroll, &tm_scroll, x, y, delta);
    });
}

/// TODO: add docs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_tracing();
    browser::setup::set_panic_hook();

    let rt = tokio::runtime::Runtime::new()?;
    let _guard = rt.enter();

    tracing::info!("Starting Albedo Browser");
    let ui = browser::setup::create_window()?;
    albedo_jit::contracts::core::register_json_parser(albedo::ace::json::parse);
    let ui_handle = ui.as_weak();

    let tab_manager = TabManager::new();
    let tabs_model = std::rc::Rc::new(slint::VecModel::default());
    ui.set_tabs_model(tabs_model.clone().into());

    setup_timers(&ui_handle, &tab_manager);
    create_initial_tab(&ui_handle, &tab_manager, &tabs_model);
    register_callbacks(&ui, &tab_manager, &ui_handle, &tabs_model);

    ui.run()?;
    Ok(())
}
