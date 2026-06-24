            // If this is a subgrid, we use the parent's context but with offsets
            let parent_ctx = parent_grid_ctx.expect("Albedo Engine: internal invariant violated");
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
