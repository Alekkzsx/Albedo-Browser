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





pub(crate) fn parse_transform(val: &str) -> Vec<TransformFunction> {
    let mut transforms = Vec::new();
    let value = value.trim();
    if val == "none" || val.is_empty() {
        return transforms;
    }

    // Simple parser for function(args)
    let mut i = 0;
    let chars: Vec<char> = val.chars().collect();
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        let start = i;
        while i < chars.len() && chars[i] != '(' {
            i += 1;
        }
        let func_name = val[start..i].trim();

        if i < chars.len() && chars[i] == '(' {
            i += 1;
            let arg_start = i;
            let mut brace_count = 1;
            while i < chars.len() && brace_count > 0 {
                if chars[i] == '(' {
                    brace_count += 1;
                } else if chars[i] == ')' {
                    brace_count -= 1;
                }
                i += 1;
            }
            let args_str = &val[arg_start..i - 1];
            let args: Vec<&str> = args_str.split(',').collect();

            match func_name {
                "translate" => {
                    if args.len() >= 2 {
                        transforms.push(TransformFunction::Translate(
                            parse_length(args[0]),
                            parse_length(args[1]),
                        ));
                    } else if args.len() == 1 {
                        transforms.push(TransformFunction::Translate(
                            parse_length(args[0]),
                            CssLength::Zero,
                        ));
                    }
                }
                "translateX" => {
                    if !args.is_empty() {
                        transforms.push(TransformFunction::TranslateX(parse_length(args[0])));
                    }
                }
                "translateY" => {
                    if !args.is_empty() {
                        transforms.push(TransformFunction::TranslateY(parse_length(args[0])));
                    }
                }
                "scale" => {
                    if args.len() >= 2 {
                        let sx = args[0].trim().parse().unwrap_or(1.0);
                        let sy = args[1].trim().parse().unwrap_or(sx);
                        transforms.push(TransformFunction::Scale(sx, sy));
                    } else if args.len() == 1 {
                        let s = args[0].trim().parse().unwrap_or(1.0);
                        transforms.push(TransformFunction::Scale(s, s));
                    }
                }
                "rotate" => {
                    if !args.is_empty() {
                        let deg_str = args[0].trim().strip_suffix("deg").unwrap_or(args[0].trim());
                        transforms.push(TransformFunction::Rotate(deg_str.parse().unwrap_or(0.0)));
                    }
                }
                "rotateX" => {
                    if !args.is_empty() {
                        let deg_str = args[0].trim().strip_suffix("deg").unwrap_or(args[0].trim());
                        transforms.push(TransformFunction::RotateX(deg_str.parse().unwrap_or(0.0)));
                    }
                }
                "rotateY" => {
                    if !args.is_empty() {
                        let deg_str = args[0].trim().strip_suffix("deg").unwrap_or(args[0].trim());
                        transforms.push(TransformFunction::RotateY(deg_str.parse().unwrap_or(0.0)));
                    }
                }
                "rotateZ" => {
                    if !args.is_empty() {
                        let deg_str = args[0].trim().strip_suffix("deg").unwrap_or(args[0].trim());
                        transforms.push(TransformFunction::RotateZ(deg_str.parse().unwrap_or(0.0)));
                    }
                }
                _ => {}
            }
        } else {
            i += 1;
        }
    }
    transforms
}
