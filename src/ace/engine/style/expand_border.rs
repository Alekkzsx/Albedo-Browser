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



pub(crate) fn expand_border(value: &str, important: bool) -> Vec<Declaration> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let mut decls = Vec::new();
    for part in parts {
        let (prop, val) = if part.ends_with("px")
            || part.ends_with("em")
            || part.ends_with("rem")
            || part == "0"
            || part == "thin"
            || part == "medium"
            || part == "thick"
        {
            ("width", part)
        } else if part == "solid"
            || part == "dashed"
            || part == "dotted"
            || part == "double"
            || part == "none"
        {
            ("style", part)
        } else {
            ("color", part)
        };
        for suffix in &["top", "right", "bottom", "left"] {
            decls.push(Declaration {
                name: format!("border-{}-{}", suffix, prop),
                value: part.to_string(),
                important,
            });
        }
    }
    decls
}
