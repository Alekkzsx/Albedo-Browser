use crate::ui::{AppWindow, ACEBox};
use crate::tab_manager::TabManager;
use slint::ComponentHandle;

pub fn sync_ace_visuals(ui: &AppWindow, tm: &TabManager) {
    println!("[UI] Syncing ACE visuals...");
    if let Some((_, Some(engine))) = tm.get_active_tab_native_data() {
        let primitives = engine.render_visual();
        let mut max_y = 0.0;
        let slint_boxes: Vec<ACEBox> = primitives.into_iter().map(|p| {
            if p.y + p.height > max_y {
                max_y = p.y + p.height;
            }
            let bg_color = slint::Color::from_argb_u8(p.bg_color.a, p.bg_color.r, p.bg_color.g, p.bg_color.b);
            let text_color = slint::Color::from_argb_u8(p.color.a, p.color.r, p.color.g, p.color.b);
            
            let (image_data, has_image) = if let Some(url) = &p.image_url {
                if url.starts_with("http") || url.starts_with("https") {
                    // Web images not yet supported in Slint sync (need async fetch -> data url or temp file)
                    (slint::Image::default(), false)
                } else {
                    let path = std::path::Path::new(url);
                    if path.exists() {
                        match slint::Image::load_from_path(path) {
                            Ok(img) => (img, true),
                            Err(_) => (slint::Image::default(), false)
                        }
                    } else {
                        (slint::Image::default(), false)
                    }
                }
            } else {
                (slint::Image::default(), false)
            };

            // Sanity check dimensions
            let safe_width = if p.width.is_finite() && p.width >= 0.0 { p.width } else { 0.0 };
            let safe_height = if p.height.is_finite() && p.height >= 0.0 { p.height } else { 0.0 };
            
            // Log if weird
            if !p.width.is_finite() || !p.height.is_finite() {
                 println!("[UI-Sync] Warning: Non-finite dimensions for node {:x}: {}x{}", p.node_ptr, p.width, p.height);
            }

            ACEBox {
                x: p.x, // x and y can be negative (offscreen)
                y: p.y,
                width: safe_width,
                height: safe_height,
                background: bg_color,
                text: p.text.into(),
                font_size: p.font_size,
                text_color: text_color,
                image_data: image_data,
                has_image: has_image,
                link_url: p.link_url.unwrap_or_default().into(),
                node_id: format!("0x{:x}", p.node_ptr).into(),
                element_type: p.element_type.into(),
            }
        }).collect();
        println!("[UI] Converted {} primitives to Slint boxes.", slint_boxes.len());
        let model = std::rc::Rc::new(slint::VecModel::from(slint_boxes));
        ui.set_ace_model(model.into());
        ui.set_content_height(max_y + 50.0);
    }
}
