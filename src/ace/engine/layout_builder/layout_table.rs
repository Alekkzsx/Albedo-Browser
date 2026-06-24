            // 1. Calculate dimensions and collect cells
            let mut row_count = 0;
            let mut col_count = 0;
            let mut cell_list = Vec::new(); // (row_idx, col_idx, node_idx)

            // Two-Pass Table Layout: Pre-calcular max-width do conteudo de cada coluna.
            // Como Taffy falha com tabelas puras, nós passaremos uma Grid com `px` fixo para CADA track.
            let mut col_max_widths: std::collections::HashMap<usize, f32> =
                std::collections::HashMap::new();

pub(crate) fn scan_table_children(
                dom: &AceDOM,
                node_idx: usize,
                row_count: &mut usize,
                col_count: &mut usize,
                cell_list: &mut Vec<(usize, usize, usize)>,
                col_max_widths: &mut std::collections::HashMap<usize, f32>,
                font_size_cache: f32,
            ) {
                if let Some(node) = dom.get_node(node_idx) {
                    for &child_idx in &node.children {
                        if let Some(child_node) = dom.get_node(child_idx) {
                            if let crate::ace::engine::dom::AceNodeType::Element(el) =
                                &child_node.node_type
                            {
                                let tag = el.tag.as_str();
                                if tag == "tr" {
                                    let mut current_cols = 0;
                                    for &cell_idx in &child_node.children {
                                        if let Some(cell_node) = dom.get_node(cell_idx) {
                                            if let crate::ace::engine::dom::AceNodeType::Element(
                                                cell_el,
                                            ) = &cell_node.node_type
                                            {
                                                if cell_el.tag == "td" || cell_el.tag == "th" {
                                                    cell_list.push((
                                                        *row_count,
                                                        current_cols,
                                                        cell_idx,
                                                    ));

                                                    // TWO-PASS: Buscar nó de texto filho pra saber a largura bruta em string
                                                    let mut text_len = 0.0;
                                                    for &inner_idx in &cell_node.children {
                                                        if let Some(inner_node) =
                                                            dom.get_node(inner_idx)
                                                        {
                                                            if let crate::ace::engine::dom::AceNodeType::Text(text_str) = &inner_node.node_type {
                                                                 // (Rough character estimation * font_size / 2.0 = text width px)
                                                                 text_len += text_str.len() as f32 * (font_size_cache * 0.55);
                                                            }
                                                        }
                                                    }
                                                    // Add 20px of default padding to the text
                                                    text_len += 20.0;

                                                    let max_w = col_max_widths
                                                        .entry(current_cols)
                                                        .or_insert(0.0);
                                                    if text_len > *max_w {
                                                        *max_w = text_len;
                                                    }

                                                    current_cols += 1;
                                                }
                                            }
                                        }
                                    }
                                    if current_cols > *col_count {
                                        *col_count = current_cols;
                                    }
                                    *row_count += 1;
                                } else if tag == "thead" || tag == "tbody" || tag == "tfoot" {
                                    scan_table_children(
                                        dom,
                                        child_idx,
                                        row_count,
                                        col_count,
                                        cell_list,
                                        col_max_widths,
                                        font_size_cache,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            scan_table_children(
                dom,
                node_idx,
                &mut row_count,
                &mut col_count,
                &mut cell_list,
                &mut col_max_widths,
                style.font_size,
            );

            // 2. Set Grid Template with Pre-calculated PX
            if col_count > 0 {
                let mut tracks = Vec::new();
                for i in 0..col_count {
                    let computed_px = *col_max_widths.get(&i).unwrap_or(&100.0);
                    tracks.push(taffy::prelude::TrackSizingFunction::Single(
                        taffy::geometry::MinMax {
                            min: taffy::prelude::MinTrackSizingFunction::Fixed(
                                taffy::prelude::LengthPercentage::Points(computed_px),
                            ),
                            max: taffy::prelude::MaxTrackSizingFunction::Fixed(
                                taffy::prelude::LengthPercentage::Points(computed_px),
                            ),
                        },
                    ));
                }
                taffy_style.grid_template_columns = tracks;
            }

            // 3. Create Children (Cells flattened)
            let mut children = Vec::new();
            for (r, c, cell_idx) in cell_list {
                let cell_nodes = self
                    .build_layout_tree(dom, taffy, stylesheet, cell_idx, vw, vh, node_map, None);
                if let Some(&cell_taffy_node) = cell_nodes.first() {
                    let mut cell_style = taffy.style(cell_taffy_node).expect("Albedo Engine: internal invariant violated").clone();
                    // 1-based index for grid placement
                    cell_style.grid_row.start =
                        taffy::prelude::GridPlacement::Line((r as i16 + 1).into());
                    cell_style.grid_column.start =
                        taffy::prelude::GridPlacement::Line((c as i16 + 1).into());
                    let _ = taffy.set_style(cell_taffy_node, cell_style);
                    children.push(cell_taffy_node);
                }
            }

            let taffy_node = taffy.new_with_children(taffy_style, &children).expect("Albedo Engine: internal invariant violated");
            node_map.insert(taffy_node, node_idx);
            return vec![taffy_node];
