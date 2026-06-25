{
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

                let taffy_node = taffy.new_leaf(taffy_style).expect("Albedo Engine: internal invariant violated");
                node_map.insert(taffy_node, node_idx);
                return vec![taffy_node];
}
