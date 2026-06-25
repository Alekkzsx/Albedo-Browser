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





pub(crate) fn inherit_from_parent(parent_style: Option<&ComputedStyle>) -> ComputedStyle {
    if let Some(parent) = parent_style {
        let mut s = ComputedStyle::default();
        s.color = parent.color.clone();
        s.font_size = parent.font_size.clone();
        s.font_family = parent.font_family.clone();
        s.font_weight = parent.font_weight.clone();
        s.text_align = parent.text_align.clone();
        s.line_height = parent.line_height.clone();
        s.letter_spacing = parent.letter_spacing.clone();
        s.word_spacing = parent.word_spacing.clone();
        s.opacity = parent.opacity;
        s.visibility = parent.visibility.clone();
        s.cursor = parent.cursor.clone();
        s.pointer_events = parent.pointer_events.clone();
        s.text_transform = parent.text_transform.clone();
        s.text_overflow = parent.text_overflow.clone();
        s.white_space = parent.white_space.clone();
        s.custom_properties = parent.custom_properties.clone();
        s
    } else {
        ComputedStyle::default()
    }
}
