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



pub(crate) fn parse_color(val: &str) -> CssColor {
    let value = value.trim();
    match val {
        "transparent" => CssColor::Transparent,
        "currentcolor" => CssColor::CurrentColor,
        _ => {
            // Handle rgba() and rgb()
            if val.starts_with("rgba(") {
                let inner = &val[5..val.len() - 1];
                let parts: Vec<&str> = inner.split(',').collect();
                if parts.len() >= 4 {
                    let r = parts[0].trim().parse::<u8>().unwrap_or(0);
                    let g = parts[1].trim().parse::<u8>().unwrap_or(0);
                    let b = parts[2].trim().parse::<u8>().unwrap_or(0);
                    let a = parts[3].trim().parse::<f32>().unwrap_or(1.0);
                    return CssColor::Rgba(r, g, b, a);
                }
            }
            if val.starts_with("rgb(") {
                let inner = &val[4..val.len() - 1];
                let parts: Vec<&str> = inner.split(',').collect();
                if parts.len() >= 3 {
                    let r = parts[0].trim().parse::<u8>().unwrap_or(0);
                    let g = parts[1].trim().parse::<u8>().unwrap_or(0);
                    let b = parts[2].trim().parse::<u8>().unwrap_or(0);
                    return CssColor::Rgba(r, g, b, 1.0);
                }
            }
            // Handle hex colors
            if val.starts_with('#') && val.len() == 7 {
                let r = u8::from_str_radix(&val[1..3], 16).unwrap_or(0);
                let g = u8::from_str_radix(&val[3..5], 16).unwrap_or(0);
                let b = u8::from_str_radix(&val[5..7], 16).unwrap_or(0);
                return CssColor::Rgba(r, g, b, 1.0);
            }
            CssColor::Named(val.to_string())
        }
    }
}
