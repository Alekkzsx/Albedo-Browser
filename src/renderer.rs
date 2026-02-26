use tiny_skia::{Pixmap, Paint, Rect, Transform, Color, BlendMode, PixmapPaint};
use crate::engine::VisualPrimitive;
use cosmic_text::{FontSystem, Buffer, Metrics, Attrs, Shaping, SwashCache};

pub fn paint_layout_tree(primitives: &[VisualPrimitive], width: u32, height: u32, scale_factor: f32, font_system: &mut FontSystem) -> Pixmap {
    let mut pixmap = Pixmap::new(width.max(1), height.max(1)).unwrap();
    pixmap.fill(Color::WHITE); // Default background

    let mut swash_cache = SwashCache::new();

    for prim in primitives {
        if prim.width <= 0.0 || prim.height <= 0.0 || prim.opacity <= 0.0 {
            continue;
        }

        let rect = Rect::from_xywh(prim.x, prim.y, prim.width, prim.height);
        if rect.is_none() { continue; }
        let rect = rect.unwrap();

        // 1. Draw Background Form/Box
        if let Some(mut color) = prim.background_color {
            let mut paint = Paint::default();
            color.set_alpha(color.alpha() * prim.opacity);
            paint.set_color(color);
            pixmap.fill_rect(rect, &paint, Transform::from_scale(scale_factor, scale_factor), None);
        }

        // 1.5 Draw Borders
        if prim.border_width > 0.0 {
            if let Some(mut color) = prim.border_color {
                let mut stroke = tiny_skia::Stroke::default();
                stroke.width = prim.border_width;
                let mut paint = Paint::default();
                color.set_alpha(color.alpha() * prim.opacity);
                paint.set_color(color);
                let path = tiny_skia::PathBuilder::from_rect(rect);
                pixmap.stroke_path(&path, &paint, &stroke, Transform::from_scale(scale_factor, scale_factor), None);
            }
        }

        // 2. Draw SVG / Canvas Sub-buffers
        if let Some(data) = &prim.canvas_data {
            if data.len() as u32 == (prim.width as u32) * (prim.height as u32) * 4 {
                if let Some(sub_pixmap) = Pixmap::from_vec(
                    data.clone(), 
                    tiny_skia::IntSize::from_wh(prim.width as u32, prim.height as u32).unwrap()
                ) {
                    pixmap.draw_pixmap(
                        (prim.x * scale_factor) as i32, 
                        (prim.y * scale_factor) as i32, 
                        sub_pixmap.as_ref(), 
                        &PixmapPaint::default(), 
                        Transform::from_scale(scale_factor, scale_factor), 
                        None
                    );
                }
            }
        }

        // 3. Draw Text Node
        if let Some(text) = &prim.text_content {
            let mut buffer = Buffer::new(font_system, Metrics::new(prim.font_size * scale_factor, prim.font_size * 1.2 * scale_factor));
            buffer.set_size(font_system, Some(prim.width * scale_factor), Some(prim.height * scale_factor));
             
            let attrs = Attrs::new(); // TODO: apply text_color and font_family
            let text_color = prim.text_color;
             
            buffer.set_text(font_system, text, attrs, Shaping::Advanced);
            buffer.shape_until_scroll(font_system, false);
             
            let r_base = (text_color.red() * 255.0) as u32;
            let g_base = (text_color.green() * 255.0) as u32;
            let b_base = (text_color.blue() * 255.0) as u32;

            for run in buffer.layout_runs() {
                for glyph in run.glyphs.iter() {
                    let physical_glyph = glyph.physical((prim.x * scale_factor, prim.y * scale_factor), 1.0);
                    if let Some(image) = swash_cache.get_image(font_system, physical_glyph.cache_key) {
                        let top = (physical_glyph.y as i32) - image.placement.top;
                        let left = (physical_glyph.x as i32) + image.placement.left;
                        
                        match image.content {
                            cosmic_text::SwashContent::Mask => { // Anti-aliased text (alpha channel only)
                                for (y, row) in image.data.chunks(image.placement.width as usize).enumerate() {
                                    for (x, alpha_byte) in row.iter().enumerate() {
                                        let p_x = left as i32 + x as i32;
                                        let p_y = top as i32 + y as i32;
                                        if p_x >= 0 && p_x < width as i32 && p_y >= 0 && p_y < height as i32 {
                                            if *alpha_byte > 0 {
                                                let pixel_idx = ((p_y as u32 * width) + p_x as u32) as usize * 4;
                                                let data = pixmap.data_mut();
                                                // Simple alpha blending
                                                let src_a = (*alpha_byte as f32 / 255.0) * text_color.alpha() * prim.opacity;
                                                let dst_a = 1.0 - src_a;
                                                
                                                data[pixel_idx] = ((r_base as f32 * src_a) + (data[pixel_idx] as f32 * dst_a)) as u8;
                                                data[pixel_idx+1] = ((g_base as f32 * src_a) + (data[pixel_idx+1] as f32 * dst_a)) as u8;
                                                data[pixel_idx+2] = ((b_base as f32 * src_a) + (data[pixel_idx+2] as f32 * dst_a)) as u8;
                                                data[pixel_idx+3] = 255;
                                            }
                                        }
                                    }
                                }
                            }
                            cosmic_text::SwashContent::Color => { // Emojis / Color fonts
                                for (y, row) in image.data.chunks(image.placement.width as usize * 4).enumerate() {
                                    for (x, pixel) in row.chunks(4).enumerate() {
                                        let p_x = left as i32 + x as i32;
                                        let p_y = top as i32 + y as i32;
                                        if p_x >= 0 && p_x < width as i32 && p_y >= 0 && p_y < height as i32 {
                                            let src_a = (pixel[3] as f32 / 255.0) * prim.opacity;
                                            if src_a > 0.0 {
                                                let pixel_idx = ((p_y as u32 * width) + p_x as u32) as usize * 4;
                                                let data = pixmap.data_mut();
                                                let dst_a = 1.0 - src_a;
                                                
                                                data[pixel_idx] = ((pixel[0] as f32 * src_a) + (data[pixel_idx] as f32 * dst_a)) as u8;
                                                data[pixel_idx+1] = ((pixel[1] as f32 * src_a) + (data[pixel_idx+1] as f32 * dst_a)) as u8;
                                                data[pixel_idx+2] = ((pixel[2] as f32 * src_a) + (data[pixel_idx+2] as f32 * dst_a)) as u8;
                                                data[pixel_idx+3] = 255;
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    
    pixmap
}
