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





pub(crate) fn expand_font(value: &str, important: bool) -> Vec<Declaration> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let mut decls = Vec::new();
    for part in parts {
        if part == "italic" || part == "oblique" {
            decls.push(Declaration { name: "font-style".to_string(), value: part.to_string(), important });
        } else if part == "bold" || part == "bolder" || part == "lighter" || part.parse::<f32>().is_ok() {
            decls.push(Declaration { name: "font-weight".to_string(), value: part.to_string(), important });
        } else if part.ends_with("px") || part.ends_with("em") || part.ends_with("rem")
            || part.ends_with('%') || part.ends_with("pt")
        {
            decls.push(Declaration { name: "font-size".to_string(), value: part.to_string(), important });
        } else {
            decls.push(Declaration { name: "font-family".to_string(), value: part.to_string(), important });
        }
    }
    decls
}
