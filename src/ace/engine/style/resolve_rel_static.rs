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





pub(crate) fn resolve_rel_static(l: CssLength, current_font_size: f32, root_font_size: f32) -> CssLength {
    match l {
        CssLength::Em(v) => CssLength::Px(v * current_font_size),
        CssLength::Rem(v) => CssLength::Px(v * root_font_size),
        CssLength::Clamp(min, val, max) => CssLength::Clamp(
            Box::new(resolve_rel_static(*min, current_font_size, root_font_size)),
            Box::new(resolve_rel_static(*val, current_font_size, root_font_size)),
            Box::new(resolve_rel_static(*max, current_font_size, root_font_size)),
        ),
        CssLength::Min(vals) => CssLength::Min(
            vals.into_iter()
                .map(|v| resolve_rel_static(v, current_font_size, root_font_size))
                .collect(),
        ),
        CssLength::Max(vals) => CssLength::Max(
            vals.into_iter()
                .map(|v| resolve_rel_static(v, current_font_size, root_font_size))
                .collect(),
        ),
        _ => l,
    }
}
