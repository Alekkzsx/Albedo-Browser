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



pub(crate) fn parse_animations(val: &str) -> Vec<self::css_values::CssAnimation> {
    let mut animations = Vec::new();
    for part in val.split(',') {
        let segments: Vec<&str> = part.trim().split_whitespace().collect();
        if segments.is_empty() {
            continue;
        }

        let mut anim = self::css_values::CssAnimation {
            name: segments[0].to_string(),
            duration_ms: 0,
            timing_function: "ease".to_string(),
            delay_ms: 0,
            iteration_count: "1".to_string(),
            direction: "normal".to_string(),
            fill_mode: "none".to_string(),
        };

        for segment in &segments[1..] {
            if segment.ends_with("ms") || segment.ends_with('s') {
                let ms = if segment.ends_with("ms") {
                    segment.trim_end_matches("ms").parse::<u32>().unwrap_or(0)
                } else {
                    (segment.trim_end_matches('s').parse::<f32>().unwrap_or(0.0) * 1000.0) as u32
                };
                if anim.duration_ms == 0 {
                    anim.duration_ms = ms;
                } else {
                    anim.delay_ms = ms;
                }
            } else if *segment == "infinite" || segment.chars().all(|c| c.is_ascii_digit()) {
                anim.iteration_count = segment.to_string();
            } else if matches!(
                *segment,
                "normal" | "reverse" | "alternate" | "alternate-reverse"
            ) {
                anim.direction = segment.to_string();
            } else if matches!(*segment, "none" | "forwards" | "backwards" | "both") {
                anim.fill_mode = segment.to_string();
            } else if matches!(
                *segment,
                "ease"
                    | "linear"
                    | "ease-in"
                    | "ease-out"
                    | "ease-in-out"
                    | "step-start"
                    | "step-end"
            ) {
                anim.timing_function = segment.to_string();
            } else {
                // Could be name if first wasn't or additional prop
            }
        }
        animations.push(anim);
    }
    animations
}
