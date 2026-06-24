{
                    if let Some(outline) = &computed_style.outline {
                        // outline-style: none => não renderiza
                        if outline.style != "none" {
                            let outline_color = Self::css_color_to_skia(&outline.color);
                            let total_gap = outline.width + outline.offset;
                            let outline_prim = DisplayItem {
                                pos_x: pos_x - total_gap,
                                pos_y: pos_y - total_gap,
                                width: w + total_gap * 2.0,
                                height: h + total_gap * 2.0,
                                background_color: None,
                                border_width: outline.width,
                                border_color: outline_color,
                                border_style: crate::ace::engine::types::BorderStyle::from_str(
                                    &outline.style,
                                ),
                                text_content: None,
                                text_color: tiny_skia::Color::BLACK,
                                text_overflow:
                                    crate::ace::engine::style::css_values::CssTextOverflow::Clip,
                                font_size: 0.0,
                                letter_spacing: 0.0,
                                word_spacing: 0.0,
                                image_url: None,
                                link_url: None,
                                node_idx,
                                element_type: crate::ace::engine::types::ElementRenderType::Other,
                                is_fixed: false,
                                opacity: 1.0,
                                border_radius: [
                                    (computed_style.border_radius_top_left + total_gap).max(0.0),
                                    (computed_style.border_radius_top_right + total_gap).max(0.0),
                                    (computed_style.border_radius_bottom_right + total_gap)
                                        .max(0.0),
                                    (computed_style.border_radius_bottom_left + total_gap).max(0.0),
                                ],
                                transform_rotate: 0.0,
                                transform_scale: (1.0, 1.0),
                                transform_translate: (0.0, 0.0),
                                canvas_data: None,
                                input_value: std::sync::Arc::from(""),
                                placeholder: std::sync::Arc::from(""),
                                input_type: crate::ace::engine::types::FormInputType::None,
                                input_min: std::sync::Arc::from(""),
                                input_max: std::sync::Arc::from(""),
                                input_step: std::sync::Arc::from(""),
                                options: std::sync::Arc::from(""),
                                padding_top: 0.0,
                                padding_right: 0.0,
                                padding_bottom: 0.0,
                                padding_left: 0.0,
                                font_weight:
                                    crate::ace::engine::style::css_values::CssFontWeight::Normal,
                                white_space:
                                    crate::ace::engine::style::css_values::CssWhiteSpace::Normal,
                                is_hovered: false,
                                is_focused: false,
                                clip_rect: None,
                            };
                            items.push(outline_prim);
                        }
                    }
}
