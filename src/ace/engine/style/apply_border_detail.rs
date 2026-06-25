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





pub(crate) fn apply_border_detail(style: &mut ComputedStyle, name: &str, val: &str) {
    match name {
        "border-top-width" | "borderTopWidth" => style.border_width_top = parse_length(val),
        "border-right-width" | "borderRightWidth" => style.border_width_right = parse_length(val),
        "border-bottom-width" | "borderBottomWidth" => style.border_width_bottom = parse_length(val),
        "border-left-width" | "borderLeftWidth" => style.border_width_left = parse_length(val),
        "border-top-color" | "borderTopColor" => style.border_color_top = parse_color(val),
        "border-right-color" | "borderRightColor" => style.border_color_right = parse_color(val),
        "border-bottom-color" | "borderBottomColor" => style.border_color_bottom = parse_color(val),
        "border-left-color" | "borderLeftColor" => style.border_color_left = parse_color(val),
        _ => {}
    }
}
