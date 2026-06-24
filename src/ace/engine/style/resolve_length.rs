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



// Helper to resolve lengths to pixels
pub(crate) fn resolve_length(
    length: &CssLength,
    parent_font_size: f32,
    root_font_size: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> f32 {
    match length {
        CssLength::Px(v) => *v,
        CssLength::Em(v) => v * parent_font_size,
        CssLength::Rem(v) => v * root_font_size,
        CssLength::Percent(v) => v / 100.0 * parent_font_size, // Default to parent font size for font-size property, caller handles context
        CssLength::Vw(v) => v / 100.0 * viewport_width,
        CssLength::Vh(v) => v / 100.0 * viewport_height,
        // For others, return 0 or default
        _ => 0.0,
    }
}
