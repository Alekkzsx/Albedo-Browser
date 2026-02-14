mod tab;
mod tab_manager;
mod engine;
mod js;

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

    // Initial Tab Creation
    let ui_handle_clone = ui_handle.clone();
    let tm_clone = tab_manager.clone();
    let sync_clone = sync_tabs.clone();
    
    // Create first tab immediately
    if let Some(ui) = ui_handle_clone.upgrade() {
        tm_clone.create_tab(ui.window(), "albedo://start");
        sync_clone();
        ui.set_current_url("".into());
        ui.set_show_start_page(true);
    }

    // Callbacks
    let tm_clone = tab_manager.clone();
    let ui_handle_clone = ui_handle.clone();
    let sync_clone = sync_tabs.clone(); 

    ui.on_navigate(move |url: slint::SharedString| {
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
        if let Some(ui) = ui_handle_clone.upgrade() {
             if let Some((url, show_start, _native_content, _mode)) = tm_clone.navigate(ui.window(), &final_url) {
                ui.set_current_url(url.into());
                ui.set_show_start_page(show_start);
                sync_ace_visuals(&ui, &tm_clone);
             }
             sync_clone();
        }
    });

    let tm_clone = tab_manager.clone();
    let sync_clone = sync_tabs.clone();
    let ui_handle_clone = ui_handle.clone();
    
    ui.on_request_new_tab(move || {
        if let Some(ui) = ui_handle_clone.upgrade() {
             tm_clone.create_tab(ui.window(), "albedo://start");
             sync_clone();
        }
    });

    let tm_clone = tab_manager.clone();
    let sync_clone = sync_tabs.clone();
    let ui_handle_clone = ui_handle.clone();
    
    ui.on_request_switch_tab(move |index| {
        let result = tm_clone.switch_to_tab(index as usize);
        sync_clone();
        
        if let Some((url, show_start, _native_content, _mode)) = result {
             if let Some(ui) = ui_handle_clone.upgrade() {
                 ui.set_current_url(url.into());
                 ui.set_show_start_page(show_start);
                 sync_ace_visuals(&ui, &tm_clone);
             }
        }
    });

    let tm_clone = tab_manager.clone();
    let sync_clone = sync_tabs.clone();
    ui.on_request_close_tab(move |index| {
        tm_clone.close_tab(index as usize);
        sync_clone();
    });

    // JS Pulse Timer
    let tm_pulse = tab_manager.clone();
    let ui_pulse = ui_handle.clone();
    let pulse_timer = slint::Timer::default();
    pulse_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(16), move || {
        if tm_pulse.pulse() {
            if let Some(ui) = ui_pulse.upgrade() {
                sync_ace_visuals(&ui, &tm_pulse);
            }
        }
    });

    ui.run()?;
    Ok(())
}

fn sync_ace_visuals(ui: &AppWindow, tm: &TabManager) {
    if let Some((_, Some(engine))) = tm.get_active_tab_native_data() {
        let primitives = engine.render_visual();
        let mut max_y = 0.0;
        let slint_boxes: Vec<ACEBox> = primitives.into_iter().map(|p| {
            if p.y + p.height > max_y {
                max_y = p.y + p.height;
            }
            let color = match p.color.as_str() {
                "blue" => slint::Color::from_rgb_u8(0, 0, 255),
                "red" => slint::Color::from_rgb_u8(255, 0, 0),
                "green" => slint::Color::from_rgb_u8(0, 255, 0),
                "orange" => slint::Color::from_rgb_u8(255, 165, 0),
                "purple" => slint::Color::from_rgb_u8(128, 0, 128),
                "yellow" => slint::Color::from_rgb_u8(255, 255, 0),
                _ => slint::Color::from_rgb_u8(200, 200, 200),
            };
            let (image_data, has_image) = if let Some(url) = &p.image_url {
                if url.starts_with("http") {
                    (slint::Image::default(), false)
                } else {
                    match slint::Image::load_from_path(std::path::Path::new(url)) {
                        Ok(img) => (img, true),
                        Err(_) => (slint::Image::default(), false)
                    }
                }
            } else {
                (slint::Image::default(), false)
            };

            ACEBox {
                x: p.x,
                y: p.y,
                width: p.width,
                height: p.height,
                background: color,
                text: p.text.into(),
                font_size: p.font_size,
                text_color: slint::Color::from_rgb_u8(51, 51, 51),
                image_data,
                has_image,
                link_url: p.link_url.unwrap_or_default().into(),
            }
        }).collect();
        let model = std::rc::Rc::new(slint::VecModel::from(slint_boxes));
        ui.set_ace_model(model.into());
        ui.set_content_height(max_y + 50.0);
    }
}