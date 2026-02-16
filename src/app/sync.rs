use crate::ui::{AppWindow, ACEBox};
use crate::tab_manager::TabManager;
use std::time::Duration;
use url::Url;



pub fn sync_ace_visuals(ui: &AppWindow, tm: &TabManager) {
    if let Some((_, Some(engine))) = tm.get_active_tab_native_data() {
        let primitives = engine.render_visual();
        let cache = engine.image_cache.lock().unwrap();
        
        let slint_boxes: Vec<ACEBox> = primitives.into_iter().map(|p| {
            // Conversor Hexadecimal Robusto
            let bg_color = if p.color.starts_with('#') && p.color.len() == 7 {
                let r = u8::from_str_radix(&p.color[1..3], 16).unwrap_or(255);
                let g = u8::from_str_radix(&p.color[3..5], 16).unwrap_or(255);
                let b = u8::from_str_radix(&p.color[5..7], 16).unwrap_or(255);
                slint::Color::from_rgb_u8(r, g, b)
            } else {
                slint::Color::from_rgb_u8(240, 240, 240) 
            };

            // Lógica de Imagem Assíncrona
            let is_image = p.element_type == "image";
            let mut image_data = slint::Image::default();
            let mut has_image = false;

            if is_image {
                if let Some(ref img_url) = p.image_url {
                    // Resolve URL para chave do cache
                    let resolved_url = if let Ok(base) = Url::parse(&engine.current_url) {
                        base.join(img_url).ok().map(|u| u.to_string()).unwrap_or(img_url.clone())
                    } else {
                        img_url.clone()
                    };

                    if let Some(img) = cache.get(&resolved_url) {
                        image_data = img.clone();
                        has_image = true;
                    } else {
                        // Não está no cache? Solicita fetch assíncrono!
                        if let Some(ref rm) = engine.resource_manager {
                            rm.fetch(resolved_url, crate::services::resource_manager::ResourceType::Image);
                        }
                    }
                }
            }

            ACEBox {
                x: p.x,
                y: p.y,
                width: p.width,
                height: p.height,
                background: bg_color,
                text: p.text.into(),
                font_size: p.font_size,
                text_color: slint::Color::from_rgb_u8(0, 0, 0),
                image_data,
                has_image,
                link_url: p.link_url.unwrap_or_default().into(),
                node_id: p.node_idx.to_string().into(),
                element_type: p.element_type.into(),
                is_fixed: p.is_fixed,
            }
        }).collect();

        let last_y = slint_boxes.last().map(|b| b.y + b.height).unwrap_or(0.0);
        
        let model = std::rc::Rc::new(slint::VecModel::from(slint_boxes));
        ui.set_ace_model(model.into());
        ui.set_content_height(last_y);
    }
}
