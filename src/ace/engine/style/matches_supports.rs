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





pub(crate) fn matches_supports(condition: &str) -> bool {
    let c = condition.trim();
    let c = if c.starts_with('(') && c.ends_with(')') {
        &c[1..c.len() - 1]
    } else {
        c
    };
    if let Some(idx) = c.find(':') {
        let name = c[..idx].trim();
        match name {
            "display"
            | "color"
            | "background-color"
            | "width"
            | "height"
            | "position"
            | "flex-direction"
            | "grid-template-columns"
            | "aspect-ratio"
            | "object-fit"
            | "filter"
            | "backdrop-filter"
            | "mix-blend-mode"
            | "transition"
            | "animation"
            | "pointer-events"
            | "cursor"
            | "visibility"
            | "opacity"
            | "z-index"
            | "box-shadow"
            | "transform" => true,
            _ => false,
        }
    } else {
        true
    }
}
