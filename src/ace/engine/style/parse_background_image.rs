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



// MELHORIA: Parse background-image (supports gradients)
pub(crate) fn parse_background_image(val: &str) -> BackgroundImage {
    let value = value.trim();

    if val == "none" || val.is_empty() {
        return BackgroundImage::None;
    }

    // Check for linear-gradient
    if val.starts_with("linear-gradient(") {
        if let Some(gradient) = parse_linear_gradient(val) {
            return BackgroundImage::Gradient(gradient);
        }
    }

    // Check for radial-gradient
    if val.starts_with("radial-gradient(") {
        if let Some(gradient) = parse_radial_gradient(val) {
            return BackgroundImage::Gradient(gradient);
        }
    }

    // Check for url()
    if val.starts_with("url(") {
        if let Some(end) = val.find(')') {
            let url = &val[4..end];
            return BackgroundImage::Url(url.to_string());
        }
    }

    // Check if it's a solid color
    if !val.contains("gradient") && !val.contains("url(") {
        return BackgroundImage::Color(parse_color(val));
    }

    BackgroundImage::None
}
