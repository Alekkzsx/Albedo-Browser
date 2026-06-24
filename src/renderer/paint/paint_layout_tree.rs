use super::*;
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache};
use tiny_skia::{BlendMode, Color, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};



/// TODO: add docs
pub fn paint_layout_tree(
    tile: &crate::ace::engine::layer_tree::Tile,
    width: u32,
    height: u32,
    scale_factor: f32,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    framebuffer: &mut Option<Pixmap>,
    dirty_rects: &[Rect], // dirty rects are relative to the Tile's logical coordinates
) {
    if framebuffer.is_none()
        || framebuffer.as_ref().expect("Albedo Engine: internal invariant violated").width() != width
        || framebuffer.as_ref().expect("Albedo Engine: internal invariant violated").height() != height
    {
        let mut new_pixmap = Pixmap::new(width.max(1), height.max(1)).expect("Albedo Engine: internal invariant violated");
        new_pixmap.fill(Color::TRANSPARENT); // Tiles need transparent backgrounds for proper alpha blending in wgpu
        *framebuffer = Some(new_pixmap);
    }

    let pixmap = framebuffer.as_mut().expect("Albedo Engine: internal invariant violated");

    let mut clear_paint = Paint::default();
    clear_paint.set_color(Color::TRANSPARENT);
    clear_paint.blend_mode = BlendMode::Source;

    for dirty in dirty_rects {
        if let Some(scaled_dirty) = Rect::from_xywh(
            (dirty.x() - tile.x as f32) * scale_factor,
            (dirty.y() - tile.y as f32) * scale_factor,
            dirty.width() * scale_factor,
            dirty.height() * scale_factor,
        ) {
            pixmap.fill_rect(scaled_dirty, &clear_paint, Transform::identity(), None);
        }
    }

    // To prevent checking the intersection for all sub-commands, we can precompute the bounds.
    for prim in &tile.display_items {
        if prim.width <= 0.0 || prim.height <= 0.0 || prim.opacity <= 0.0 {
            continue;
        }

        // Adjust coordinates relative to the Tile's local space
        let local_x = prim.x - tile.x as f32;
        let local_y = prim.y - tile.y as f32;

        let mut rect = Rect::from_xywh(local_x, local_y, prim.width, prim.height);
        if rect.is_none() {
            continue;
        }

        let mut intersects_dirty = false;
        for dirty in dirty_rects {
            if rect.expect("Albedo Engine: internal invariant violated").intersect(dirty).is_some() {
                intersects_dirty = true;
                break;
            }
        }

        if !intersects_dirty {
            continue;
        }

        // 0. Clip Intersection
        if let Some(cr) = prim.clip_rect {
            if let Some(clip) = Rect::from_xywh(cr[0], cr[1], cr[2], cr[3]) {
                rect = rect.expect("Albedo Engine: internal invariant violated").intersect(&clip);
                if rect.is_none() {
                    continue;
                }
            }
        }

        let rect = rect.expect("Albedo Engine: internal invariant violated");

        // 1. Draw Background Form/Box
        if let Some(mut color) = prim.background_color {
            let mut paint = Paint::default();
            color.set_alpha(color.alpha() * prim.opacity);
            paint.set_color(color);
            pixmap.fill_rect(
                rect,
                &paint,
                Transform::from_scale(scale_factor, scale_factor),
                None,
            );
        }

        // 1.5 Draw Borders
        if prim.border_width > 0.0 {
            if prim.border_style == crate::ace::engine::types::BorderStyle::None {
                // Não desenhar borda se estilo for none
            } else if let Some(mut color) = prim.border_color {
                let mut stroke = tiny_skia::Stroke::default();
                stroke.width = prim.border_width;
                // Aplicar estilo de traço (dashed, dotted, etc.)
                match prim.border_style {
                    crate::ace::engine::types::BorderStyle::Dashed => {
                        stroke.dash = tiny_skia::StrokeDash::new(
                            vec![prim.border_width * 3.0, prim.border_width * 2.0],
                            0.0,
                        );
                    }
                    crate::ace::engine::types::BorderStyle::Dotted => {
                        stroke.dash = tiny_skia::StrokeDash::new(
                            vec![prim.border_width, prim.border_width],
                            0.0,
                        );
                        stroke.line_cap = tiny_skia::LineCap::Round;
                    }
                    _ => {} // Solid e outros => traço contínuo default
                }
                let mut paint = Paint::default();
                color.set_alpha(color.alpha() * prim.opacity);
                paint.set_color(color);
                let path = tiny_skia::PathBuilder::from_rect(rect);
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    Transform::from_scale(scale_factor, scale_factor),
                    None,
                );
            }
        }

        // 2. Draw SVG / Canvas Sub-buffers
        if let Some(data) = &prim.canvas_data {
            if data.len() as u32 == (prim.width as u32) * (prim.height as u32) * 4 {
                if let Some(sub_pixmap) = Pixmap::from_vec(
                    data.clone(),
                    tiny_skia::IntSize::from_wh(prim.width as u32, prim.height as u32).expect("Albedo Engine: internal invariant violated"),
                ) {
                    pixmap.draw_pixmap(
                        (local_x * scale_factor) as i32,
                        (local_y * scale_factor) as i32,
                        sub_pixmap.as_ref(),
                        &PixmapPaint::default(),
                        Transform::from_scale(scale_factor, scale_factor),
                        None,
                    );
                }
            }
        }

        // 3. Draw Text Node
            include!("paint_text.rs");
            }
        }

        // 4. Draw Form Controls (Specialized)
        if prim.element_type == crate::ace::engine::types::ElementRenderType::Input {
            match prim.input_type {
                    include!("paint_form_color.rs");
                }
                    include!("paint_form_range.rs");
                }
                crate::ace::engine::types::FormInputType::Date
                    include!("paint_form_date.rs");
                }
                _ => {}
            }
        }
    }
}
