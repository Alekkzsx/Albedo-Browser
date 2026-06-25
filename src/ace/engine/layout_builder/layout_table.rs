{
            // 1. Calculate dimensions and collect cells
            let mut row_count = 0;
            let mut col_count = 0;
            let mut cell_list = Vec::new(); // (row_idx, col_idx, node_idx)

            // Two-Pass Table Layout: Pre-calcular max-width do conteudo de cada coluna.
            // Como Taffy falha com tabelas puras, nós passaremos uma Grid com `px` fixo para CADA track.
            let mut col_max_widths: std::collections::HashMap<usize, f32> =
                std::collections::HashMap::new();

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
}
