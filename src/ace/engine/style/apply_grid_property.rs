use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use cssparser::{CowRcStr, ParseError, Parser, ParserInput, SourceLocation, ToCss};
use precomputed_hash::PrecomputedHash;
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching::{ElementSelectorFlags, MatchingContext, MatchingMode};
use selectors::OpaqueElement;
use std::collections::HashMap;
pub mod css_values;
use self::css_values::{
    BackgroundImage, BoxShadow, ComputedStyle, CssAlignContent, CssAlignItems, CssBlendMode,
    CssBoxSizing, CssClear, CssColor, CssContent, CssCursor, CssDisplay, CssFlexDirection,
    CssFlexWrap, CssFloat, CssFontWeight, CssJustifyContent, CssLength, CssObjectFit,
    CssObjectPosition, CssOverflow, CssPointerEvents, CssPosition, CssTextAlign, CssTextOverflow,
    CssTextTransform, CssVisibility, CssWhiteSpace, Gradient, GradientStop, TextShadow,
};
use crate::ace::engine::style::css_values::CssFilter;
use crate::ace::engine::style::css_values::TransformFunction;

pub mod animation;



pub(crate) fn apply_grid_property(style: &mut ComputedStyle, name: &str, val: &str) {
    match name {
        "grid-template-columns" | "gridTemplateColumns" => {
            style.grid_template_columns = parse_grid_track_list(val)
        }
        "grid-template-rows" | "gridTemplateRows" => {
            style.grid_template_rows = parse_grid_track_list(val)
        }
        "grid-template-areas" | "gridTemplateAreas" => {
            style.grid_template_areas = parse_grid_template_areas(val)
        }
        "grid-column-gap" | "column-gap" => style.grid_column_gap = parse_length(val),
        "grid-row-gap" | "row-gap" => style.grid_row_gap = parse_length(val),
        "gap" => {
            let parts = split_spaces_top_level(val);
            if parts.len() == 1 {
                let gap = parse_length(parts[0]);
                style.grid_row_gap = gap.clone();
                style.grid_column_gap = gap;
            } else if parts.len() >= 2 {
                style.grid_row_gap = parse_length(parts[0]);
                style.grid_column_gap = parse_length(parts[1]);
            }
        }
        "grid-column-start" | "gridColumnStart" => {
            style.grid_column_start = parse_grid_placement(val)
        }
        "grid-column-end" | "gridColumnEnd" => style.grid_column_end = parse_grid_placement(val),
        "grid-row-start" | "gridRowStart" => style.grid_row_start = parse_grid_placement(val),
        "grid-row-end" | "gridRowEnd" => style.grid_row_end = parse_grid_placement(val),
        "grid-column" | "gridColumn" => {
            let parts: Vec<&str> = val.split('/').collect();
            if parts.len() == 1 {
                style.grid_column_start = parse_grid_placement(parts[0]);
                style.grid_column_end = CssLength::Auto;
            } else if parts.len() >= 2 {
                style.grid_column_start = parse_grid_placement(parts[0]);
                style.grid_column_end = parse_grid_placement(parts[1]);
            }
        }
        "grid-row" | "gridRow" => {
            let parts: Vec<&str> = val.split('/').collect();
            if parts.len() == 1 {
                style.grid_row_start = parse_grid_placement(parts[0]);
                style.grid_row_end = CssLength::Auto;
            } else if parts.len() >= 2 {
                style.grid_row_start = parse_grid_placement(parts[0]);
                style.grid_row_end = parse_grid_placement(parts[1]);
            }
        }
        "grid-area" | "gridArea" => {
            let parts: Vec<&str> = val.split('/').collect();
            if parts.len() == 1 {
                let name = parse_grid_placement(parts[0]);
                style.grid_row_start = name.clone();
                style.grid_column_start = name.clone();
                style.grid_row_end = name.clone();
                style.grid_column_end = name;
            } else if parts.len() >= 4 {
                style.grid_row_start = parse_grid_placement(parts[0]);
                style.grid_column_start = parse_grid_placement(parts[1]);
                style.grid_row_end = parse_grid_placement(parts[2]);
                style.grid_column_end = parse_grid_placement(parts[3]);
            }
        }
        _ => {}
    }
}
