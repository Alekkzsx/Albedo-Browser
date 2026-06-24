use super::*;

use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::style::Stylesheet;
use crate::ace::engine::layout::ElementGeometry;
use crate::utils::time::unix_timestamp_secs_f64;
use taffy::geometry::MinMax;
use crate::ace::engine::core::AceEngine;



impl AceEngine {
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
}
