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



pub(crate) fn apply_typography(style: &mut ComputedStyle, name: &str, val: &str) {
    match name {
        "font-family" | "fontFamily" => {
            style.font_family = val.trim().trim_matches('\'').trim_matches('"').to_string()
        }
        "font-weight" | "fontWeight" => style.font_weight = parse_font_weight(val),
        "font-style" | "fontStyle" => style.font_style = val.to_string(),
        "line-height" | "lineHeight" => style.line_height = parse_length(val),
        "letter-spacing" | "letterSpacing" => style.letter_spacing = parse_length(val),
        "word-spacing" | "wordSpacing" => style.word_spacing = parse_length(val),
        "text-align" | "textAlign" => style.text_align = parse_text_align(val),
        _ => {}
    }
}
