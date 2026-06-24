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



pub(crate) fn parse_align_content(val: &str) -> CssAlignContent {
    match val.trim() {
        "flex-start" => CssAlignContent::FlexStart,
        "flex-end" => CssAlignContent::FlexEnd,
        "center" => CssAlignContent::Center,
        "space-between" => CssAlignContent::SpaceBetween,
        "space-around" => CssAlignContent::SpaceAround,
        "space-evenly" => CssAlignContent::SpaceEvenly,
        "stretch" => CssAlignContent::Stretch,
        _ => CssAlignContent::Stretch,
    }
}
