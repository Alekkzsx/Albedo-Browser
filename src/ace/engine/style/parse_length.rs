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





// Parsing Helpers
pub(crate) fn parse_length(val: &str) -> CssLength {
    let value = value.trim();
    if val == "auto" {
        return CssLength::Auto;
    }
    if val == "0" {
        return CssLength::Zero;
    }

    if val.starts_with("clamp(") && val.ends_with(")") {
        let inner = &val[6..val.len() - 1];
        let parts = split_comma_top_level(inner);
        if parts.len() == 3 {
            return CssLength::Clamp(
                Box::new(parse_length(parts[0])),
                Box::new(parse_length(parts[1])),
                Box::new(parse_length(parts[2])),
            );
        }
    }

    if val.starts_with("min(") && val.ends_with(")") {
        let inner = &val[4..val.len() - 1];
        let parts = split_comma_top_level(inner);
        return CssLength::Min(parts.iter().map(|p| parse_length(p)).collect());
    }

    if val.starts_with("max(") && val.ends_with(")") {
        let inner = &val[4..val.len() - 1];
        let parts = split_comma_top_level(inner);
        return CssLength::Max(parts.iter().map(|p| parse_length(p)).collect());
    }

    if val.starts_with("calc(") && val.ends_with(")") {
        let inner = &val[5..val.len() - 1];
        // Support simple A + B or A - B if units are same
        if inner.contains(" + ")
            || inner.contains(" - ")
            || inner.contains(" * ")
            || inner.contains(" / ")
        {
            // Very basic calc parser for same-unit additions
            let parts: Vec<&str> = if inner.contains(" + ") {
                inner.split(" + ").collect()
            } else if inner.contains(" - ") {
                inner.split(" - ").collect()
            } else if inner.contains(" * ") {
                inner.split(" * ").collect()
            } else {
                inner.split(" / ").collect()
            };

            if parts.len() == 2 {
                let l1 = parse_length(parts[0].trim());
                let l2 = parse_length(parts[1].trim());
                if let (CssLength::Px(v1), CssLength::Px(v2)) = (&l1, &l2) {
                    if inner.contains(" + ") {
                        return CssLength::Px(v1 + v2);
                    }
                    if inner.contains(" - ") {
                        return CssLength::Px(v1 - v2);
                    }
                }
                // Handle multiplication/division by unitless number
                if let CssLength::Px(v1) = l1 {
                    if let Ok(v2) = parts[1].trim().parse::<f32>() {
                        if inner.contains(" * ") {
                            return CssLength::Px(v1 * v2);
                        }
                        if inner.contains(" / ") && v2 != 0.0 {
                            return CssLength::Px(v1 / v2);
                        }
                    }
                }
            }
        }
        return CssLength::Calc(inner.to_string());
    }

    if let Some(n) = val.strip_suffix("px") {
        if let Ok(num) = n.trim().parse::<f32>() {
            return CssLength::Px(num);
        }
    }
    if let Some(n) = val.strip_suffix("%") {
        if let Ok(num) = n.trim().parse::<f32>() {
            return CssLength::Percent(num);
        }
    }
    if let Some(n) = val.strip_suffix("vw") {
        if let Ok(num) = n.trim().parse::<f32>() {
            return CssLength::Vw(num);
        }
    }
    if let Some(n) = val.strip_suffix("vh") {
        if let Ok(num) = n.trim().parse::<f32>() {
            return CssLength::Vh(num);
        }
    }
    if let Some(n) = val.strip_suffix("rem") {
        if let Ok(num) = n.trim().parse::<f32>() {
            return CssLength::Rem(num);
        }
    }
    if let Some(n) = val.strip_suffix("em") {
        if let Ok(num) = n.trim().parse::<f32>() {
            return CssLength::Em(num);
        }
    }
    if let Some(n) = val.strip_suffix("fr") {
        if let Ok(num) = n.trim().parse::<f32>() {
            return CssLength::Fr(num);
        }
    }

    if val == "min-content" {
        return CssLength::MinContent;
    }
    if val == "max-content" {
        return CssLength::MaxContent;
    }
    if val == "auto-fill" {
        return CssLength::AutoFill;
    }
    if val == "auto-fit" {
        return CssLength::AutoFit;
    }

    CssLength::Auto
}
