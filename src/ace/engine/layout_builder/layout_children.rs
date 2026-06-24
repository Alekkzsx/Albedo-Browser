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
