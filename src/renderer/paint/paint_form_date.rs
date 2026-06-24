{
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
