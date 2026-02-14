use crate::ui::{AppWindow, ACEBox};
use crate::tab_manager::TabManager;
use slint::ComponentHandle;

pub fn sync_ace_visuals(ui: &AppWindow, tm: &TabManager) {
    if let Some((_, Some(engine))) = tm.get_active_tab_native_data() {
        let primitives = engine.render_visual();
        println!("MAIN: Sincronizando {} primitivos com a UI", primitives.len());

        let slint_boxes: Vec<ACEBox> = primitives.into_iter().map(|p| {
            // Conversor de Cores Seguro
            let bg_color = match p.color.as_str() {
                "white" | "#ffffff" => slint::Color::from_rgb_u8(255, 255, 255),
                "black" | "#000000" => slint::Color::from_rgb_u8(0, 0, 0),
                "blue"  | "#0000FF" => slint::Color::from_rgb_u8(0, 0, 255),
                "red" => slint::Color::from_rgb_u8(255, 0, 0),
                "transparent" => slint::Color::from_argb_u8(0, 0, 0, 0),
                _ => slint::Color::from_argb_u8(0, 0, 0, 0), // Default transparente
            };
            
            // Texto deve ser preto a menos que especificado
            let txt_color = if p.color == "black" { 
                slint::Color::from_rgb_u8(255, 255, 255) 
            } else { 
                slint::Color::from_rgb_u8(0, 0, 0) 
            };

            ACEBox {
                x: p.x, y: p.y, width: p.width, height: p.height,
                background: bg_color,
                text: p.text.into(),
                font_size: p.font_size,
                text_color: txt_color,
                image_data: slint::Image::default(), // Ignora imagens por agora
                has_image: false,
                link_url: p.link_url.unwrap_or_default().into(),
                node_id: "".into(),
                element_type: "text".into(),
            }
        }).collect();

        // Calcular altura total para o Scroll
        let content_height = slint_boxes.last()
            .map(|b| b.y + b.height + 50.0)
            .unwrap_or(0.0);

        let model = std::rc::Rc::new(slint::VecModel::from(slint_boxes));
        ui.set_ace_model(model.into());
        ui.set_content_height(content_height); // Habilita o scroll
    }
}
