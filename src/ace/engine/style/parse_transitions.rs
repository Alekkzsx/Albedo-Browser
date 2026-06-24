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



pub(crate) fn parse_transitions(val: &str) -> Vec<self::css_values::CssTransition> {
    let mut transitions = Vec::new();
    for part in val.split(',') {
        let segments: Vec<&str> = part.trim().split_whitespace().collect();
        if segments.is_empty() {
            continue;
        }

        let mut t = self::css_values::CssTransition {
            property: segments[0].to_string(),
            duration_ms: 0,
            timing_function: "ease".to_string(),
            delay_ms: 0,
        };

        for segment in &segments[1..] {
            if segment.ends_with("ms") {
                if let Ok(ms) = segment.trim_end_matches("ms").parse::<u32>() {
                    if t.duration_ms == 0 {
                        t.duration_ms = ms;
                    } else {
                        t.delay_ms = ms;
                    }
                }
            } else if segment.ends_with('s') {
                if let Ok(s) = segment.trim_end_matches('s').parse::<f32>() {
                    let ms = (s * 1000.0) as u32;
                    if t.duration_ms == 0 {
                        t.duration_ms = ms;
                    } else {
                        t.delay_ms = ms;
                    }
                }
            } else {
                t.timing_function = segment.to_string();
            }
        }
        transitions.push(t);
    }
    transitions
}
