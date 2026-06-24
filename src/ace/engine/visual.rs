
use crate::ace::engine::layout_types::DisplayItem;
use crate::ace::engine::core::AceEngine;

impl AceEngine {
    pub fn render_visual(&self, vw: f32, vh: f32) -> crate::ace::engine::layer_tree::LayerTree {
        let mut items = Vec::new();
        let mut fixed_nodes = Vec::new();

        // ─── VIRTUAL SCROLLING / CULLING BOUNDARIES ─────────────
        // viewport_y is negative when scrolled down
        let scroll_y = (-self.viewport_y).max(0.0);
        // Add a buffer so elements don't pop-in instantly
        let culling_buffer = vh * 1.5;
        let visible_top = scroll_y - culling_buffer;
        let visible_bottom = scroll_y + vh + culling_buffer;
        // ────────────────────────────────────────────────────────

        if let Some(ref dom_arc) = self.dom {
            let dom = dom_arc.lock().unwrap();
            let stylesheet = self.stylesheet.lock().unwrap();
            let geometry = self.element_geometry.lock().unwrap();
            let element_styles = self.element_styles.lock().unwrap();

            for (node_idx, node) in dom.nodes.iter().enumerate() {
                let (is_element, element_tag): (bool, &str) = match &node.node_type {
                    crate::ace::engine::dom::AceNodeType::Element(el) => (true, el.tag.as_str()),
                    crate::ace::engine::dom::AceNodeType::Text(_) => (false, "#text"),
                    _ => continue,
                };

                // Ocultar tags técnicas e seu conteúdo (CSS, JS, etc.)
                if is_element && self.is_technical_tag(&element_tag) {
                    continue;
                }
                if self.has_technical_ancestor(&dom, node_idx) {
                    continue;
                }

                // Usar estilo cacheado ou calcular se faltar (ex: novos nós)
                let mut computed_style = if let Some(cached) = element_styles.get(&node_idx) {
                    cached.clone()
                } else {
                    // Fallback para calculate_style simples (ponto de melhoria futuro: herança aqui também)
                    stylesheet.calculate_style(
                        &dom,
                        node_idx,
                        None,
                        None,
                        self.hovered_element,
                        self.focused_element,
                        self.active_element,
                        None,
                        0.0,
                        vw,
                        vh,
                        "light",
                    )
                };

                // Inherit parent styles for pure text nodes
                if !is_element {
                    if let Some(parent_idx) = node.parent {
                        if let Some(parent_style) = element_styles.get(&parent_idx) {
                            computed_style = parent_style.clone();
                        }
                    }
                }

                if matches!(
                    computed_style.display,
                    crate::ace::engine::style::css_values::CssDisplay::None
                ) {
                    continue;
                }

                if let Some(geom) = geometry.get(&node_idx) {
                    let x = geom.x;
                    let mut y = geom.y;
                    let w = geom.width;
                    let h = geom.height;
                    let mut is_sticky_fixed = false;

                    // ─── STICKY POSITIONING ─────────────────────────────────
                    // position:sticky → o elemento flui normalmente até que o
                    // scroll do viewport ultrapasse seu threshold (top/bottom).
                    // Quando "stuck", comporta-se como fixed, clampado pelo parent.
                    let effective_position = if is_element {
                        computed_style.position.clone()
                    } else {
                        // Texto herda sticky do pai
                        if let Some(parent_idx) = node.parent {
                            if let Some(ps) = element_styles.get(&parent_idx) {
                                ps.position.clone()
                            } else {
                                crate::ace::engine::style::css_values::CssPosition::Static
                            }
                        } else {
                            crate::ace::engine::style::css_values::CssPosition::Static
                        }
                    };

                    if effective_position == crate::ace::engine::style::css_values::CssPosition::Sticky {
                        let font_size = computed_style.font_size;
                        // Resolver o threshold CSS (top, bottom)
                        let sticky_top = crate::ace::engine::style::css_values::resolve_length(
                            &computed_style.top,
                            font_size,
                            16.0,
                            vw,
                            vh,
                        );

                        // scroll_y: quanto o viewport desceu (viewport_y é negativo quando scrollado)
                        let scroll_y = (-self.viewport_y).max(0.0);

                        // Para texto filho de sticky, usar a geometria do pai sticky
                        let (natural_y, elem_h, parent_info) = if !is_element {
                            if let Some(parent_idx) = node.parent {
                                if let Some(pg) = geometry.get(&parent_idx) {
                                    // Delta relativo ao pai
                                    let delta_y = geom.y - pg.y;
                                    // Buscar avô para constraint
                                    let gp_info = if let Some(pnode) = dom.get_node(parent_idx) {
                                        if let Some(gp_idx) = pnode.parent {
                                            geometry.get(&gp_idx).map(|gpg| (gpg.y, gpg.height))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };
                                    (pg.y, pg.height, Some((delta_y, gp_info)))
                                } else {
                                    (geom.y, h, None)
                                }
                            } else {
                                (geom.y, h, None)
                            }
                        } else {
                            (geom.y, h, None)
                        };

                        // Buscar parent container para clampar
                        let (_parent_top, parent_bottom) = if let Some((_, gp_info)) = &parent_info
                        {
                            // Para texto: constraint é o avô do sticky
                            if let Some((gp_y, gp_h)) = gp_info {
                                (*gp_y, *gp_y + *gp_h)
                            } else {
                                (0.0, vh * 10.0) // fallback
                            }
                        } else if let Some(parent_idx) = node.parent {
                            if let Some(pg) = geometry.get(&parent_idx) {
                                (pg.y, pg.y + pg.height)
                            } else {
                                (0.0, vh * 10.0)
                            }
                        } else {
                            (0.0, vh * 10.0)
                        };

                        // Clampar: o elemento gruda quando sairia do viewport
                        let threshold_line = scroll_y + sticky_top;
                        if natural_y < threshold_line {
                            // Elemento grudou — fixar na posição do threshold
                            let max_y = parent_bottom - elem_h;
                            let stuck_y = threshold_line.min(max_y);

                            if let Some((delta_y, _)) = parent_info {
                                // Texto filho: aplicar o mesmo delta do pai sticky
                                y = stuck_y + delta_y;
                            } else {
                                y = stuck_y;
                            }
                            is_sticky_fixed = true;
                        }
                    }
                    // ─── FIM STICKY ─────────────────────────────────────────

                    // ─── CULLING (VIRTUAL SCROLLING) ────────────────────────
                    // Skip processing elements completely outside the visible viewport
                    // Exceptions: position:fixed (always on screen)
                    if effective_position != crate::ace::engine::style::css_values::CssPosition::Fixed {
                        let element_top = y;
                        let element_bottom = y + h;

                        if element_bottom < visible_top || element_top > visible_bottom {
                            continue; // Element is entirely culled
                        }
                    }
                    // ────────────────────────────────────────────────────────

                    let background_color =
                        Self::css_color_to_skia(&computed_style.background_color);
                    let border_color = Self::css_color_to_skia(&computed_style.border_color_top);
                    let text_color = Self::css_color_to_skia(&computed_style.color)
                        .unwrap_or(tiny_skia::Color::BLACK);

                    let mut text = match &node.node_type {
                        crate::ace::engine::dom::AceNodeType::Text(t) => {
                            crate::ace::engine::layout::inline::apply_text_transform(
                                t.as_ref(),
                                &computed_style.text_transform,
                            )
                        }
                        _ => String::new(),
                    };

                    // Disclosure marker para <summary>: ▶ (fechado) ou ▼ (aberto)
                    if !text.is_empty() {
                        if let Some(parent_idx) = node.parent {
                            if let Some(parent_node) = dom.get_node(parent_idx) {
                                if let crate::ace::engine::dom::AceNodeType::Element(parent_el) =
                                    &parent_node.node_type
                                {
                                    if parent_el.tag == "summary" {
                                        // Verificar se este é o primeiro filho de texto do summary
                                        let is_first_text =
                                            parent_node.children.first() == Some(&node_idx);
                                        if is_first_text {
                                            // Verificar o avô <details> pelo atributo "open"
                                            let is_open = if let Some(gp_idx) = parent_node.parent {
                                                if let Some(gp_node) = dom.get_node(gp_idx) {
                                                    if let crate::ace::engine::dom::AceNodeType::Element(gp_el) = &gp_node.node_type {
                                                        gp_el.tag == "details" && gp_el.attributes.contains_key("open")
                                                    } else { false }
                                                } else {
                                                    false
                                                }
                                            } else {
                                                false
                                            };
                                            let marker = if is_open { "▼ " } else { "▶ " };
                                            text = format!("{}{}", marker, text);
                                        }
                                    }
                                }
                            }
                        }
                    }

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

                    let mut canvas_data = None;
                    if element_tag == "canvas" {
                        let contexts = self.canvas_contexts.lock().unwrap();
                        if let Some(ctx2d) = contexts.get(&node_idx) {
                            canvas_data = Some(ctx2d.get_pixels().to_vec());
                        }
                    } else if element_tag == "svg" {
                        let svg_xml = dom.serialize_subtree_html(node_idx);
                        if let Some(pixels) =
                            crate::ace::engine::svg::rasterize_svg_to_pixels(&svg_xml, w, h)
                        {
                            canvas_data = Some(pixels);
                        }
                    }

                    // ─── CLIPPING (OVERFLOW: HIDDEN / SCROLL / AUTO) ─────────
                    let mut clip_rect: Option<[f32; 4]> = None;
                    let mut current_ancestor = node.parent;
                    while let Some(pidx) = current_ancestor {
                        if let Some(pgeom) = geometry.get(&pidx) {
                            if pgeom.overflow_y
                                != crate::ace::engine::style::css_values::CssOverflow::Visible
                                || pgeom.overflow_x
                                    != crate::ace::engine::style::css_values::CssOverflow::Visible
                            {
                                let cr = [pgeom.x, pgeom.y, pgeom.width, pgeom.height];
                                clip_rect = Some(match clip_rect {
                                    Some(c) => {
                                        // Intersect rects
                                        let x1 = c[0].max(cr[0]);
                                        let y1 = c[1].max(cr[1]);
                                        let x2 = (c[0] + c[2]).min(cr[0] + cr[2]);
                                        let y2 = (c[1] + c[3]).min(cr[1] + cr[3]);
                                        let w = (x2 - x1).max(0.0);
                                        let h = (y2 - y1).max(0.0);
                                        [x1, y1, w, h]
                                    }
                                    None => cr,
                                });
                            }
                        }
                        current_ancestor = dom.get_node(pidx).and_then(|n| n.parent);
                    }
                    // ─── FIM CLIPPING ───────────────────────────────────────

                    let is_fixed = is_sticky_fixed
                        || matches!(
                            computed_style.position,
                            crate::ace::engine::style::css_values::CssPosition::Fixed
                        );

                    let prim = DisplayItem {
                        x,
                        y,
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

                    if let Some(outline) = &computed_style.outline {
                        // outline-style: none => não renderiza
                        if outline.style != "none" {
                            let outline_color = Self::css_color_to_skia(&outline.color);
                            let total_gap = outline.width + outline.offset;
                            let outline_prim = DisplayItem {
                                x: x - total_gap,
                                y: y - total_gap,
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
                    if prim.is_fixed {
                        fixed_nodes.push(node_idx);
                    }
                    items.push(prim);
                }
            }
        }
        crate::ace::engine::layer_tree::LayerTree::build(items, &fixed_nodes)
    }
    pub fn tick(&mut self, now: f64) -> bool {
        let mut am = self.animation_manager.lock().unwrap();
        let mut has_changes = am.tick(now);

        // Disparar requestAnimationFrame callbacks
        if let Some(ref rt) = self.js_runtime {
            if rt.run_raf_callbacks(now) {
                has_changes = true;
            }
        }

        if has_changes {
            tracing::debug!("tick() has_changes == true, setting styles_dirty");
            self.styles_dirty
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }

        has_changes
    }
    pub fn css_color_to_skia(
        css_color: &crate::ace::engine::style::css_values::CssColor,
    ) -> Option<tiny_skia::Color> {
        use crate::ace::engine::style::css_values::CssColor;
        match css_color {
            CssColor::Rgba(r, g, b, a) => {
                Some(tiny_skia::Color::from_rgba8(*r, *g, *b, (*a * 255.0) as u8))
            }
            CssColor::Named(name) => {
                if name.eq_ignore_ascii_case("transparent") {
                    return None;
                }
                if name.starts_with("#") {
                    let hex = name.trim_start_matches('#');
                    if hex.len() == 3 {
                        // Expandir dígito hex: 0xA => 0xAA == A * 17 (zero alocações)
                        let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0) * 17;
                        let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0) * 17;
                        let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0) * 17;
                        Some(tiny_skia::Color::from_rgba8(r, g, b, 255))
                    } else {
                        let r =
                            u8::from_str_radix(if hex.len() >= 2 { &hex[0..2] } else { hex }, 16)
                                .unwrap_or(0);
                        let g =
                            u8::from_str_radix(if hex.len() >= 4 { &hex[2..4] } else { "0" }, 16)
                                .unwrap_or(0);
                        let b =
                            u8::from_str_radix(if hex.len() >= 6 { &hex[4..6] } else { "0" }, 16)
                                .unwrap_or(0);
                        let a = if hex.len() == 8 {
                            u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
                        } else {
                            255
                        };
                        Some(tiny_skia::Color::from_rgba8(r, g, b, a))
                    }
                } else {
                    let lower = name.to_ascii_lowercase();
                    match lower.as_str() {
                        "white" => Some(tiny_skia::Color::from_rgba8(255, 255, 255, 255)),
                        "black" | "currentcolor" => {
                            Some(tiny_skia::Color::from_rgba8(0, 0, 0, 255))
                        }
                        "red" => Some(tiny_skia::Color::from_rgba8(255, 0, 0, 255)),
                        "green" => Some(tiny_skia::Color::from_rgba8(0, 128, 0, 255)),
                        "blue" => Some(tiny_skia::Color::from_rgba8(0, 0, 255, 255)),
                        "darkblue" => Some(tiny_skia::Color::from_rgba8(0, 0, 139, 255)),
                        "darkgreen" => Some(tiny_skia::Color::from_rgba8(0, 100, 0, 255)),
                        "darkred" => Some(tiny_skia::Color::from_rgba8(139, 0, 0, 255)),
                        "yellow" => Some(tiny_skia::Color::from_rgba8(255, 255, 0, 255)),
                        "gray" | "grey" => Some(tiny_skia::Color::from_rgba8(128, 128, 128, 255)),
                        "lightslategray" => Some(tiny_skia::Color::from_rgba8(119, 136, 153, 255)),
                        _ => None,
                    }
                }
            }
            _ => None,
        }
    }
}
