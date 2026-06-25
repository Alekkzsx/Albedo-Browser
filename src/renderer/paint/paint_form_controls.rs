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
                    .expect("Albedo Engine: internal invariant violated");
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