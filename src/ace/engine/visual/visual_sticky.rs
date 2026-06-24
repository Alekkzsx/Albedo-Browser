{
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
                                pos_y = stuck_y + delta_y;
                            } else {
                                pos_y = stuck_y;
                            }
                            is_sticky_fixed = true;
                        }
                    }
                    // ─── FIM STICKY ─────────────────────────────────────────

}
