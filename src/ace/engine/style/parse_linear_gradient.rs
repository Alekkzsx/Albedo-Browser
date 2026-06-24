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



pub(crate) fn parse_linear_gradient(val: &str) -> Option<Gradient> {
    let inner = val.strip_prefix("linear-gradient(")?.strip_suffix(')')?;

    let mut angle = 180.0f32; // Default is to bottom (180 degrees)
    let mut stops = Vec::new();

    // Parse angle if present
    let mut remaining = inner;
    if remaining.starts_with("to ") {
        // Handle "to right", "to bottom right", etc.
        let parts: Vec<&str> = remaining.split_whitespace().collect();
        if parts.len() >= 2 {
            let direction = parts[1];
            angle = match direction {
                "top" => 0.0,
                "right" => 90.0,
                "bottom" => 180.0,
                "left" => 270.0,
                _ => {
                    if parts.len() >= 3 {
                        let dir2 = parts[2];
                        match (direction, dir2) {
                            ("top", "left") => 315.0,
                            ("top", "right") => 45.0,
                            ("bottom", "left") => 225.0,
                            ("bottom", "right") => 135.0,
                            _ => 180.0,
                        }
                    } else {
                        180.0
                    }
                }
            };
            remaining = remaining.splitn(2, ')').nth(1).unwrap_or("");
        }
    } else if remaining.starts_with("deg") {
        if let Some(deg) = remaining.split_whitespace().next() {
            if let Some(n) = deg.strip_suffix("deg") {
                if let Ok(a) = n.parse::<f32>() {
                    angle = a;
                }
            }
        }
        // Find the first color stop
        if let Some(idx) = remaining.find(',') {
            remaining = &remaining[idx + 1..];
        }
    } else if let Some(idx) = remaining.find(',') {
        // Check if first part is angle in degrees
        let first = remaining[..idx].trim();
        if let Ok(a) = first.parse::<f32>() {
            angle = a;
            remaining = &remaining[idx + 1..];
        }
    }

    // Parse color stops
    parse_color_stops(remaining, &mut stops);

    if stops.is_empty() {
        // Add default stops
        stops.push(GradientStop {
            color: CssColor::Transparent,
            position: Some(0.0),
        });
        stops.push(GradientStop {
            color: CssColor::Named("black".to_string()),
            position: Some(1.0),
        });
    }

    Some(Gradient::Linear { angle, stops })
}
