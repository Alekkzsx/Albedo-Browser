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





pub(crate) fn apply_outline_property(style: &mut ComputedStyle, name: &str, val: &str, current_font_size: f32) {
    match name {
        "outline-width" | "outlineWidth" => {
            let w = resolve_length(&parse_length(val), current_font_size, 16.0, 0.0, 0.0);
            let mut o = style.outline.clone().unwrap_or_else(|| {
                let mut d = default_outline();
                d.width = 0.0;
                d
            });
            o.width = w;
            style.outline = Some(o);
        }
        "outline-color" | "outlineColor" => {
            let c = parse_color(val);
            let mut o = style.outline.clone().unwrap_or(default_outline());
            o.color = c;
            style.outline = Some(o);
        }
        "outline-style" | "outlineStyle" => {
            if val.trim() == "none" {
                style.outline = None;
            } else {
                let mut o = style.outline.clone().unwrap_or_else(|| {
                    let mut d = default_outline();
                    d.style = "none".into();
                    d
                });
                o.style = val.trim().to_string();
                style.outline = Some(o);
            }
        }
        "outline-offset" | "outlineOffset" => {
            let off = resolve_length(&parse_length(val), current_font_size, 16.0, 0.0, 0.0);
            let mut o = style.outline.clone().unwrap_or(default_outline());
            o.offset = off;
            style.outline = Some(o);
        }
        _ => {}
    }
}
