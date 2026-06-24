use super::*;

use crate::ace::engine::layout_types::DisplayItem;
use crate::ace::engine::core::AceEngine;


impl AceEngine {
    /// TODO: add docs
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
            let dom = dom_arc.lock().unwrap_or_else(|e| e.into_inner());
            let stylesheet = self.stylesheet.lock().unwrap_or_else(|e| e.into_inner());
            let geometry = self.element_geometry.lock().unwrap_or_else(|e| e.into_inner());
            let element_styles = self.element_styles.lock().unwrap_or_else(|e| e.into_inner());

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
                    let pos_x = geom.x;
                    let mut pos_y = geom.y;
                    let w = geom.width;
                    let h = geom.height;
                    let mut is_sticky_fixed = false;

                    include!("visual_sticky.rs");
                    // ─── CULLING (VIRTUAL SCROLLING) ────────────────────────
                    // Skip processing elements completely outside the visible viewport
                    // Exceptions: position:fixed (always on screen)
                    if effective_position != crate::ace::engine::style::css_values::CssPosition::Fixed {
                        let element_top = pos_y;
                        let element_bottom = pos_y + h;

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

                    include!("visual_text_overflow.rs");
                    let mut canvas_data = None;
                    if element_tag == "canvas" {
                        let contexts = self.canvas_contexts.lock().unwrap_or_else(|e| e.into_inner());
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
                    let prim = include!("visual_display_item.rs");
                    include!("visual_outline.rs");
                    include!("visual_backdrop.rs");
                    if prim.is_fixed {
                        fixed_nodes.push(node_idx);
                    }
                    items.push(prim);
                }
            }
        }
        crate::ace::engine::layer_tree::LayerTree::build(items, &fixed_nodes)
    }
}
