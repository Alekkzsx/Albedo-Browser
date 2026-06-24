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



pub(crate) fn matches_feature(feature: &str, vw: f32, vh: f32, color_scheme: &str) -> bool {
    // Remove outer parens
    let f = feature.trim();
    let f = if f.starts_with('(') && f.ends_with(')') {
        &f[1..f.len() - 1]
    } else {
        f
    };

    if let Some(idx) = f.find(':') {
        let name = f[..idx].trim();
        let value = f[idx + 1..].trim();

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
            "min-height" => vh >= get_px(val),
            "max-height" => vh <= get_px(val),
            "orientation" => {
                if val == "landscape" {
                    vw >= vh
                } else if val == "portrait" {
                    vh >= vw
                } else {
                    false
                }
            }
            "prefers-color-scheme" => val == color_scheme,
            _ => true,
        }
    } else {
        true
    }
}
