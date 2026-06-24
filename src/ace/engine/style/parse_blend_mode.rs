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



pub(crate) fn parse_blend_mode(val: &str) -> CssBlendMode {
    match val.trim() {
        "multiply" => CssBlendMode::Multiply,
        "screen" => CssBlendMode::Screen,
        "overlay" => CssBlendMode::Overlay,
        "darken" => CssBlendMode::Darken,
        "lighten" => CssBlendMode::Lighten,
        "color-dodge" => CssBlendMode::ColorDodge,
        "color-burn" => CssBlendMode::ColorBurn,
        "hard-light" => CssBlendMode::HardLight,
        "soft-light" => CssBlendMode::SoftLight,
        "difference" => CssBlendMode::Difference,
        "exclusion" => CssBlendMode::Exclusion,
        "hue" => CssBlendMode::Hue,
        "saturation" => CssBlendMode::Saturation,
        "color" => CssBlendMode::Color,
        "luminosity" => CssBlendMode::Luminosity,
        _ => CssBlendMode::Normal,
    }
}
