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





pub(crate) fn apply_padding_shorthand(style: &mut ComputedStyle, val: &str, resolve_rel: impl Fn(CssLength) -> CssLength) {
    let parts: Vec<&str> = val.split_whitespace().collect();
    match parts.len() {
        1 => {
            let p = resolve_rel(parse_length(parts[0]));
            style.padding_top = p.clone();
            style.padding_right = p.clone();
            style.padding_bottom = p.clone();
            style.padding_left = p;
        }
        2 => {
            let v = resolve_rel(parse_length(parts[0]));
            let h = resolve_rel(parse_length(parts[1]));
            style.padding_top = v.clone();
            style.padding_bottom = v;
            style.padding_right = h.clone();
            style.padding_left = h;
        }
        4 => {
            style.padding_top = resolve_rel(parse_length(parts[0]));
            style.padding_right = resolve_rel(parse_length(parts[1]));
            style.padding_bottom = resolve_rel(parse_length(parts[2]));
            style.padding_left = resolve_rel(parse_length(parts[3]));
        }
        _ => {}
    }
}
