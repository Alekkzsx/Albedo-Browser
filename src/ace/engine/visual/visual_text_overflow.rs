{
                    // --- OPTIMIZED TEXT-OVERFLOW: ELLIPSIS (AceEngine) ---
                    // This logic handles the case where a single text node overflows its container
                    // specifically with white-space: nowrap and text-overflow: ellipsis/clip.
                    if !text.is_empty()
                        && matches!(
                            computed_style.white_space,
                            crate::ace::engine::style::css_values::CssWhiteSpace::NoWrap
                        )
                    {
                        let font_size = computed_style.font_size;
                        let line_height = crate::ace::engine::style::css_values::resolve_length(
                            &computed_style.line_height,
                            font_size,
                            16.0,
                            vw,
                            vh,
                        );
                        let line_height = if line_height <= 0.0 {
                            font_size * 1.2
                        } else {
                            line_height
                        };

                        let letter_spacing = crate::ace::engine::style::css_values::resolve_length(
                            &computed_style.letter_spacing,
                            font_size,
                            16.0,
                            vw,
                            vh,
                        );
                        let word_spacing = crate::ace::engine::style::css_values::resolve_length(
                            &computed_style.word_spacing,
                            font_size,
                            16.0,
                            vw,
                            vh,
                        );

                        let (total_w, _) = self.text_measurer.measure_text(
                            &text,
                            font_size,
                            line_height,
                            Some(&computed_style.font_family),
                            cosmic_text::Weight::NORMAL,
                            None,
                            letter_spacing,
                            word_spacing,
                        );

                        if total_w > w {
                            let use_ellipsis = matches!(
                                computed_style.text_overflow,
                                crate::ace::engine::style::css_values::CssTextOverflow::Ellipsis
                            );
                            let ellipsis = "…";
                            let (ell_w, _) = if use_ellipsis {
                                self.text_measurer.measure_text(
                                    ellipsis,
                                    font_size,
                                    line_height,
                                    Some(&computed_style.font_family),
                                    cosmic_text::Weight::NORMAL,
                                    None,
                                    letter_spacing,
                                    word_spacing,
                                )
                            } else {
                                (0.0, 0.0)
                            };

                            let safe_width = w - ell_w;

                            if safe_width > 0.0 {
                                let mut best_len = 0;
                                let mut left = 0;
                                let mut right = text.len();

                                while left <= right {
                                    let mid: usize = (left + right) / 2;
                                    let mut mid_adj = mid;
                                    while mid_adj > 0 && !text.is_char_boundary(mid_adj) {
                                        mid_adj -= 1;
                                    }

                                    let (sub_w, _) = self.text_measurer.measure_text(
                                        &text[..mid_adj],
                                        font_size,
                                        line_height,
                                        Some(&computed_style.font_family),
                                        cosmic_text::Weight::NORMAL,
                                        None,
                                        letter_spacing,
                                        word_spacing,
                                    );

                                    if sub_w <= safe_width {
                                        best_len = mid_adj;
                                        left = mid + 1;
                                        while left < text.len() && !text.is_char_boundary(left) {
                                            left += 1;
                                        }
                                    } else {
                                        right = mid.saturating_sub(1);
                                    }
                                }

                                let mut result_text = text[..best_len].to_string();
                                if use_ellipsis {
                                    result_text.push_str(ellipsis);
                                }
                                text = result_text;
                            } else if use_ellipsis {
                                text = ellipsis.to_string();
                            } else {
                                text = String::new(); // Clip everything
                            }
                        }
                    }
                    // -----------------------------------------------------

}
