use super::*;
use std::fmt;



/// Resolve CSS length to pixels given parent and root font sizes
pub fn resolve_length(
    length: &CssLength,
    parent_font_size: f32,
    root_font_size: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> f32 {
    match length {
        CssLength::Px(v) => *v,
        CssLength::Percent(v) => *v / 100.0, // This needs context - usually handled by layout engine
        CssLength::Vw(v) => *v / 100.0 * viewport_width,
        CssLength::Vh(v) => *v / 100.0 * viewport_height,
        CssLength::Rem(v) => *v * root_font_size,
        CssLength::Em(v) => *v * parent_font_size,
        CssLength::Fr(v) => *v,
        CssLength::Zero => 0.0,
        CssLength::Auto => 0.0,
        CssLength::Number(v) => *v,
        CssLength::Subgrid => 0.0,
        CssLength::Span(v) => *v as f32,
        CssLength::Name(_) | CssLength::LineNames(_) => 0.0,
        CssLength::Clamp(min, val, max) => {
            let min_v = resolve_length(
                min,
                parent_font_size,
                root_font_size,
                viewport_width,
                viewport_height,
            );
            let val_v = resolve_length(
                val,
                parent_font_size,
                root_font_size,
                viewport_width,
                viewport_height,
            );
            let max_v = resolve_length(
                max,
                parent_font_size,
                root_font_size,
                viewport_width,
                viewport_height,
            );
            val_v.max(min_v).min(max_v)
        }
        CssLength::Min(vals) => vals
            .iter()
            .map(|v| {
                resolve_length(
                    v,
                    parent_font_size,
                    root_font_size,
                    viewport_width,
                    viewport_height,
                )
            })
            .fold(f32::INFINITY, f32::min),
        CssLength::Max(vals) => vals
            .iter()
            .map(|v| {
                resolve_length(
                    v,
                    parent_font_size,
                    root_font_size,
                    viewport_width,
                    viewport_height,
                )
            })
            .fold(f32::NEG_INFINITY, f32::max),
        CssLength::MinMax(min, max) => {
            let _min_v = resolve_length(
                min,
                parent_font_size,
                root_font_size,
                viewport_width,
                viewport_height,
            );
            let max_v = resolve_length(
                max,
                parent_font_size,
                root_font_size,
                viewport_width,
                viewport_height,
            );
            // Rough approximation: use max for now
            max_v
        }
        CssLength::Repeat(_count, sub) => {
            // Rough approximation: resolve first element * 3 (arbitrary)
            if let Some(first) = sub.first() {
                resolve_length(
                    first,
                    parent_font_size,
                    root_font_size,
                    viewport_width,
                    viewport_height,
                ) * 3.0
            } else {
                0.0
            }
        }
        CssLength::MinContent
        | CssLength::MaxContent
        | CssLength::AutoFill
        | CssLength::AutoFit => 0.0, // Needs layout context
        CssLength::Calc(_) => 0.0, // Needs complex parser
    }
}
