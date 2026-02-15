use crate::ui::{AppWindow, ACEBox};
use crate::tab_manager::TabManager;
use slint::ComponentHandle;

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

            ACEBox {
                x: p.x,
                y: p.y,
                width: p.width,
                height: p.height,
                background: bg_color,
                text: p.text.into(),
                font_size: p.font_size,
                text_color: slint::Color::from_rgb_u8(0, 0, 0), // Força texto preto para testar
                image_data: slint::Image::default(),
                has_image: false,
                link_url: p.link_url.unwrap_or_default().into(),
                node_id: "".into(),
                element_type: p.element_type.into(),
            }
        }).collect();

        // LOG DE DEBUG: Se isso printar > 0, os dados chegaram no Slint
        println!("DEBUG: Enviando {} caixas para a UI", slint_boxes.len());

        let last_y = slint_boxes.last().map(|b| b.y + b.height).unwrap_or(0.0);
        
        let model = std::rc::Rc::new(slint::VecModel::from(slint_boxes));
        ui.set_ace_model(model.into());
        ui.set_content_height(last_y);
    }
}
