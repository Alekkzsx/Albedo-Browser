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





// MELHORIA: Parse box-shadow
pub(crate) fn parse_box_shadow(val: &str) -> Vec<BoxShadow> {
    let value = value.trim();
    if val == "none" || val.is_empty() {
        return Vec::new();
    }

    let mut shadows = Vec::new();

    // Split by comma for multiple shadows
    for shadow_val in val.split(',') {
        let shadow_val = shadow_val.trim();
        if shadow_val.is_empty() || shadow_val == "none" {
            continue;
        }

        let mut offset_x = 0.0f32;
        let mut offset_y = 0.0f32;
        let mut blur = 0.0f32;
        let mut spread = 0.0f32;
        let mut color = CssColor::Named("black".to_string());
        let mut inset = false;

        let parts: Vec<&str> = shadow_val.split_whitespace().collect();
        let mut i = 0;

        while i < parts.len() {
            let part = parts[i];

            // Check for inset keyword
            if part == "inset" {
                inset = true;
                i += 1;
                continue;
            }

            // Try to parse as length
            if let Some(n) = part.strip_suffix("px") {
                if let Ok(num) = n.parse::<f32>() {
                    if i == 0 || (i > 0 && parts.get(i - 1) == Some(&"inset")) {
                        offset_x = num;
                        i += 1;
                        if i < parts.len() {
                            if let Some(n2) = parts[i].strip_suffix("px") {
                                if let Ok(num2) = n2.parse::<f32>() {
                                    offset_y = num2;
                                    i += 1;
                                    continue;
                                }
                            }
                        }
                    } else if i > 0 {
                        if let Ok(num2) = part.parse::<f32>() {
                            if blur == 0.0 {
                                blur = num2;
                            } else {
                                spread = num2;
                            }
                            i += 1;
                            continue;
                        }
                    }
                }
            }

            // If not a number, might be a color
            if !part.ends_with("px") && !part.ends_with("em") && !part.ends_with("rem") {
                color = parse_color(part);
            }
            i += 1;
        }

        shadows.push(BoxShadow {
            offset_x,
            offset_y,
            blur,
            spread,
            color,
            inset,
        });
    }

    shadows
}
