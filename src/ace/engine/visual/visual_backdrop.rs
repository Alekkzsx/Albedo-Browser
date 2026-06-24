{
                    // Backdrop para <dialog data-ace-modal> (modal)
                    if element_tag == "dialog" {
                        if let crate::ace::engine::dom::AceNodeType::Element(el) = &node.node_type {
                            if el.attributes.contains_key("data-ace-modal") {
                                let backdrop = DisplayItem {
                                    x: 0.0,
                                    y: 0.0,
                                    width: vw,
                                    height: vh,
                                    background_color: Some(tiny_skia::Color::from_rgba8(
                                        0, 0, 0, 76,
                                    )),
                                    border_width: 0.0,
                                    border_color: None,
                                    border_style: crate::ace::engine::types::BorderStyle::None,
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
                                    is_fixed: true,
                                    opacity: 1.0,
                                    border_radius: [0.0; 4],
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
                                if backdrop.is_fixed {
                                    fixed_nodes.push(node_idx);
                                }
                                items.push(backdrop);
                            }
                        }
                    }
}
