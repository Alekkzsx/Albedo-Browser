
use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::style::Stylesheet;
use crate::ace::engine::layout::ElementGeometry;
use crate::utils::time::unix_timestamp_secs_f64;
use taffy::geometry::MinMax;
use crate::ace::engine::core::AceEngine;

#[derive(Clone, Debug, Default)]
pub struct GridContext {
    pub column_names: std::collections::HashMap<String, Vec<i16>>,
    pub row_names: std::collections::HashMap<String, Vec<i16>>,
    pub areas: std::collections::HashMap<String, (usize, usize, usize, usize)>,
    pub col_offset: i16,
    pub row_offset: i16,
}

impl AceEngine {
    pub(crate) fn build_layout_tree(
        &self,
        dom: &mut AceDOM,
        taffy: &mut taffy::Taffy,
        stylesheet: &Stylesheet,
        node_idx: usize,
        vw: f32,
        vh: f32,
        node_map: &mut std::collections::HashMap<taffy::prelude::Node, usize>,
        parent_grid_ctx: Option<&GridContext>,
    ) -> Vec<taffy::prelude::Node> {
        let (node_type, children_indices) = {
            let node = dom.get_node(node_idx).unwrap();
            (node.node_type.clone(), node.children.clone())
        };
        let now = unix_timestamp_secs_f64();

        let style = {
            let engine_styles = self.element_styles.lock().unwrap();
            engine_styles.get(&node_idx).cloned().unwrap_or_else(|| {
                let am = self.animation_manager.lock().unwrap();
                stylesheet.calculate_style(
                    dom,
                    node_idx,
                    None,
                    None,
                    self.hovered_element,
                    self.focused_element,
                    self.active_element,
                    Some(&am),
                    now,
                    vw,
                    vh,
                    "light",
                )
            })
        };

        // --- Incremental Check ---
        let existing_node = {
            let n2t = self.node_to_taffy.lock().unwrap();
            n2t.get(&node_idx).cloned()
        };

        let node_flags = {
            let node = dom.get_node(node_idx).unwrap();
            node.dirty
        };

        let is_strictly_dirty = node_flags.intersects(
            crate::ace::engine::dom::NodeDirtyFlags::LAYOUT
                | crate::ace::engine::dom::NodeDirtyFlags::CHILDREN
                | crate::ace::engine::dom::NodeDirtyFlags::STYLE,
        );
        let has_dirty_descendants =
            node_flags.contains(crate::ace::engine::dom::NodeDirtyFlags::SUBTREE);

        if let Some(taffy_node) = existing_node {
            if !is_strictly_dirty && !has_dirty_descendants && parent_grid_ctx.is_none() {
                // TRUE INCREMENTAL: Node and its entire subtree are clean. Skip everything.
                node_map.insert(taffy_node, node_idx);
                self.populate_node_map_recursively(dom, taffy, taffy_node, node_idx, node_map);
                return vec![taffy_node];
            }

            if !is_strictly_dirty && has_dirty_descendants && parent_grid_ctx.is_none() {
                // Style/Layout is clean, but children need work.
                // We don't return early here, but we will skip set_style later.
            }
        }

        // --- Element Geometry: Style Extraction ---
        {
            let mut geometry = self.element_geometry.lock().unwrap();
            let geom = geometry.entry(node_idx).or_insert(ElementGeometry::new());

            // Resolve Box Model
            let resolve = |l: &crate::ace::engine::style::css_values::CssLength| -> f32 {
                crate::ace::engine::style::css_values::resolve_length(l, style.font_size, 16.0, vw, vh)
            };

            geom.float = style.float.clone();
            geom.clear = style.clear.clone();

            geom.padding_top = resolve(&style.padding_top);
            geom.padding_right = resolve(&style.padding_right);
            geom.padding_bottom = resolve(&style.padding_bottom);
            geom.padding_left = resolve(&style.padding_left);

            geom.border_top = resolve(&style.border_width_top);
            geom.border_right = resolve(&style.border_width_right);
            geom.border_bottom = resolve(&style.border_width_bottom);
            geom.border_left = resolve(&style.border_width_left);

            geom.overflow_x = style.overflow.clone();
            geom.overflow_y = style.overflow.clone();
        }
        // ------------------------------------------

        let mut taffy_style = self.convert_to_taffy_style(&style);

        // Resolve manual grid placements if in grid container
        if let Some(ctx) = parent_grid_ctx {
            self.apply_grid_context(&mut taffy_style, &style, ctx);
        }

        // Check if we should skip this node (flattening)
        // subgrid also triggers flattening in this implementation to inherit tracks
        let is_subgrid_cols = style
            .grid_template_columns
            .contains(&crate::ace::engine::style::css_values::CssLength::Subgrid);
        let is_subgrid_rows = style
            .grid_template_rows
            .contains(&crate::ace::engine::style::css_values::CssLength::Subgrid);
        let should_flatten = style.display
            == crate::ace::engine::style::css_values::CssDisplay::Contents
            || is_subgrid_cols
            || is_subgrid_rows;

        let mut grid_ctx = None;

        if style.display == crate::ace::engine::style::css_values::CssDisplay::Grid {
            grid_ctx = Some(GridContext {
                column_names: self.resolve_grid_names(&style.grid_template_columns),
                row_names: self.resolve_grid_names(&style.grid_template_rows),
                areas: self.resolve_grid_areas(&style.grid_template_areas),
                col_offset: 0,
                row_offset: 0,
            });
        } else if style.display == crate::ace::engine::style::css_values::CssDisplay::Table {
            // 1. Calculate dimensions and collect cells
            let mut row_count = 0;
            let mut col_count = 0;
            let mut cell_list = Vec::new(); // (row_idx, col_idx, node_idx)

            // Two-Pass Table Layout: Pre-calcular max-width do conteudo de cada coluna.
            // Como Taffy falha com tabelas puras, nós passaremos uma Grid com `px` fixo para CADA track.
            let mut col_max_widths: std::collections::HashMap<usize, f32> =
                std::collections::HashMap::new();

            fn scan_table_children(
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
                    let mut cell_style = taffy.style(cell_taffy_node).unwrap().clone();
                    // 1-based index for grid placement
                    cell_style.grid_row.start =
                        taffy::prelude::GridPlacement::Line((r as i16 + 1).into());
                    cell_style.grid_column.start =
                        taffy::prelude::GridPlacement::Line((c as i16 + 1).into());
                    let _ = taffy.set_style(cell_taffy_node, cell_style);
                    children.push(cell_taffy_node);
                }
            }

            let taffy_node = taffy.new_with_children(taffy_style, &children).unwrap();
            node_map.insert(taffy_node, node_idx);
            return vec![taffy_node];
        } else if (is_subgrid_cols || is_subgrid_rows) && parent_grid_ctx.is_some() {
            // If this is a subgrid, we use the parent's context but with offsets
            let parent_ctx = parent_grid_ctx.unwrap();
            let mut ctx = parent_ctx.clone();

            let resolve_line_local = |l: &crate::ace::engine::style::css_values::CssLength,
                                      names: &std::collections::HashMap<String, Vec<i16>>|
             -> Option<i16> {
                use crate::ace::engine::style::css_values::CssLength;
                match l {
                    CssLength::Number(v) => Some(*v as i16),
                    CssLength::Name(n) => names.get(n).and_then(|v| v.first().copied()),
                    CssLength::Span(v) => Some(*v as i16),
                    _ => None,
                }
            };

            // Calculate offsets based on subgrid's own placement in parent
            if let Some(start) =
                resolve_line_local(&style.grid_column_start, &parent_ctx.column_names)
            {
                ctx.col_offset += start - 1;
            } else if let crate::ace::engine::style::css_values::CssLength::Name(name) =
                &style.grid_column_start
            {
                if let Some(area) = parent_ctx.areas.get(name) {
                    ctx.col_offset += (area.2 as i16) - 1;
                }
            }

            if let Some(start) = resolve_line_local(&style.grid_row_start, &parent_ctx.row_names) {
                ctx.row_offset += start - 1;
            } else if let crate::ace::engine::style::css_values::CssLength::Name(name) =
                &style.grid_row_start
            {
                if let Some(area) = parent_ctx.areas.get(name) {
                    ctx.row_offset += (area.0 as i16) - 1;
                }
            }

            grid_ctx = Some(ctx);
        } else if let crate::ace::engine::dom::AceNodeType::Element(el) = &node_type {
            if el.tag == "iframe" {
                // If this is an iframe, ensure we have a subframe engine for it
                let mut needs_init = false;
                let mut iframe_src = String::new();

                {
                    if dom.subframes.is_none() {
                        dom.subframes = Some(std::sync::Arc::new(std::sync::Mutex::new(
                            std::collections::HashMap::new(),
                        )));
                    }

                    let subframes_arc = dom.subframes.as_ref().unwrap().clone();
                    let mut subframes = subframes_arc.lock().unwrap();

                    if !subframes.contains_key(&node_idx) {
                        let sub_engine =
                            std::sync::Arc::new(std::sync::Mutex::new(AceEngine::new()));
                        // Marcar qual nó <iframe> este subframe representa no pai
                        {
                            let sub_eng = sub_engine.lock().unwrap();
                            if let Some(ref dom_arc) = sub_eng.dom {
                                let mut sub_dom = dom_arc.lock().unwrap();
                                sub_dom.iframe_node_idx = Some(node_idx);
                            }
                        }
                        subframes.insert(node_idx, sub_engine);
                        needs_init = true;

                        if let Some(src) = el.attributes.get("src") {
                            iframe_src = src.clone();
                        }
                    }
                }

                if needs_init && !iframe_src.is_empty() {
                    if let Some(subframes_arc) = &dom.subframes {
                        let subframes_lock = subframes_arc.lock().unwrap();
                        if let Some(engine_arc) = subframes_lock.get(&node_idx) {
                            let mut sub_engine = engine_arc.lock().unwrap();
                            // Copy over resource manager and store target URL.
                            // JS runtime is initialized lazily when the iframe content
                            // is actually loaded (do NOT call init_js_for_url here as
                            // it runs init_stdlib which uses tokio and may block).
                            sub_engine.resource_manager = self.resource_manager.clone();
                            sub_engine.current_url = iframe_src.clone();
                        }
                    }
                }
            } else if el.tag == "svg" {
                // SVGs are treated as leaf nodes in layout, but we need to extract their dimensions
                let mut svg_width = style.width.clone();
                let mut svg_height = style.height.clone();

                // If CSS size is auto, try to get from attributes
                if svg_width == crate::ace::engine::style::css_values::CssLength::Auto {
                    if let Some(w_attr) = el.attributes.get("width") {
                        if let Ok(w) = w_attr.parse::<f32>() {
                            svg_width = crate::ace::engine::style::css_values::CssLength::Px(w);
                        }
                    }
                }
                if svg_height == crate::ace::engine::style::css_values::CssLength::Auto {
                    if let Some(h_attr) = el.attributes.get("height") {
                        if let Ok(h) = h_attr.parse::<f32>() {
                            svg_height = crate::ace::engine::style::css_values::CssLength::Px(h);
                        }
                    }
                }

                // Override taffy style size for the SVG leaf node
                taffy_style.size.width = self.to_taffy_dimension(&svg_width);
                taffy_style.size.height = self.to_taffy_dimension(&svg_height);

                let taffy_node = taffy.new_leaf(taffy_style).unwrap();
                node_map.insert(taffy_node, node_idx);
                return vec![taffy_node];
            }
        } else if let crate::ace::engine::dom::AceNodeType::Text(text) = &node_type {
            let transformed_text =
                crate::ace::engine::layout::inline::apply_text_transform(text.as_ref(), &style.text_transform);
            // Text nodes need an intrinsic size estimate to be visible
            let font_size = style.font_size;
            let line_height = font_size * 1.2;

            // Usar o TextMeasurer global para dimensões reais
            let letter_spacing = crate::ace::engine::style::css_values::resolve_length(
                &style.letter_spacing,
                font_size,
                16.0,
                vw,
                vh,
            );
            let word_spacing = crate::ace::engine::style::css_values::resolve_length(
                &style.word_spacing,
                font_size,
                16.0,
                vw,
                vh,
            );

            let (text_width, text_height) = self.text_measurer.measure_text(
                &transformed_text,
                font_size,
                line_height,
                Some(&style.font_family),
                cosmic_text::Weight::NORMAL,
                None,
                letter_spacing,
                word_spacing,
            );

            let width = text_width;
            let height = text_height;

            taffy_style.size.width = taffy::prelude::Dimension::Points(width);
            taffy_style.size.height = taffy::prelude::Dimension::Points(height);

            // Se o texto for curto, não queremos que ele "estique" se for um bloco
            taffy_style.max_size.width = taffy::prelude::Dimension::Points(width);

            let taffy_node = if let Some(node) = existing_node {
                if is_strictly_dirty {
                    let _ = taffy.set_style(node, taffy_style);
                }
                node
            } else {
                let node = taffy.new_leaf(taffy_style).unwrap();
                self.node_to_taffy.lock().unwrap().insert(node_idx, node);
                node
            };

            node_map.insert(taffy_node, node_idx);
            return vec![taffy_node];
        }

        let mut children = Vec::new();

        if style.display == crate::ace::engine::style::css_values::CssDisplay::Table {
            // Already handled in build_layout_tree main block for tables
        } else if style.display == crate::ace::engine::style::css_values::CssDisplay::TableRow {
            // Table rows are flattened, their children (cells) become direct children of the table grid
            for child_idx in children_indices {
                children.extend(self.build_layout_tree(
                    dom,
                    taffy,
                    stylesheet,
                    child_idx,
                    vw,
                    vh,
                    node_map,
                    grid_ctx.as_ref(),
                ));
            }
        } else if style.display == crate::ace::engine::style::css_values::CssDisplay::TableHeader {
            // Table headers are also flattened, their children (cells) become direct children of the table grid
            for child_idx in children_indices {
                children.extend(self.build_layout_tree(
                    dom,
                    taffy,
                    stylesheet,
                    child_idx,
                    vw,
                    vh,
                    node_map,
                    grid_ctx.as_ref(),
                ));
            }
        } else {
            // <details> filtering: quando fechado (sem atributo "open"),
            // renderizar apenas filhos <summary>, ignorando todo o resto
            let is_details_closed = if let crate::ace::engine::dom::AceNodeType::Element(el) = &node_type
            {
                el.tag == "details" && !el.attributes.contains_key("open")
            } else {
                false
            };

            for child_idx in children_indices {
                if is_details_closed {
                    if let Some(child_node) = dom.get_node(child_idx) {
                        let is_summary = if let crate::ace::engine::dom::AceNodeType::Element(child_el) =
                            &child_node.node_type
                        {
                            child_el.tag == "summary"
                        } else {
                            false
                        };
                        if !is_summary {
                            continue;
                        }
                    }
                }
                children.extend(self.build_layout_tree(
                    dom,
                    taffy,
                    stylesheet,
                    child_idx,
                    vw,
                    vh,
                    node_map,
                    grid_ctx.as_ref(),
                ));
            }
        }

        if should_flatten {
            // If flattening, we don't create a Taffy node for THIS element.
            // We just return its children to be added to the grandparent.
            return children;
        }

        let taffy_node = if let Some(node) = existing_node {
            if is_strictly_dirty {
                let _ = taffy.set_style(node, taffy_style);
            }
            // Only update children if list is potentially changed
            let is_children_dirty = {
                let dom_node = dom.get_node(node_idx).unwrap();
                dom_node.dirty.intersects(
                    crate::ace::engine::dom::NodeDirtyFlags::CHILDREN
                        | crate::ace::engine::dom::NodeDirtyFlags::LAYOUT,
                )
            };
            if is_children_dirty || taffy.children(node).unwrap_or_default().len() != children.len()
            {
                let _ = taffy.set_children(node, &children);
            }
            node
        } else {
            let node = taffy.new_with_children(taffy_style, &children).unwrap();
            self.node_to_taffy.lock().unwrap().insert(node_idx, node);
            node
        };

        node_map.insert(taffy_node, node_idx);
        vec![taffy_node]
    }
    pub(crate) fn convert_to_taffy_style(
        &self,
        style: &crate::ace::engine::style::css_values::ComputedStyle,
    ) -> taffy::prelude::Style {
        let mut t_style = taffy::prelude::Style::default();

        t_style.display = match style.display {
            crate::ace::engine::style::css_values::CssDisplay::Grid
            | crate::ace::engine::style::css_values::CssDisplay::Table => taffy::prelude::Display::Grid,
            crate::ace::engine::style::css_values::CssDisplay::Flex => taffy::prelude::Display::Flex,
            crate::ace::engine::style::css_values::CssDisplay::None => taffy::prelude::Display::None,
            crate::ace::engine::style::css_values::CssDisplay::Contents
            | crate::ace::engine::style::css_values::CssDisplay::TableRow
            | crate::ace::engine::style::css_values::CssDisplay::TableHeader => {
                taffy::prelude::Display::None
            } // Will be flattened/handled manually
            _ => taffy::prelude::Display::Flex,
        };

        t_style.flex_direction = match style.display {
            crate::ace::engine::style::css_values::CssDisplay::Block => {
                taffy::prelude::FlexDirection::Column
            }
            _ => match style.flex_direction {
                crate::ace::engine::style::css_values::CssFlexDirection::Row => {
                    taffy::prelude::FlexDirection::Row
                }
                crate::ace::engine::style::css_values::CssFlexDirection::Column => {
                    taffy::prelude::FlexDirection::Column
                }
                crate::ace::engine::style::css_values::CssFlexDirection::RowReverse => {
                    taffy::prelude::FlexDirection::RowReverse
                }
                crate::ace::engine::style::css_values::CssFlexDirection::ColumnReverse => {
                    taffy::prelude::FlexDirection::ColumnReverse
                }
            },
        };

        t_style.flex_wrap = match style.flex_wrap {
            crate::ace::engine::style::css_values::CssFlexWrap::NoWrap => {
                taffy::prelude::FlexWrap::NoWrap
            }
            crate::ace::engine::style::css_values::CssFlexWrap::Wrap => taffy::prelude::FlexWrap::Wrap,
            crate::ace::engine::style::css_values::CssFlexWrap::WrapReverse => {
                taffy::prelude::FlexWrap::WrapReverse
            }
        };

        t_style.position = match style.position {
            crate::ace::engine::style::css_values::CssPosition::Absolute
            | crate::ace::engine::style::css_values::CssPosition::Fixed => {
                taffy::prelude::Position::Absolute
            }
            _ => {
                if style.float != crate::ace::engine::style::css_values::CssFloat::None {
                    taffy::prelude::Position::Absolute
                } else {
                    taffy::prelude::Position::Relative
                }
            }
        };

        t_style.size = taffy::prelude::Size {
            width: self.to_taffy_dimension(&style.width),
            height: self.to_taffy_dimension(&style.height),
        };

        t_style.min_size = taffy::prelude::Size {
            width: self.to_taffy_dimension(&style.min_width),
            height: self.to_taffy_dimension(&style.min_height),
        };

        t_style.max_size = taffy::prelude::Size {
            width: self.to_taffy_dimension(&style.max_width),
            height: self.to_taffy_dimension(&style.max_height),
        };

        t_style.margin = taffy::prelude::Rect {
            left: self.to_taffy_length_percentage(&style.margin_left).into(),
            right: self.to_taffy_length_percentage(&style.margin_right).into(),
            top: self.to_taffy_length_percentage(&style.margin_top).into(),
            bottom: self.to_taffy_length_percentage(&style.margin_bottom).into(),
        };

        t_style.padding = taffy::prelude::Rect {
            left: self.to_taffy_length_percentage(&style.padding_left).into(),
            right: self.to_taffy_length_percentage(&style.padding_right).into(),
            top: self.to_taffy_length_percentage(&style.padding_top).into(),
            bottom: self
                .to_taffy_length_percentage(&style.padding_bottom)
                .into(),
        };

        t_style.gap = taffy::prelude::Size {
            width: self.to_taffy_length_percentage(&style.grid_column_gap),
            height: self.to_taffy_length_percentage(&style.grid_row_gap),
        };

        // Filter out LineNames before passing to Taffy
        t_style.grid_template_columns = style
            .grid_template_columns
            .iter()
            .filter(|l| !matches!(l, crate::ace::engine::style::css_values::CssLength::LineNames(_)))
            .map(|l| self.to_taffy_track_size(l))
            .collect();

        t_style.grid_template_rows = style
            .grid_template_rows
            .iter()
            .filter(|l| !matches!(l, crate::ace::engine::style::css_values::CssLength::LineNames(_)))
            .map(|l| self.to_taffy_track_size(l))
            .collect();

        // Resolve Placements
        t_style.grid_column.start = self.resolve_placement(&style.grid_column_start);
        t_style.grid_column.end = self.resolve_placement(&style.grid_column_end);
        t_style.grid_row.start = self.resolve_placement(&style.grid_row_start);
        t_style.grid_row.end = self.resolve_placement(&style.grid_row_end);

        t_style
    }
    pub(crate) fn to_taffy_dimension(
        &self,
        len: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::Dimension {
        use crate::ace::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::Dimension::Points(*v),
            CssLength::Percent(v) => taffy::prelude::Dimension::Percent(*v / 100.0),
            CssLength::Auto => taffy::prelude::Dimension::Auto,
            _ => taffy::prelude::Dimension::Auto,
        }
    }
    pub(crate) fn apply_grid_context(
        &self,
        t_style: &mut taffy::prelude::Style,
        style: &crate::ace::engine::style::css_values::ComputedStyle,
        ctx: &GridContext,
    ) {
        use crate::ace::engine::style::css_values::CssLength;

        let resolve_line =
            |l: &CssLength, names: &std::collections::HashMap<String, Vec<i16>>| -> Option<i16> {
                match l {
                    CssLength::Number(v) => Some(*v as i16),
                    CssLength::Name(n) => names.get(n).and_then(|v| v.first().copied()),
                    _ => None,
                }
            };

        if let Some(start) = resolve_line(&style.grid_column_start, &ctx.column_names) {
            t_style.grid_column.start =
                taffy::prelude::GridPlacement::Line((start + ctx.col_offset).into());
        }
        if let Some(end) = resolve_line(&style.grid_column_end, &ctx.column_names) {
            t_style.grid_column.end =
                taffy::prelude::GridPlacement::Line((end + ctx.col_offset).into());
        }
        if let Some(start) = resolve_line(&style.grid_row_start, &ctx.row_names) {
            t_style.grid_row.start =
                taffy::prelude::GridPlacement::Line((start + ctx.row_offset).into());
        }
        if let Some(end) = resolve_line(&style.grid_row_end, &ctx.row_names) {
            t_style.grid_row.end =
                taffy::prelude::GridPlacement::Line((end + ctx.row_offset).into());
        }

        // Apply grid-area name if it matches an area
        if let CssLength::Name(name) = &style.grid_row_start {
            if let Some(area) = ctx.areas.get(name) {
                t_style.grid_row.start =
                    taffy::prelude::GridPlacement::Line(((area.0 as i16) + ctx.row_offset).into());
                t_style.grid_row.end =
                    taffy::prelude::GridPlacement::Line(((area.1 as i16) + ctx.row_offset).into());
                t_style.grid_column.start =
                    taffy::prelude::GridPlacement::Line(((area.2 as i16) + ctx.col_offset).into());
                t_style.grid_column.end =
                    taffy::prelude::GridPlacement::Line(((area.3 as i16) + ctx.col_offset).into());
            }
        }
    }
    pub(crate) fn resolve_grid_names(
        &self,
        tracks: &[crate::ace::engine::style::css_values::CssLength],
    ) -> std::collections::HashMap<String, Vec<i16>> {
        let mut map = std::collections::HashMap::new();
        let mut line_idx = 0;
        for track in tracks {
            match track {
                crate::ace::engine::style::css_values::CssLength::LineNames(names) => {
                    for name in names {
                        map.entry(name.clone()).or_insert(Vec::new()).push(line_idx);
                    }
                }
                _ => {
                    line_idx += 1;
                }
            }
        }
        map
    }
    pub(crate) fn resolve_grid_areas(
        &self,
        areas: &[String],
    ) -> std::collections::HashMap<String, (usize, usize, usize, usize)> {
        let mut map = std::collections::HashMap::new();
        for (row_idx, row_str) in areas.iter().enumerate() {
            let cols: Vec<&str> = row_str.split_whitespace().collect();
            for (col_idx, area_name) in cols.iter().enumerate() {
                if *area_name == "." {
                    continue;
                }
                let entry = map.entry(area_name.to_string()).or_insert((
                    row_idx,
                    row_idx + 1,
                    col_idx,
                    col_idx + 1,
                ));
                entry.0 = entry.0.min(row_idx);
                entry.1 = entry.1.max(row_idx + 1);
                entry.2 = entry.2.min(col_idx);
                entry.3 = entry.3.max(col_idx + 1);
            }
        }
        map
    }
    pub(crate) fn to_taffy_length_percentage(
        &self,
        len: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::LengthPercentage {
        use crate::ace::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::LengthPercentage::Points(*v),
            CssLength::Percent(v) => taffy::prelude::LengthPercentage::Percent(*v / 100.0),
            _ => taffy::prelude::LengthPercentage::Points(0.0),
        }
    }
    pub(crate) fn to_taffy_track_size(
        &self,
        len: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::TrackSizingFunction {
        use crate::ace::engine::style::css_values::CssLength;
        match len {
            CssLength::Fr(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Auto,
                max: taffy::prelude::MaxTrackSizingFunction::Fraction(*v),
            }),
            CssLength::Px(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Points(*v),
                ),
                max: taffy::prelude::MaxTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Points(*v),
                ),
            }),
            CssLength::Percent(v) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Percent(*v / 100.0),
                ),
                max: taffy::prelude::MaxTrackSizingFunction::Fixed(
                    taffy::prelude::LengthPercentage::Percent(*v / 100.0),
                ),
            }),
            CssLength::MinMax(min, max) => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: self.to_taffy_min_track(min),
                max: self.to_taffy_max_track(max),
            }),
            CssLength::Repeat(count, sub) => {
                let repetition = match count.as_str() {
                    "auto-fill" => taffy::prelude::GridTrackRepetition::AutoFill,
                    "auto-fit" => taffy::prelude::GridTrackRepetition::AutoFit,
                    _ => {
                        let c = count.parse::<u16>().unwrap_or(1);
                        taffy::prelude::GridTrackRepetition::Count(c)
                    }
                };
                let tracks = sub
                    .iter()
                    .map(|l| match self.to_taffy_track_size(l) {
                        taffy::prelude::TrackSizingFunction::Single(mm) => mm,
                        _ => MinMax {
                            min: taffy::prelude::MinTrackSizingFunction::Auto,
                            max: taffy::prelude::MaxTrackSizingFunction::Auto,
                        },
                    })
                    .collect();
                taffy::prelude::TrackSizingFunction::Repeat(repetition, tracks)
            }
            _ => taffy::prelude::TrackSizingFunction::Single(MinMax {
                min: taffy::prelude::MinTrackSizingFunction::Auto,
                max: taffy::prelude::MaxTrackSizingFunction::Auto,
            }),
        }
    }
    pub(crate) fn to_taffy_min_track(
        &self,
        len: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::MinTrackSizingFunction {
        use crate::ace::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::MinTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Points(*v),
            ),
            CssLength::Percent(v) => taffy::prelude::MinTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Percent(*v / 100.0),
            ),
            CssLength::MinContent => taffy::prelude::MinTrackSizingFunction::MinContent,
            CssLength::MaxContent => taffy::prelude::MinTrackSizingFunction::MaxContent,
            _ => taffy::prelude::MinTrackSizingFunction::Auto,
        }
    }
    pub(crate) fn to_taffy_max_track(
        &self,
        len: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::MaxTrackSizingFunction {
        use crate::ace::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::MaxTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Points(*v),
            ),
            CssLength::Percent(v) => taffy::prelude::MaxTrackSizingFunction::Fixed(
                taffy::prelude::LengthPercentage::Percent(*v / 100.0),
            ),
            CssLength::Fr(v) => taffy::prelude::MaxTrackSizingFunction::Fraction(*v),
            CssLength::MinContent => taffy::prelude::MaxTrackSizingFunction::MinContent,
            CssLength::MaxContent => taffy::prelude::MaxTrackSizingFunction::MaxContent,
            _ => taffy::prelude::MaxTrackSizingFunction::Auto,
        }
    }
    pub(crate) fn _to_taffy_length_percentage_auto(
        &self,
        len: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::LengthPercentageAuto {
        use crate::ace::engine::style::css_values::CssLength;
        match len {
            CssLength::Px(v) => taffy::prelude::LengthPercentageAuto::Points(*v),
            CssLength::Percent(v) => taffy::prelude::LengthPercentageAuto::Percent(*v / 100.0),
            CssLength::Auto => taffy::prelude::LengthPercentageAuto::Auto,
            _ => taffy::prelude::LengthPercentageAuto::Points(0.0),
        }
    }
    pub(crate) fn resolve_placement(
        &self,
        placement: &crate::ace::engine::style::css_values::CssLength,
    ) -> taffy::prelude::GridPlacement {
        use crate::ace::engine::style::css_values::CssLength;
        match placement {
            CssLength::Number(v) => taffy::prelude::GridPlacement::Line((*v as i16).into()),
            CssLength::Span(v) => taffy::prelude::GridPlacement::Span(*v),
            CssLength::Name(_n) => {
                // This is a placeholder; real resolution happens in build_layout_tree
                // if we have parent context.
                taffy::prelude::GridPlacement::Auto
            }
            _ => taffy::prelude::GridPlacement::Auto,
        }
    }
    pub(crate) fn populate_node_map_recursively(
        &self,
        dom: &AceDOM,
        taffy: &taffy::Taffy,
        node: taffy::prelude::Node,
        node_idx: usize,
        node_map: &mut std::collections::HashMap<taffy::prelude::Node, usize>,
    ) {
        node_map.insert(node, node_idx);
        if let Ok(children) = taffy.children(node) {
            if let Some(dom_node) = dom.get_node(node_idx) {
                // Simplified 1:1 mapping for stable subtrees
                for (i, &taffy_child) in children.iter().enumerate() {
                    if i < dom_node.children.len() {
                        self.populate_node_map_recursively(
                            dom,
                            taffy,
                            taffy_child,
                            dom_node.children[i],
                            node_map,
                        );
                    }
                }
            }
        }
    }
}
