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





pub(crate) fn expand_background(value: &str, important: bool) -> Vec<Declaration> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let mut decls = Vec::new();
    for part in parts {
        if part.starts_with("url(") || part.starts_with("linear-gradient(") || part.starts_with("radial-gradient(") {
            decls.push(Declaration { name: "background-image".to_string(), value: part.to_string(), important });
        } else if part == "no-repeat" || part == "repeat" || part == "repeat-x" || part == "repeat-y" {
            decls.push(Declaration { name: "background-repeat".to_string(), value: part.to_string(), important });
        } else if part == "center" || part == "top" || part == "bottom" || part == "left" || part == "right"
            || part.ends_with('%') || part.ends_with("px")
        {
            decls.push(Declaration { name: "background-position".to_string(), value: part.to_string(), important });
        } else {
            decls.push(Declaration { name: "background-color".to_string(), value: part.to_string(), important });
        }
    }
    decls
}
