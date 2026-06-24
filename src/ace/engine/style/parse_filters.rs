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



pub(crate) fn parse_filters(val: &str) -> Vec<CssFilter> {
    let mut filters = Vec::new();
    let value = value.trim();
    if val == "none" || val.is_empty() {
        return filters;
    }

    // Simple parser for filter functions: blur(5px) grayscale(50%) etc.
    let mut i = 0;
    let chars: Vec<char> = val.chars().collect();
    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == ',') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        let start = i;
        while i < chars.len() && chars[i] != '(' {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        let func_name = val[start..i].trim();
        i += 1; // skip '('

        let arg_start = i;
        let mut depth = 1;
        while i < chars.len() && depth > 0 {
            if chars[i] == '(' {
                depth += 1;
            } else if chars[i] == ')' {
                depth -= 1;
            }
            i += 1;
        }
        if depth == 0 {
            let arg = &val[arg_start..i - 1].trim();
            match func_name {
                "blur" => filters.push(CssFilter::Blur(parse_length(arg))),
                "brightness" => {
                    if let Ok(n) = arg.strip_suffix('%').unwrap_or(arg).parse::<f32>() {
                        filters.push(CssFilter::Brightness(if arg.ends_with('%') {
                            n / 100.0
                        } else {
                            n
                        }));
                    }
                }
                "grayscale" => {
                    if let Ok(n) = arg.strip_suffix('%').unwrap_or(arg).parse::<f32>() {
                        filters.push(CssFilter::Grayscale(if arg.ends_with('%') {
                            n / 100.0
                        } else {
                            n
                        }));
                    }
                }
                "invert" => {
                    if let Ok(n) = arg.strip_suffix('%').unwrap_or(arg).parse::<f32>() {
                        filters.push(CssFilter::Invert(if arg.ends_with('%') {
                            n / 100.0
                        } else {
                            n
                        }));
                    }
                }
                "sepia" => {
                    if let Ok(n) = arg.strip_suffix('%').unwrap_or(arg).parse::<f32>() {
                        filters.push(CssFilter::Sepia(if arg.ends_with('%') {
                            n / 100.0
                        } else {
                            n
                        }));
                    }
                }
                "opacity" => {
                    if let Ok(n) = arg.strip_suffix('%').unwrap_or(arg).parse::<f32>() {
                        filters.push(CssFilter::Opacity(if arg.ends_with('%') {
                            n / 100.0
                        } else {
                            n
                        }));
                    }
                }
                _ => {}
            }
        }
    }
    filters
}
