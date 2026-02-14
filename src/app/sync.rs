use crate::ui::{AppWindow, ACEBox};
use crate::tab_manager::TabManager;
use slint::ComponentHandle;

pub fn sync_ace_visuals(ui: &AppWindow, tm: &TabManager) {
    if let Some((_, Some(engine))) = tm.get_active_tab_native_data() {
        let primitives = engine.render_visual();
        println!("MAIN: Sincronizando UI. Itens: {}", primitives.len());

        let slint_boxes: Vec<ACEBox> = primitives.into_iter().map(|p| {
            
            // 1. DECODIFICADOR DE CORES
            let bg_color = match p.color.as_str() {
                // Cores nomeadas básicas
                "white" | "#ffffff" | "#FFFFFF" => slint::Color::from_rgb_u8(255, 255, 255),
                "black" | "#000000" => slint::Color::from_rgb_u8(0, 0, 0),
                "transparent" => slint::Color::from_argb_u8(0, 0, 0, 0),
                
                // Decodificador HEX (#RRGGBB)
                s if s.starts_with("#") && s.len() == 7 => {
                    let r = u8::from_str_radix(&s[1..3], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&s[3..5], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&s[5..7], 16).unwrap_or(0);
                    slint::Color::from_rgb_u8(r, g, b)
                }
                
                // Fallback (Marca texto rosa para denunciar erro)
                _ => slint::Color::from_rgb_u8(255, 0, 255), 
            };
            
            // 2. CONTRASTE DE TEXTO AUTOMÁTICO
            let txt_color = if p.color == "black" || p.color == "#000000" { 
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
                image_data: slint::Image::default(), 
                has_image: false,
                link_url: p.link_url.unwrap_or_default().into(),
                node_id: "".into(),
                element_type: p.element_type.into(),
            }
        }).collect();

        // 3. CALCULA ALTURA TOTAL DO SCROLL
        let max_height = slint_boxes.last()
            .map(|b| b.y + b.height + 100.0) // +100px de folga no final
            .unwrap_or(800.0);

        // 4. ENVIA PRO SLINT
        let model = std::rc::Rc::new(slint::VecModel::from(slint_boxes));
        ui.set_ace_model(model.into());
        ui.set_content_height(max_height);
    }
}
