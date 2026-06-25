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





pub(crate) fn apply_margin_shorthand(style: &mut ComputedStyle, val: &str, resolve_rel: impl Fn(CssLength) -> CssLength) {
    let parts: Vec<&str> = val.split_whitespace().collect();
    match parts.len() {
        1 => {
            let m = resolve_rel(parse_length(parts[0]));
            style.margin_top = m.clone();
            style.margin_right = m.clone();
            style.margin_bottom = m.clone();
            style.margin_left = m;
        }
        2 => {
            let v = resolve_rel(parse_length(parts[0]));
            let h = resolve_rel(parse_length(parts[1]));
            style.margin_top = v.clone();
            style.margin_bottom = v;
            style.margin_right = h.clone();
            style.margin_left = h;
        }
        4 => {
            style.margin_top = resolve_rel(parse_length(parts[0]));
            style.margin_right = resolve_rel(parse_length(parts[1]));
            style.margin_bottom = resolve_rel(parse_length(parts[2]));
            style.margin_left = resolve_rel(parse_length(parts[3]));
        }
        _ => {}
    }
}
