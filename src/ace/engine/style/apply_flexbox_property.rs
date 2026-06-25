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





pub(crate) fn apply_flexbox_property(style: &mut ComputedStyle, name: &str, val: &str) {
    match name {
        "flex-direction" | "flexDirection" => style.flex_direction = parse_flex_direction(val),
        "justify-content" | "justifyContent" => style.justify_content = parse_justify_content(val),
        "align-items" | "alignItems" => style.align_items = parse_align_items(val),
        "flex-wrap" | "flexWrap" => style.flex_wrap = parse_flex_wrap(val),
        "flex-grow" | "flexGrow" => {
            if let Ok(n) = val.parse::<f32>() {
                style.flex_grow = n;
            }
        }
        "flex-shrink" | "flexShrink" => {
            if let Ok(n) = val.parse::<f32>() {
                style.flex_shrink = n;
            }
        }
        "flex-basis" | "flexBasis" => style.flex_basis = parse_length(val),
        _ => {}
    }
}
