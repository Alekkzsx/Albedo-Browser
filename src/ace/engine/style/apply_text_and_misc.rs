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



pub(crate) fn apply_text_and_misc(style: &mut ComputedStyle, name: &str, val: &str) {
    match name {
        "text-transform" => style.text_transform = parse_text_transform(val),
        "text-overflow" => style.text_overflow = parse_text_overflow(val),
        "white-space" | "whiteSpace" => {
            style.white_space = match val {
                "normal" => CssWhiteSpace::Normal,
                "nowrap" => CssWhiteSpace::NoWrap,
                "pre" => CssWhiteSpace::Pre,
                "pre-wrap" => CssWhiteSpace::PreWrap,
                "pre-line" => CssWhiteSpace::PreLine,
                _ => CssWhiteSpace::Normal,
            };
        }
        "background-color" | "background" => style.background_color = parse_color(val),
        "color" => style.color = parse_color(val),
        "border-radius" => style.border_radius_top_left = parse_border_radius(val),
        "box-shadow" => style.box_shadow = parse_box_shadow(val),
        "text-shadow" => style.text_shadow = parse_text_shadow(val),
        "background-image" => style.background_image = parse_background_image(val),
        "content" => style.content = parse_content(val),
        "aspect-ratio" => apply_aspect_ratio(style, val),
        "box-sizing" | "boxSizing" => style.box_sizing = parse_box_sizing(val),
        "visibility" => style.visibility = parse_visibility(val),
        "cursor" => style.cursor = parse_cursor(val),
        "pointer-events" | "pointerEvents" => style.pointer_events = parse_pointer_events(val),
        "transform" => style.transform = parse_transform(val),
        _ => {}
    }
}
