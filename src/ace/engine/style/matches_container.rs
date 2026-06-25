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





pub(crate) fn matches_container(condition: &str, vw: f32) -> bool {
    // Basic stub: use vw for container size as a fallback
    let c = condition.trim();
    let c = if c.starts_with('(') && c.ends_with(')') {
        &c[1..c.len() - 1]
    } else {
        c
    };
    if let Some(idx) = c.find(':') {
        let name = c[..idx].trim();
        let value = c[idx + 1..].trim();
        let get_px = |v: &str| -> f32 {
            if let Some(p) = v.strip_suffix("px") {
                p.parse::<f32>().unwrap_or(0.0)
            } else {
                0.0
            }
        };
        match name {
            "min-width" => vw >= get_px(val),
            "max-width" => vw <= get_px(val),
            _ => true,
        }
    } else {
        true
    }
}
