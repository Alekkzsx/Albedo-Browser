use crate::ui::{AppWindow, ACEBox};
use crate::browser::tabs::manager::TabManager;
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
                            let parent_origin = crate::network::security::Origin::from_url(&engine.current_url);
                            rm.fetch(resolved_url, crate::network::resources::ResourceType::Image, parent_origin);
                        }
                    }
                }
            }
            
            // Lógica de Canvas: Se houver dados crus, eles ganham da URL
            if let Some(data) = p.canvas_data {
                if data.len() == (p.width as usize * p.height as usize * 4) {
                    let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&data, p.width as u32, p.height as u32);
                    image_data = slint::Image::from_rgba8_premultiplied(buffer);
                    has_image = true;
                }
            }

            // Lógica de Outline (Borda)
            let is_outline = p.element_type.starts_with("outline-");
            let (final_bg, final_border_color, final_border_width) = if is_outline {
                 // Outline: Fundo transparente, Borda com a cor
                 let border_col = if p.color.starts_with('#') && p.color.len() == 7 {
                    let r = u8::from_str_radix(&p.color[1..3], 16).unwrap_or(255);
                    let g = u8::from_str_radix(&p.color[3..5], 16).unwrap_or(255);
                    let b = u8::from_str_radix(&p.color[5..7], 16).unwrap_or(255);
                    slint::Color::from_rgb_u8(r, g, b)
                } else {
                    slint::Color::from_rgb_u8(0, 100, 255) 
                };
                (slint::Color::from_argb_u8(0, 0, 0, 0), border_col, 2.0)
            } else {
                // Normal: Fundo com a cor, Borda transparente
                (bg_color, slint::Color::from_argb_u8(0, 0, 0, 0), 0.0)
            };

            ACEBox {
                x: p.x,
                y: p.y,
                width: p.width,
                height: p.height,
                background: final_bg,
                text: p.text.into(),
                font_size: p.font_size,
                text_color: slint::Color::from_rgb_u8(0, 0, 0),
                image_data,
                has_image,
                link_url: p.link_url.unwrap_or_default().into(),
                node_id: p.node_idx.to_string().into(),
                element_type: p.element_type.clone().into(),
                is_fixed: p.is_fixed,
                border_width: final_border_width, // Slint expects logical length as f32
                border_color: final_border_color,
                opacity: p.opacity,
                border_radius_tl: p.border_radius[0],
                border_radius_tr: p.border_radius[1],
                border_radius_br: p.border_radius[2],
                border_radius_bl: p.border_radius[3],
                rotate: p.transform_rotate, 
                scale_x: p.transform_scale.0,
                scale_y: p.transform_scale.1,
                // Form Widgets
                is_input: p.element_type == "input",
                is_textarea: p.element_type == "textarea",
                is_select: p.element_type == "select",
                placeholder: p.placeholder.into(),
                input_value: p.input_value.into(),
                input_type: p.input_type.into(),
                options: p.options.into(),
            }
        }).collect();

        let last_y = slint_boxes.last().map(|b| b.y + b.height).unwrap_or(0.0);
        
        let model = std::rc::Rc::new(slint::VecModel::from(slint_boxes));
        ui.set_ace_model(model.into());
        ui.set_content_height(last_y);
        ui.set_viewport_y(engine.viewport_y);
    }
}
