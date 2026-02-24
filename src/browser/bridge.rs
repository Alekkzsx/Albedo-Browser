use crate::ui::{AppWindow, ACEBox};
use crate::browser::tabs::manager::TabManager;
use std::time::Duration;
use url::Url;
use slint::ComponentHandle;


pub fn sync_ace_visuals(ui: &AppWindow, tm: &TabManager) {
    if let Some((_, Some(engine), _)) = tm.get_active_tab_native_data() {
        let vw = (ui.window().size().width as f32).max(800.0);
        let vh = (ui.window().size().height as f32).max(600.0);
        let primitives = engine.render_visual(vw, vh);
        let cache = engine.image_cache.lock().unwrap();
        
        let mut slint_boxes: Vec<ACEBox> = Vec::new();
        let mut quad_instances = Vec::new();
        
        println!("[Bridge] Engine build {} primitives.", primitives.len());
        // --- 1. Passagem: Coletar Instâncias WGPU e Texts para Slint ---
        for p in &primitives {
            let is_image = p.element_type == "image";
            // Outline (borda) = fundo transparente, mas para simplificar nosso WGPU recém feito, desenhamos com color do border.
            // Para "perfeição" teríamos width customizado na GPU, por agora faremos WGPU de backgrounds e Slint outlines.
            let is_outline = p.element_type.starts_with("outline-");
            
            // Decoração em CPU para Color parsing
            let mut bg_color = slint::Color::from_argb_u8(0,0,0,0);
            let mut wgpu_color = [0.0, 0.0, 0.0, 0.0];
            
            if !is_outline {
                if p.color.starts_with('#') && (p.color.len() == 7 || p.color.len() == 9) {
                    let r = u8::from_str_radix(&p.color[1..3], 16).unwrap_or(255);
                    let g = u8::from_str_radix(&p.color[3..5], 16).unwrap_or(255);
                    let b = u8::from_str_radix(&p.color[5..7], 16).unwrap_or(255);
                    let a = if p.color.len() == 9 { u8::from_str_radix(&p.color[7..9], 16).unwrap_or(255) } else { 255 };
                    
                    bg_color = slint::Color::from_argb_u8(a, r, g, b);
                    wgpu_color = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                }
            }
            
            // Se for background sólido, e não for Form Widget ou Imagem fixa, nós injetamos na GPU.
            let is_form = p.element_type == "input" || p.element_type == "textarea" || p.element_type == "select";
            
            if !is_image && !is_outline && bg_color.alpha() > 0 && !is_form {
                // Background normal -> vai pra GPU
                quad_instances.push(crate::engine::graphics::gpu::QuadInstance {
                    position: [p.x, p.y],
                    size: [p.width, p.height],
                    color: wgpu_color,
                    border_radius: [p.border_radius[0], p.border_radius[1], p.border_radius[2], p.border_radius[3]],
                    z_index: p.node_idx as f32,
                    _pad: [0.0; 3]
                });
            }

            // Lógica de Imagem Assíncrona via Cache (Slint Raster)
            let mut image_data = slint::Image::default();
            let mut has_image = false;

            if is_image {
                if let Some(ref img_url) = p.image_url {
                    // Resolve URL para chave do cache
                    let resolved_url = if let Ok(base) = Url::parse(&engine.current_url) {
                        base.join(img_url).ok().map(|u| u.to_string()).unwrap_or(img_url.to_string())
                    } else {
                        img_url.to_string()
                    };

                    if let Some(img) = cache.get(&resolved_url) {
                        image_data = img.clone();
                        has_image = true;
                    } else {
                        if let Some(ref rm) = engine.resource_manager {
                            let parent_origin = crate::network::security::Origin::from_url(&engine.current_url);
                            rm.fetch(resolved_url, crate::network::resources::ResourceType::Image, parent_origin);
                        }
                    }
                }
            }
            
            if let Some(data) = &p.canvas_data {
                if data.len() == (p.width as usize * p.height as usize * 4) {
                    let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(data, p.width as u32, p.height as u32);
                    image_data = slint::Image::from_rgba8_premultiplied(buffer);
                    has_image = true;
                }
            }

            // Lógica de Outline
            let (final_border_color, final_border_width) = if is_outline {
                 let border_col = if p.color.starts_with('#') && p.color.len() == 7 {
                    let r = u8::from_str_radix(&p.color[1..3], 16).unwrap_or(255);
                    let g = u8::from_str_radix(&p.color[3..5], 16).unwrap_or(255);
                    let b = u8::from_str_radix(&p.color[5..7], 16).unwrap_or(255);
                    slint::Color::from_rgb_u8(r, g, b)
                } else {
                    slint::Color::from_rgb_u8(0, 0, 0) 
                };
                (border_col, 2.0)
            } else {
                (slint::Color::from_argb_u8(0, 0, 0, 0), 0.0)
            };
            
            // Só mandamos o box pro Slint se ele tiver conteúdo que o Slint deva manejar:
            // Textos, Imagens, Outlines, Form Widgets e Backgrounds sólidos (enquanto WGPU opcional).
            let has_bg = bg_color.alpha() > 0;
            let needs_slint = p.text.len() > 0 || has_image || is_outline || is_form || has_bg;
            
            if needs_slint {
                slint_boxes.push(ACEBox {
                    x: p.x,
                    y: p.y,
                    width: p.width,
                    height: p.height,
                    background: bg_color, // Sempre passar a cor para o Slint agora
                    text: p.text.clone().into(),
                    text_overflow: p.text_overflow.clone().into(),
                    font_size: p.font_size,
                    text_color: slint::Color::from_rgb_u8(0, 0, 0),
                    image_data,
                    has_image,
                    link_url: p.link_url.clone().unwrap_or_default().into(),
                    node_id: p.node_idx.to_string().into(),
                    element_type: p.element_type.clone().into(),
                    is_fixed: p.is_fixed,
                    border_width: final_border_width,
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
                    placeholder: p.placeholder.clone().into(),
                    input_value: p.input_value.clone().into(),
                    input_type: p.input_type.clone().into(),
                    options: p.options.clone().into(),
                });
            }
        }

        let last_y = primitives.last().map(|p| p.y + p.height).unwrap_or(0.0);
        
        // --- 2. Passagem: WGPU Render assíncrono para o Background global ---
        // Aqui iniciaríamos a Tokio task e jogaríamos pra engine, mas por enquanto, dado
        // a sincronia do timer Slint neste ponto, iremos invocar WGPU se tivermos instâncias.
        // Como o WGPU retorna bytes, podemos criar um ACEBox gigante para baixo z-index injetado na Slint.
        if !quad_instances.is_empty() {
             let bounds_w = (ui.window().size().width as f32).max(800.0) as u32;
             let bounds_h = (ui.window().size().height as f32).max(600.0) as u32;
             
             // TODO: O WGPU exige processamento assíncrono. Em producao usariamos callback slint::invoke_from_event_loop
             // Chamar rt.block_on aqui, dentro do loop de eventos Slint / tokio enter causa 
             // "Cannot start a runtime from within a runtime" e Crash fatal (0xcfffffff). 
             // Temporariamente ignorado até as streams assíncronas de WGPU para Slint Image_data ficarem prontas.
             eprintln!("[Bridge] Ignorando WGPU render com {} quadrantes (Tokio block_on panic workaround).", quad_instances.len());
        }
        
        let model = std::rc::Rc::new(slint::VecModel::from(slint_boxes));
        ui.set_ace_model(model.into());
        ui.set_content_height(last_y);
        ui.set_viewport_y(engine.viewport_y);
    }
}
