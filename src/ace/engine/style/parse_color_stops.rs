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





pub(crate) fn parse_color_stops(input: &str, stops: &mut Vec<GradientStop>) {
    let parts: Vec<&str> = input.split(',').collect();

    for (i, part) in parts.iter().enumerate() {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let mut color = CssColor::Named("black".to_string());
        let mut position = None;

        // Split by space to find color and position
        let tokens: Vec<&str> = part.split_whitespace().collect();
        let has_tokens = !tokens.is_empty();

        for token in &tokens {
            if token.ends_with("%") {
                if let Ok(p) = token.strip_suffix("%").unwrap_or("").parse::<f32>() {
                    position = Some(p / 100.0);
                }
            } else if !token.is_empty() {
                color = parse_color(token);
            }
        }

        // If no position specified, calculate based on index
        if position.is_none() && has_tokens {
            position = Some(i as f32 / parts.len() as f32);
        }

        stops.push(GradientStop { color, position });
    }
}
