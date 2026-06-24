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



pub(crate) fn parse_radial_gradient(val: &str) -> Option<Gradient> {
    let inner = val.strip_prefix("radial-gradient(")?.strip_suffix(')')?;

    let mut shape = "circle".to_string();
    let mut stops = Vec::new();

    // Parse shape
    if inner.contains("circle") {
        shape = "circle".to_string();
    } else if inner.contains("ellipse") {
        shape = "ellipse".to_string();
    }

    // Find the color stops (after shape specification)
    let remaining = if let Some(idx) = inner.find(',') {
        &inner[idx + 1..]
    } else {
        inner
    };

    parse_color_stops(remaining, &mut stops);

    if stops.is_empty() {
        stops.push(GradientStop {
            color: CssColor::Transparent,
            position: Some(0.0),
        });
        stops.push(GradientStop {
            color: CssColor::Named("black".to_string()),
            position: Some(1.0),
        });
    }

    Some(Gradient::Radial { shape, stops })
}
