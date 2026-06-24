                        pos_x,
                        pos_y,
                        width: w,
                        height: h,
                        background_color,
                        border_width: geom
                            .border_top
                            .max(geom.border_right)
                            .max(geom.border_bottom)
                            .max(geom.border_left),
                        border_color,
                        border_style: crate::ace::engine::types::BorderStyle::Solid,
                        text_content: if text.is_empty() {
                            None
                        } else {
                            Some(std::sync::Arc::from(text.as_str()))
                        },
                        text_color,
                        text_overflow: computed_style.text_overflow.clone(),
                        font_size: computed_style.font_size,
                        letter_spacing: crate::ace::engine::style::css_values::resolve_length(
                            &computed_style.letter_spacing,
                            computed_style.font_size,
                            16.0,
                            vw,
                            vh,
                        ),
                        word_spacing: crate::ace::engine::style::css_values::resolve_length(
                            &computed_style.word_spacing,
                            computed_style.font_size,
                            16.0,
                            vw,
                            vh,
                        ),
                        image_url: None,
                        link_url: None,
                        node_idx,
                        element_type: crate::ace::engine::types::ElementRenderType::from_str(
                            &element_tag,
                        ),
                        is_fixed,
                        opacity: computed_style.opacity,
                        border_radius: [
                            computed_style.border_radius_top_left,
                            computed_style.border_radius_top_right,
                            computed_style.border_radius_bottom_right,
                            computed_style.border_radius_bottom_left,
                        ],
                        transform_rotate: 0.0,
                        transform_scale: (1.0, 1.0),
                        transform_translate: (0.0, 0.0),
                        canvas_data,
                        input_value: if element_tag == "input"
                            || element_tag == "textarea"
                            || element_tag == "select"
                        {
                            if let crate::ace::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                std::sync::Arc::from(
                                    el.attributes
                                        .get("value")
                                        .map(|s| s.as_str())
                                        .unwrap_or_default(),
                                )
                            } else {
                                std::sync::Arc::from("")
                            }
                        } else {
                            std::sync::Arc::from("")
                        },
                        placeholder: if let crate::ace::engine::dom::AceNodeType::Element(el) =
                            &node.node_type
                        {
                            std::sync::Arc::from(
                                el.attributes
                                    .get("placeholder")
                                    .map(|s| s.as_str())
                                    .unwrap_or_default(),
                            )
                        } else {
                            std::sync::Arc::from("")
                        },
                        input_type: if element_tag == "input" {
                            if let crate::ace::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                crate::ace::engine::types::FormInputType::from_str(
                                    el.attributes
                                        .get("type")
                                        .map(|s| s.as_str())
                                        .unwrap_or("text"),
                                )
                            } else {
                                crate::ace::engine::types::FormInputType::None
                            }
                        } else {
                            crate::ace::engine::types::FormInputType::None
                        },
                        input_min: if element_tag == "input" {
                            if let crate::ace::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                std::sync::Arc::from(
                                    el.attributes
                                        .get("min")
                                        .map(|s| s.as_str())
                                        .unwrap_or_default(),
                                )
                            } else {
                                std::sync::Arc::from("")
                            }
                        } else {
                            std::sync::Arc::from("")
                        },
                        input_max: if element_tag == "input" {
                            if let crate::ace::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                std::sync::Arc::from(
                                    el.attributes
                                        .get("max")
                                        .map(|s| s.as_str())
                                        .unwrap_or_default(),
                                )
                            } else {
                                std::sync::Arc::from("")
                            }
                        } else {
                            std::sync::Arc::from("")
                        },
                        input_step: if element_tag == "input" {
                            if let crate::ace::engine::dom::AceNodeType::Element(el) = &node.node_type {
                                std::sync::Arc::from(
                                    el.attributes
                                        .get("step")
                                        .map(|s| s.as_str())
                                        .unwrap_or_default(),
                                )
                            } else {
                                std::sync::Arc::from("")
                            }
                        } else {
                            std::sync::Arc::from("")
                        },
                        options: if element_tag == "select" {
                            let mut opts = Vec::new();
                            for &child_idx in &node.children {
                                if let Some(child) = dom.get_node(child_idx) {
                                    if let crate::ace::engine::dom::AceNodeType::Element(child_el) =
                                        &child.node_type
                                    {
                                        if child_el.tag == "option" {
                                            let txt = child.get_text_content();
                                            if !txt.is_empty() {
                                                opts.push(txt);
                                            }
                                        }
                                    }
                                }
                            }
                            std::sync::Arc::from(opts.join("|").as_str())
                        } else {
                            std::sync::Arc::from("")
                        },
                        padding_top: geom.padding_top,
                        padding_right: geom.padding_right,
                        padding_bottom: geom.padding_bottom,
                        padding_left: geom.padding_left,
                        font_weight: computed_style.font_weight.clone(),
                        white_space: computed_style.white_space.clone(),
                        is_hovered: self.hovered_element == Some(node_idx),
                        is_focused: self.focused_element == Some(node_idx),
                        clip_rect,
                    };
