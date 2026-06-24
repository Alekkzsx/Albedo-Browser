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



pub(crate) fn matches_media_query_part(part: &str, vw: f32, vh: f32, color_scheme: &str) -> bool {
    let normalized = part.to_lowercase();
    // Helper to handle "and" splitting safely would be better, but simple split acts as "good enough" for now
    let segments: Vec<&str> = normalized.split(" and ").collect();

    for (i, segment) in segments.iter().enumerate() {
        let seg = segment.trim();
        if i == 0 {
            // First segment might have modifier/type
            let mut s = seg;
            let mut negate = false;

            if s.starts_with("not ") {
                negate = true;
                s = &s[4..].trim();
            } else if s.starts_with("only ") {
                s = &s[5..].trim();
            }

            let seg_match = if s.starts_with('(') {
                matches_feature(s, vw, vh, color_scheme)
            } else {
                s == "screen" || s == "all"
            };

            if negate == seg_match {
                return false;
            }
        } else {
            // Subsequent segments are features
            if !matches_feature(seg, vw, vh, color_scheme) {
                return false;
            }
        }
    }
    true
}
