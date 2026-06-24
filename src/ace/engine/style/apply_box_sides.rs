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



pub(crate) fn apply_box_sides(style: &mut ComputedStyle, name: &str, val: &str, resolve_rel: impl Fn(CssLength) -> CssLength) {
    match name {
        "margin-top" => style.margin_top = resolve_rel(parse_length(val)),
        "margin-right" => style.margin_right = resolve_rel(parse_length(val)),
        "margin-bottom" => style.margin_bottom = resolve_rel(parse_length(val)),
        "margin-left" => style.margin_left = resolve_rel(parse_length(val)),
        "margin" => apply_margin_shorthand(style, val, &resolve_rel),
        "padding-top" => style.padding_top = resolve_rel(parse_length(val)),
        "padding-right" => style.padding_right = resolve_rel(parse_length(val)),
        "padding-bottom" => style.padding_bottom = resolve_rel(parse_length(val)),
        "padding-left" => style.padding_left = resolve_rel(parse_length(val)),
        "padding" => apply_padding_shorthand(style, val, &resolve_rel),
        _ => {}
    }
}
