{
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
                        let bytes = word.len() - word.chars().last().expect("Albedo Engine: internal invariant violated").len_utf8();
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
                            let c = trailing.chars().next().expect("Albedo Engine: internal invariant violated");
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
