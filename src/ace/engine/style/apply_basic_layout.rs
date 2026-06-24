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



pub(crate) fn apply_basic_layout(style: &mut ComputedStyle, name: &str, val: &str, resolve_rel: impl Fn(CssLength) -> CssLength) {
    match name {
        "display" => style.display = parse_display(val),
        "position" => style.position = parse_position(val),
        "overflow" => style.overflow = parse_overflow(val),
        "float" => style.float = parse_float(val),
        "clear" => style.clear = parse_clear(val),
        "z-index" | "zIndex" => {
            if val == "auto" {
                style.z_index = i32::MIN;
            } else if let Ok(n) = val.parse::<i32>() {
                style.z_index = n;
            }
        }
        "width" => style.width = resolve_rel(parse_length(val)),
        "height" => style.height = resolve_rel(parse_length(val)),
        "top" => style.top = resolve_rel(parse_length(val)),
        "right" => style.right = resolve_rel(parse_length(val)),
        "bottom" => style.bottom = resolve_rel(parse_length(val)),
        "left" => style.left = resolve_rel(parse_length(val)),
        _ => {}
    }
}
