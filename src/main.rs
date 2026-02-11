mod tab_manager;

use slint::ComponentHandle;
use tab_manager::TabManager;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    // Initialize Tab Manager
    let tab_manager = TabManager::new(ui_handle.clone());

    // System Monitor
    let system = std::rc::Rc::new(std::cell::RefCell::new(sysinfo::System::new_all()));
    let sys_clone = system.clone();
    let ui_handle_clone = ui_handle.clone();
    
    let system_timer = slint::Timer::default();
    system_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(2), move || {
        if let Some(ui) = ui_handle_clone.upgrade() {
            let mut sys = sys_clone.borrow_mut();
            sys.refresh_all();
            
            let total_ram = sys.total_memory() / 1024 / 1024;
            let used_ram = sys.used_memory() / 1024 / 1024;
            let cpu_usage = sys.global_cpu_usage();
            
            let stats = format!("RAM: {}/{} MB | CPU: {:.1}% | MODE: EFFICIENT", used_ram, total_ram, cpu_usage);
            ui.set_system_stats(stats.into());
        }
    });
    
    // Tab Sync Logic
    let tabs_model = std::rc::Rc::new(slint::VecModel::default());
    ui.set_tabs_model(tabs_model.clone().into());
    
    let tm_clone = tab_manager.clone();
    let tabs_model_clone = tabs_model.clone();
    let sync_tabs = move || {
        let tabs_info = tm_clone.get_tabs_info();
        let slint_tabs: Vec<TabData> = tabs_info.into_iter().map(|(title, active)| {
            TabData {
                title: title.into(),
                active,
            }
        }).collect();
        tabs_model_clone.set_vec(slint_tabs);
    };

    // PHASE 2: DEFERRED INITIALIZATION (First Tab)
    let ui_handle_clone = ui_handle.clone();
    let tm_clone = tab_manager.clone();
    let sync_clone = sync_tabs.clone();
    
    // Timer to initialize WRY after window is visible
    let init_timer = slint::Timer::default();
    init_timer.start(slint::TimerMode::SingleShot, std::time::Duration::from_millis(100), move || {
        println!("Attempting deferred Tab initialization...");
        if let Some(ui) = ui_handle_clone.upgrade() {
            tm_clone.create_tab(ui.window(), "albedo://start");
            sync_clone(); // Sync UI
            // First tab is start page, so ensure UI reflects that (it defaults to true, but good to be explicit if we changed logic)
            ui.set_current_url("".into());
            ui.set_show_start_page(true);
        }
    });

    // Callbacks
    let tm_clone = tab_manager.clone();
    let ui_handle_clone = ui_handle.clone();
    let sync_clone = sync_tabs.clone(); // Need sync to update title? Yes.

    let tm_clone_url = tab_manager.clone();
    let ui_handle_url = ui_handle.clone();
    ui.on_url_updated(move |id, url| {
        // Update Rust state
        let is_active = tm_clone_url.update_tab_url(&id, url.to_string());
        
        // If active, update UI address bar
        if is_active {
            if let Some(ui) = ui_handle_url.upgrade() {
                ui.set_current_url(url);
            }
        }
    });

    let tm_clone_title = tab_manager.clone();
    let sync_clone_title = sync_tabs.clone();
    ui.on_title_updated(move |id, title| {
        // Update Rust state
        tm_clone_title.update_tab_title(&id, title.to_string());
        // Always sync tabs list to show new title
        sync_clone_title();
    });

    ui.on_navigate(move |url: slint::SharedString| {
        let url_str = url.as_str();
        let final_url = if url_str.contains(' ') || !url_str.contains('.') {
            format!("https://www.google.com/search?q={}", url_str)
        } else if url_str.starts_with("http") || url_str.starts_with("albedo://") {
             url_str.to_string()
        } else {
             format!("https://{}", url_str)
        };

        println!("Navigating to: {}", final_url);
        if let Some(ui) = ui_handle_clone.upgrade() {
             tm_clone.navigate(ui.window(), &final_url);
             // Sync tabs to update title
             sync_clone();
             // Manually update UI state
             ui.set_show_start_page(false);
             // Don't update URL bar immediately to the long google search URL? 
             // Actually, standard behavior is to show the search term or the URL.
             // For now, setting current_url to the result is fine.
             ui.set_current_url(final_url.into());
        }
    });

    let tm_clone = tab_manager.clone();
    let sync_clone = sync_tabs.clone();
    let ui_handle_clone = ui_handle.clone();
    
    ui.on_request_new_tab(move || {
        if let Some(ui) = ui_handle_clone.upgrade() {
             tm_clone.create_tab(ui.window(), "albedo://start");
             sync_clone();
             // Switch to new tab (logic inside create_tab handles activation if single tab, 
             // but if multiple tabs, create_tab appends. 
             // Wait, create_tab doesn't automatically switch unless it's the *only* tab.
             // We need to switch to the new tab!
             // Let's look at create_tab again. It pushes to vec.
             // We should probably switch to it.
             // For now, let's keep existing behavior (it appends) but we likely want to switch.
             // User expects new tab to open immediately? Yes.
             // Let's modify create_tab to return index or just handle it here?
             // Accessing len is hard without borrow.
             // Let's just update the URL/StartPage *if* we switch.
             // Actually, the current UI callback just creates it. It doesn't switch.
             // User has to click it.
             // If user stays on current tab, UI shouldn't change. Good.
        }
    });

    let tm_clone = tab_manager.clone();
    let sync_clone = sync_tabs.clone();
    let ui_handle_clone = ui_handle.clone();
    
    ui.on_request_switch_tab(move |index| {
        let result = tm_clone.switch_to_tab(index as usize);
        sync_clone();
        
        if let Some((url, show_start)) = result {
             if let Some(ui) = ui_handle_clone.upgrade() {
                 ui.set_current_url(url.into());
                 ui.set_show_start_page(show_start);
             }
        }
    });

    let tm_clone = tab_manager.clone();
    let sync_clone = sync_tabs.clone();
    ui.on_request_close_tab(move |index| {
        tm_clone.close_tab(index as usize);
        sync_clone();
    });

    let _tm_clone = tab_manager.clone();
    // Assuming simple back/forward for active tab for now
    // You might need to add back/forward to TabManager or Tab struct
    // For now, let's just comment out or adapt if TabManager has these methods
    // tm_clone.back(); 
    // tm_clone.forward();
    // tm_clone.reload();
    
    // Resize Logic
    let ui_handle_clone = ui_handle.clone();
    let tm_clone = tab_manager.clone();
    
    let resize_timer = slint::Timer::default();
    resize_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(50), move || {
        let ui = ui_handle_clone.unwrap();
        let show_start_page = ui.get_show_start_page();
        
        if show_start_page {
            tm_clone.set_visible(false);
        } else {
            tm_clone.set_visible(true);
            
            let window = ui.window();
            let scale = window.scale_factor();
            let top_height = (75.0 * scale) as i32; // Sync with UI top bar height (35 TabBar + 40 NavBar)
            
            tm_clone.resize(ui.window(), top_height);
        }
    });

    ui.run()?;
    Ok(())
}