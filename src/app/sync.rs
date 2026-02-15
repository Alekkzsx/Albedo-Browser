use crate::ui::{AppWindow, ACEBox};
use crate::tab_manager::TabManager;
use std::time::Duration;
use url::Url;

fn load_image_from_url(url_str: &str, current_url: &str) -> Option<slint::Image> {
    // Resolve relative URLs
    let resolved_url: String = if let Ok(base) = Url::parse(current_url) {
        base.join(url_str).ok()?.to_string()
    } else {
        url_str.to_string()
    };

    let img_url = resolved_url.as_str();

    // Use reqwest to fetch the image
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build() 
    {
        Ok(c) => c,
        Err(_) => return None,
    };

    let response = match client.get(img_url).send() {
        Ok(resp) if resp.status().is_success() => resp,
        _ => return None,
    };

    let bytes = match response.bytes() {
        Ok(b) => b,
        Err(_) => return None,
    };

    // Load image from memory using the image crate
    let img = match image::load_from_memory(&bytes) {
        Ok(i) => i,
        Err(_) => return None,
    };

    // Convert to RGBA8
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let rgba_data = rgba.into_raw();
    
    // Create SharedPixelBuffer for Slint
    let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(width, height);
    {
        let slice = buffer.make_mut_bytes();
        // Image crate gives us RGBA, Slint wants RGBA too, so just copy
        slice.copy_from_slice(&rgba_data);
    }

    Some(slint::Image::from_rgba8_premultiplied(buffer))
}

pub fn sync_ace_visuals(ui: &AppWindow, tm: &TabManager) {
    if let Some((_, Some(engine))) = tm.get_active_tab_native_data() {
        let primitives = engine.render_visual();
        
        let slint_boxes: Vec<ACEBox> = primitives.into_iter().map(|p| {
            // Conversor Hexadecimal Robusto
            let bg_color = if p.color.starts_with('#') && p.color.len() == 7 {
                let r = u8::from_str_radix(&p.color[1..3], 16).unwrap_or(255);
                let g = u8::from_str_radix(&p.color[3..5], 16).unwrap_or(255);
                let b = u8::from_str_radix(&p.color[5..7], 16).unwrap_or(255);
                slint::Color::from_rgb_u8(r, g, b)
            } else {
                // Fallback se não for hex: Verde limão para denunciar o erro!
                slint::Color::from_rgb_u8(50, 255, 50) 
            };

            // Check if this is an image element
            let is_image = p.element_type == "image";
            let (image_data, has_image) = if is_image {
                if let Some(ref img_url) = p.image_url {
                    let img = load_image_from_url(img_url, &engine.current_url);
                    if let Some(i) = img {
                        (i, true)
                    } else {
                        (slint::Image::default(), false)
                    }
                } else {
                    (slint::Image::default(), false)
                }
            } else {
                (slint::Image::default(), false)
            };

            ACEBox {
                x: p.x,
                y: p.y,
                width: p.width,
                height: p.height,
                background: bg_color,
                text: p.text.into(),
                font_size: p.font_size,
                text_color: slint::Color::from_rgb_u8(0, 0, 0), // Força texto preto para testar
                image_data,
                has_image,
                link_url: p.link_url.unwrap_or_default().into(),
                node_id: "".into(),
                element_type: p.element_type.into(),
            }
        }).collect();

        let last_y = slint_boxes.last().map(|b| b.y + b.height).unwrap_or(0.0);
        
        let model = std::rc::Rc::new(slint::VecModel::from(slint_boxes));
        ui.set_ace_model(model.into());
        ui.set_content_height(last_y);
    }
}
