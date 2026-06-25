use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use cssparser::{CowRcStr, ParseError, Parser, ParserInput, SourceLocation, ToCss};
use precomputed_hash::PrecomputedHash;
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching::{ElementSelectorFlags, MatchingContext, MatchingMode};
use selectors::OpaqueElement;
use std::collections::HashMap;
use crate::ace::engine::style::css_values::{
    BackgroundImage, BoxShadow, ComputedStyle, CssAlignContent, CssAlignItems, CssBlendMode,
    CssBoxSizing, CssClear, CssColor, CssContent, CssCursor, CssDisplay, CssFlexDirection,
    CssFlexWrap, CssFloat, CssFontWeight, CssJustifyContent, CssLength, CssObjectFit,
    CssObjectPosition, CssOverflow, CssPointerEvents, CssPosition, CssTextAlign, CssTextOverflow,
    CssTextTransform, CssVisibility, CssWhiteSpace, Gradient, GradientStop, TextShadow,
};
use crate::ace::engine::style::css_values::CssFilter;
use crate::ace::engine::style::css_values::TransformFunction;





pub(crate) fn parse_grid_track_list(val: &str) -> Vec<CssLength> {
    let parts = split_spaces_top_level(val);
    let mut tracks = Vec::new();

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if part == "subgrid" {
            tracks.push(CssLength::Subgrid);
            continue;
        }

        if part.starts_with('[') && part.ends_with(']') {
            let names = part[1..part.len() - 1]
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            tracks.push(CssLength::LineNames(names));
            continue;
        }

        if part.starts_with("repeat(") && part.ends_with(")") {
            let inner = &part[7..part.len() - 1];
            let args = split_comma_top_level(inner);
            if args.len() >= 2 {
                let count_str = args[0].trim();
                let track_str = args[1].trim();

                let repeat_mode = count_str.to_string();
                let sub_tracks = parse_grid_track_list(track_str);

                if let Ok(n) = count_str.parse::<usize>() {
                    for _ in 0..n {
                        tracks.extend(sub_tracks.clone());
                    }
                } else {
                    tracks.push(CssLength::Repeat(repeat_mode, sub_tracks));
                }
            }
        } else if part.starts_with("minmax(") && part.ends_with(")") {
            let inner = &part[7..part.len() - 1];
            let args = split_comma_top_level(inner);
            if args.len() == 2 {
                let min = parse_length(args[0]);
                let max = parse_length(args[1]);
                tracks.push(CssLength::MinMax(Box::new(min), Box::new(max)));
            }
        } else {
            tracks.push(parse_length(part));
        }
    }
    tracks
}
