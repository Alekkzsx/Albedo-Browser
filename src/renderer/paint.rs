use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache};
use tiny_skia::{BlendMode, Color, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};

fn blend_mask_pixel(
    data: &mut [u8],
    pixel_idx: usize,
    alpha_byte: u8,
    opacity: f32,
    color: (f32, f32, f32),
) {
    let src_a = (alpha_byte as f32 / 255.0) * opacity;
    if src_a > 0.0 {
        let dst_a = 1.0 - src_a;
        data[pixel_idx] = (color.0 * src_a + data[pixel_idx] as f32 * dst_a) as u8;
        data[pixel_idx + 1] = (color.1 * src_a + data[pixel_idx + 1] as f32 * dst_a) as u8;
        data[pixel_idx + 2] = (color.2 * src_a + data[pixel_idx + 2] as f32 * dst_a) as u8;
        data[pixel_idx + 3] = 255;
    }
}

fn blend_color_pixel(data: &mut [u8], pixel_idx: usize, src_pixel: &[u8], opacity: f32) {
    let src_a = (src_pixel[3] as f32 / 255.0) * opacity;
    if src_a > 0.0 {
        let dst_a = 1.0 - src_a;
        data[pixel_idx] = (src_pixel[0] as f32 * src_a + data[pixel_idx] as f32 * dst_a) as u8;
        data[pixel_idx + 1] =
            (src_pixel[1] as f32 * src_a + data[pixel_idx + 1] as f32 * dst_a) as u8;
        data[pixel_idx + 2] =
            (src_pixel[2] as f32 * src_a + data[pixel_idx + 2] as f32 * dst_a) as u8;
        data[pixel_idx + 3] = 255;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_mask_pixel_basic() {
        let mut data = vec![128u8; 8]; // 2 pixels, RGBA
        blend_mask_pixel(&mut data, 0, 128, 1.0, (255.0, 0.0, 0.0));
        // src_a = 128/255 * 1.0 ≈ 0.502
        assert!(data[0] > 128); // red channel should increase
        assert_eq!(data[1], 128); // green unchanged (0.0 * src_a = 0)
        assert_eq!(data[2], 128); // blue unchanged
        assert_eq!(data[3], 255); // alpha always 255
    }

    #[test]
    fn blend_mask_pixel_zero_alpha() {
        let mut data = vec![128u8; 4];
        blend_mask_pixel(&mut data, 0, 0, 1.0, (255.0, 0.0, 0.0));
        assert_eq!(data, vec![128, 128, 128, 128]); // no change
    }

    #[test]
    fn blend_color_pixel_basic() {
        let mut data = vec![0u8; 8]; // 2 pixels RGBA
        let src = vec![255u8, 128, 64, 255]; // fully opaque source pixel
        blend_color_pixel(&mut data, 0, &src, 1.0);
        assert_eq!(data[0], 255); // red
        assert_eq!(data[1], 128); // green
        assert_eq!(data[2], 64); // blue
        assert_eq!(data[3], 255); // alpha
    }

    #[test]
    fn blend_color_pixel_semi_transparent() {
        let mut data = vec![200u8; 4];
        let src = vec![100u8, 100, 100, 128]; // ~50% alpha
        blend_color_pixel(&mut data, 0, &src, 1.0);
        // src_a = 128/255 ≈ 0.502
        assert!(data[0] > 100 && data[0] < 200);
    }
}

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
        || framebuffer.as_ref().unwrap().width() != width
        || framebuffer.as_ref().unwrap().height() != height
    {
        let mut new_pixmap = Pixmap::new(width.max(1), height.max(1)).unwrap();
        new_pixmap.fill(Color::TRANSPARENT); // Tiles need transparent backgrounds for proper alpha blending in wgpu
        *framebuffer = Some(new_pixmap);
    }

    let pixmap = framebuffer.as_mut().unwrap();

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
            if rect.unwrap().intersect(dirty).is_some() {
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
                rect = rect.unwrap().intersect(&clip);
                if rect.is_none() {
                    continue;
                }
            }
        }

        let rect = rect.unwrap();

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
                    tiny_skia::IntSize::from_wh(prim.width as u32, prim.height as u32).unwrap(),
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
        if let Some(text) = &prim.text_content {
            let font_size = prim.font_size * scale_factor;
            let line_height = prim.font_size * 1.2 * scale_factor;
            let mut buffer = Buffer::new(font_system, Metrics::new(font_size, line_height));

            let attrs = Attrs::new(); // TODO: apply css font_family/weight
            let text_color = prim.text_color;

            let r_base = (text_color.red() * 255.0) as u32;
            let g_base = (text_color.green() * 255.0) as u32;
            let b_base = (text_color.blue() * 255.0) as u32;
            let max_w = prim.width * scale_factor;

            // Determinar limites de clipping absolutos para os pixels do texto
            let (clip_min_x, clip_min_y, clip_max_x, clip_max_y) = if let Some(cr) = prim.clip_rect
            {
                (
                    (cr[0] * scale_factor) as i32,
                    (cr[1] * scale_factor) as i32,
                    ((cr[0] + cr[2]) * scale_factor) as i32,
                    ((cr[1] + cr[3]) * scale_factor) as i32,
                )
            } else {
                (0, 0, width as i32, height as i32)
            };

            let render_glyph_image = |swash_cache: &mut SwashCache,
                                      font_system: &mut FontSystem,
                                      physical_glyph: cosmic_text::PhysicalGlyph,
                                      pixmap: &mut Pixmap| {
                if let Some(image) = swash_cache.get_image(font_system, physical_glyph.cache_key) {
                    let top = (physical_glyph.y as i32) - image.placement.top;
                    let left = (physical_glyph.x as i32) + image.placement.left;
                    match image.content {
                        cosmic_text::SwashContent::Mask => {
                            let text_opacity = text_color.alpha() * prim.opacity;
                            let color = (r_base as f32, g_base as f32, b_base as f32);
                            for (y, row) in image
                                .data
                                .chunks(image.placement.width as usize)
                                .enumerate()
                            {
                                for (x, alpha_byte) in row.iter().enumerate() {
                                    let p_x = left as i32 + x as i32;
                                    let p_y = top as i32 + y as i32;
                                    if p_x >= clip_min_x
                                        && p_x < clip_max_x
                                        && p_y >= clip_min_y
                                        && p_y < clip_max_y
                                    {
                                        let pixel_idx =
                                            ((p_y as u32 * width) + p_x as u32) as usize * 4;
                                        blend_mask_pixel(
                                            pixmap.data_mut(),
                                            pixel_idx,
                                            *alpha_byte,
                                            text_opacity,
                                            color,
                                        );
                                    }
                                }
                            }
                        }
                        cosmic_text::SwashContent::Color => {
                            for (y, row) in image
                                .data
                                .chunks(image.placement.width as usize * 4)
                                .enumerate()
                            {
                                for (x, pixel) in row.chunks(4).enumerate() {
                                    let p_x = left as i32 + x as i32;
                                    let p_y = top as i32 + y as i32;
                                    if p_x >= clip_min_x
                                        && p_x < clip_max_x
                                        && p_y >= clip_min_y
                                        && p_y < clip_max_y
                                    {
                                        let pixel_idx =
                                            ((p_y as u32 * width) + p_x as u32) as usize * 4;
                                        blend_color_pixel(
                                            pixmap.data_mut(),
                                            pixel_idx,
                                            pixel,
                                            prim.opacity,
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            };

            let letter_spacing = prim.letter_spacing * scale_factor;
            let word_spacing = prim.word_spacing * scale_factor;

            if letter_spacing == 0.0 && word_spacing == 0.0 {
                // Fast Path
                buffer.set_size(font_system, Some(max_w), Some(prim.height * scale_factor));
                buffer.set_text(font_system, text, attrs, Shaping::Advanced);
                buffer.shape_until_scroll(font_system, false);
                for run in buffer.layout_runs() {
                    for glyph in run.glyphs.iter() {
                        let physical_glyph =
                            glyph.physical((local_x * scale_factor, local_y * scale_factor), 1.0);
                        render_glyph_image(swash_cache, font_system, physical_glyph, pixmap);
                    }
                }
            } else {
                // Spaced Path (Tokenized Greedy Wrapping & Manual Rendering)
                let mut current_x = 0.0;
                let mut current_y = 0.0; // Starts relative to top

                let mut space_w = 0.0;
                buffer.set_text(font_system, " ", attrs, Shaping::Advanced);
                buffer.shape_until_scroll(font_system, false);
                if let Some(run) = buffer.layout_runs().next() {
                    space_w = run.line_w;
                }

                let mut first_word_in_line = true;

                for word in text.split_inclusive(|c: char| c.is_whitespace()) {
                    let is_space_ended = word.ends_with(|c: char| c.is_whitespace());
                    let (word_trim, trailing) = if is_space_ended {
                        let bytes = word.len() - word.chars().last().unwrap().len_utf8();
                        (&word[..bytes], &word[bytes..])
                    } else {
                        (word, "")
                    };

                    let mut word_w = 0.0;
                    if letter_spacing != 0.0 {
                        for c in word_trim.chars() {
                            let mut b = [0; 4];
                            let char_str = c.encode_utf8(&mut b);
                            buffer.set_text(font_system, char_str, attrs, Shaping::Advanced);
                            buffer.shape_until_scroll(font_system, false);
                            word_w += buffer.layout_runs().next().map_or(0.0, |r| r.line_w)
                                + letter_spacing;
                        }
                    } else {
                        buffer.set_text(font_system, word_trim, attrs, Shaping::Advanced);
                        buffer.shape_until_scroll(font_system, false);
                        word_w = buffer.layout_runs().next().map_or(0.0, |r| r.line_w);
                    }

                    if !first_word_in_line && current_x + word_w > max_w {
                        current_x = 0.0;
                        current_y += line_height;
                    }

                    // Render the word
                    if letter_spacing != 0.0 {
                        let mut cx = current_x;
                        for c in word_trim.chars() {
                            let mut b = [0; 4];
                            let char_str = c.encode_utf8(&mut b);
                            buffer.set_text(font_system, char_str, attrs, Shaping::Advanced);
                            buffer.shape_until_scroll(font_system, false);
                            for run in buffer.layout_runs() {
                                for glyph in run.glyphs.iter() {
                                    let physical_glyph = glyph.physical(
                                        (
                                            (local_x + cx) * scale_factor,
                                            (local_y + current_y) * scale_factor,
                                        ),
                                        1.0,
                                    );
                                    render_glyph_image(
                                        swash_cache,
                                        font_system,
                                        physical_glyph,
                                        pixmap,
                                    );
                                }
                            }
                            cx += buffer.layout_runs().next().map_or(0.0, |r| r.line_w)
                                + letter_spacing;
                        }
                        current_x = cx;
                    } else {
                        buffer.set_text(font_system, word_trim, attrs, Shaping::Advanced);
                        buffer.shape_until_scroll(font_system, false);
                        for run in buffer.layout_runs() {
                            for glyph in run.glyphs.iter() {
                                let physical_glyph = glyph.physical(
                                    (
                                        local_x * scale_factor + current_x,
                                        local_y * scale_factor + current_y,
                                    ),
                                    1.0,
                                );
                                render_glyph_image(
                                    swash_cache,
                                    font_system,
                                    physical_glyph,
                                    pixmap,
                                );
                            }
                        }
                        current_x += word_w;
                    }

                    if is_space_ended {
                        let mut spc_adv = space_w;
                        if letter_spacing != 0.0 {
                            let mut b = [0; 4];
                            let c = trailing.chars().next().unwrap();
                            let char_str = c.encode_utf8(&mut b);
                            buffer.set_text(font_system, char_str, attrs, Shaping::Advanced);
                            buffer.shape_until_scroll(font_system, false);
                            // Also render space if it had visual meaning, but space is invisible Mask
                            for run in buffer.layout_runs() {
                                for glyph in run.glyphs.iter() {
                                    let physical_glyph = glyph.physical(
                                        (
                                            local_x * scale_factor + current_x,
                                            local_y * scale_factor + current_y,
                                        ),
                                        1.0,
                                    );
                                    render_glyph_image(
                                        swash_cache,
                                        font_system,
                                        physical_glyph,
                                        pixmap,
                                    );
                                }
                            }
                            spc_adv = buffer.layout_runs().next().map_or(0.0, |r| r.line_w);
                        }
                        current_x += spc_adv + word_spacing + letter_spacing;
                    }

                    first_word_in_line = false;
                    let _ = first_word_in_line;
                }
            }
        }

        // 4. Draw Form Controls (Specialized)
        if prim.element_type == crate::ace::engine::types::ElementRenderType::Input {
            match prim.input_type {
                crate::ace::engine::types::FormInputType::Color => {
                    // Premium Color Button: Rounded and with a "chip" look
                    if let Ok(color) = crate::utils::color::parse_hex_color(&prim.input_value) {
                        let mut paint = Paint::default();
                        paint.set_color(color);
                        paint.anti_alias = true;

                        let inset = 4.0;
                        let _corner_radius = 4.0;

                        // Draw shadow/depth effect
                        let mut shadow_paint = Paint::default();
                        shadow_paint.set_color(Color::from_rgba8(0, 0, 0, 40));
                        if let Some(shadow_rect) = Rect::from_xywh(
                            local_x + inset,
                            local_y + inset + 1.0,
                            (prim.width - inset * 2.0).max(0.0),
                            (prim.height - inset * 2.0).max(0.0),
                        ) {
                            let shadow_path = PathBuilder::from_rect(shadow_rect);
                            pixmap.fill_path(
                                &shadow_path,
                                &shadow_paint,
                                tiny_skia::FillRule::Winding,
                                Transform::from_scale(scale_factor, scale_factor),
                                None,
                            );
                        }

                        if let Some(inner_rect) = Rect::from_xywh(
                            local_x + inset,
                            local_y + inset,
                            (prim.width - inset * 2.0).max(0.0),
                            (prim.height - inset * 2.0).max(0.0),
                        ) {
                            let path = PathBuilder::from_rect(inner_rect);
                            pixmap.fill_path(
                                &path,
                                &paint,
                                tiny_skia::FillRule::Winding,
                                Transform::from_scale(scale_factor, scale_factor),
                                None,
                            );

                            // Highlight on hover
                            if prim.is_hovered {
                                let mut highlight = Paint::default();
                                highlight.set_color(Color::from_rgba8(255, 255, 255, 60));
                                pixmap.fill_path(
                                    &path,
                                    &highlight,
                                    tiny_skia::FillRule::Winding,
                                    Transform::from_scale(scale_factor, scale_factor),
                                    None,
                                );
                            }
                        }
                    }
                }
                crate::ace::engine::types::FormInputType::Range => {
                    // Modern Slider: Gradient track and circular thumb
                    let track_h = 6.0;
                    let track_y = local_y + (prim.height - track_h) / 2.0;

                    // Track with gradient
                    let mut track_paint = Paint::default();
                    let shader = tiny_skia::LinearGradient::new(
                        tiny_skia::Point::from_xy(local_x, track_y),
                        tiny_skia::Point::from_xy(local_x + prim.width, track_y),
                        vec![
                            tiny_skia::GradientStop::new(
                                0.0,
                                Color::from_rgba8(220, 220, 220, 255),
                            ),
                            tiny_skia::GradientStop::new(
                                1.0,
                                Color::from_rgba8(180, 180, 180, 255),
                            ),
                        ],
                        tiny_skia::SpreadMode::Pad,
                        Transform::identity(),
                    )
                    .unwrap();
                    track_paint.shader = shader;
                    track_paint.anti_alias = true;

                    if let Some(track_rect) = Rect::from_xywh(local_x, track_y, prim.width, track_h)
                    {
                        let track_path = PathBuilder::from_rect(track_rect);
                        pixmap.fill_path(
                            &track_path,
                            &track_paint,
                            tiny_skia::FillRule::Winding,
                            Transform::from_scale(scale_factor, scale_factor),
                            None,
                        );
                    }

                    // Logic for positioning
                    let min: f32 = prim.input_min.parse().unwrap_or(0.0);
                    let max: f32 = prim.input_max.parse().unwrap_or(100.0);
                    let val: f32 = prim.input_value.parse().unwrap_or((min + max) / 2.0);
                    let percent = if max > min {
                        (val - min) / (max - min)
                    } else {
                        0.5
                    };

                    let thumb_radius = 9.0;
                    let thumb_x = local_x
                        + (prim.width - thumb_radius * 2.0) * percent.clamp(0.0, 1.0)
                        + thumb_radius;
                    let thumb_y = local_y + prim.height / 2.0;

                    // Thumb Shadow
                    let mut shadow_paint = Paint::default();
                    shadow_paint.set_color(Color::from_rgba8(0, 0, 0, 60));
                    shadow_paint.anti_alias = true;
                    let mut pb = tiny_skia::PathBuilder::new();
                    pb.push_circle(thumb_x, thumb_y + 1.0, thumb_radius);
                    if let Some(shadow_path) = pb.finish() {
                        pixmap.fill_path(
                            &shadow_path,
                            &shadow_paint,
                            tiny_skia::FillRule::Winding,
                            Transform::from_scale(scale_factor, scale_factor),
                            None,
                        );
                    }

                    // Thumb
                    let mut thumb_paint = Paint::default();
                    let thumb_color = if prim.is_hovered {
                        Color::from_rgba8(0, 100, 200, 255)
                    } else {
                        Color::from_rgba8(0, 120, 215, 255)
                    };
                    thumb_paint.set_color(thumb_color);
                    thumb_paint.anti_alias = true;

                    let mut pb = tiny_skia::PathBuilder::new();
                    pb.push_circle(thumb_x, thumb_y, thumb_radius);
                    if let Some(path) = pb.finish() {
                        pixmap.fill_path(
                            &path,
                            &thumb_paint,
                            tiny_skia::FillRule::Winding,
                            Transform::from_scale(scale_factor, scale_factor),
                            None,
                        );

                        // Thumb Border
                        let mut stroke = tiny_skia::Stroke::default();
                        stroke.width = 2.0;
                        let mut stroke_paint = Paint::default();
                        stroke_paint.set_color(Color::WHITE);
                        pixmap.stroke_path(
                            &path,
                            &stroke_paint,
                            &stroke,
                            Transform::from_scale(scale_factor, scale_factor),
                            None,
                        );
                    }
                }
                crate::ace::engine::types::FormInputType::Date
                | crate::ace::engine::types::FormInputType::Time => {
                    // Date/Time with Icon and polished text
                    let is_date = prim.input_type == crate::ace::engine::types::FormInputType::Date;

                    // Draw Icon on the right
                    let icon_size = 14.0;
                    let icon_x = local_x + prim.width - icon_size - 6.0;
                    let icon_y = local_y + (prim.height - icon_size) / 2.0;

                    let mut icon_paint = Paint::default();
                    icon_paint.set_color(Color::from_rgba8(100, 100, 100, 255));
                    icon_paint.anti_alias = true;

                    let mut pb = tiny_skia::PathBuilder::new();
                    if is_date {
                        // Calendar Icon
                        pb.move_to(icon_x, icon_y + 2.0);
                        pb.line_to(icon_x + icon_size, icon_y + 2.0);
                        pb.line_to(icon_x + icon_size, icon_y + icon_size);
                        pb.line_to(icon_x, icon_y + icon_size);
                        pb.close();
                        // Top bar
                        pb.move_to(icon_x, icon_y + 4.0);
                        pb.line_to(icon_x + icon_size, icon_y + 4.0);
                    } else {
                        // Clock Icon
                        pb.push_circle(
                            icon_x + icon_size / 2.0,
                            icon_y + icon_size / 2.0,
                            icon_size / 2.0,
                        );
                        pb.move_to(icon_x + icon_size / 2.0, icon_y + icon_size / 2.0);
                        pb.line_to(icon_x + icon_size / 2.0, icon_y + 4.0);
                        pb.move_to(icon_x + icon_size / 2.0, icon_y + icon_size / 2.0);
                        pb.line_to(icon_x + icon_size - 4.0, icon_y + icon_size / 2.0);
                    }

                    if let Some(path) = pb.finish() {
                        let mut stroke = tiny_skia::Stroke::default();
                        stroke.width = 1.5;
                        pixmap.stroke_path(
                            &path,
                            &icon_paint,
                            &stroke,
                            Transform::from_scale(scale_factor, scale_factor),
                            None,
                        );
                    }

                    // Value Text
                    if !prim.input_value.is_empty() {
                        let value_color = prim.text_color;
                        let vr_base = (value_color.red() * 255.0) as f32;
                        let vg_base = (value_color.green() * 255.0) as f32;
                        let vb_base = (value_color.blue() * 255.0) as f32;
                        let value_opacity = value_color.alpha() * prim.opacity;

                        let mut buffer = Buffer::new(
                            font_system,
                            Metrics::new(
                                prim.font_size * scale_factor,
                                prim.font_size * 1.2 * scale_factor,
                            ),
                        );
                        buffer.set_size(
                            font_system,
                            Some((prim.width - icon_size - 10.0) * scale_factor),
                            Some(prim.height * scale_factor),
                        );
                        buffer.set_text(
                            font_system,
                            &prim.input_value,
                            Attrs::new(),
                            Shaping::Advanced,
                        );
                        buffer.shape_until_scroll(font_system, false);

                        for run in buffer.layout_runs() {
                            for glyph in run.glyphs.iter() {
                                let physical_glyph = glyph.physical(
                                    (
                                        local_x * scale_factor + 6.0,
                                        local_y * scale_factor
                                            + (prim.height * scale_factor
                                                - prim.font_size * scale_factor)
                                                / 2.0,
                                    ),
                                    1.0,
                                );
                                if let Some(image) =
                                    swash_cache.get_image(font_system, physical_glyph.cache_key)
                                {
                                    let top = (physical_glyph.y as i32) - image.placement.top;
                                    let left = (physical_glyph.x as i32) + image.placement.left;
                                    if let cosmic_text::SwashContent::Mask = image.content {
                                        for (y, row) in image
                                            .data
                                            .chunks(image.placement.width as usize)
                                            .enumerate()
                                        {
                                            for (x, alpha_byte) in row.iter().enumerate() {
                                                let p_x = left + x as i32;
                                                let p_y = top + y as i32;
                                                if p_x >= 0
                                                    && p_x < width as i32
                                                    && p_y >= 0
                                                    && p_y < height as i32
                                                {
                                                    let pixel_idx = ((p_y as u32 * width)
                                                        + p_x as u32)
                                                        as usize
                                                        * 4;
                                                    blend_mask_pixel(
                                                        pixmap.data_mut(),
                                                        pixel_idx,
                                                        *alpha_byte,
                                                        value_opacity,
                                                        (vr_base, vg_base, vb_base),
                                                    );
                                                }
                                            }
                                        }
                                    }
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
