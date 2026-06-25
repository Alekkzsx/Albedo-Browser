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





pub(crate) fn expand_outline(value: &str, important: bool) -> Vec<Declaration> {
    let val_trimmed = value.trim();
    if val_trimmed == "none" || val_trimmed == "0" {
        return vec![Declaration {
            name: "outline-style".to_string(),
            value: "none".to_string(),
            important,
        }];
    }
    let parts: Vec<&str> = value.split_whitespace().collect();
    let mut decls = Vec::new();
    for part in parts {
        if part.ends_with("px") || part.ends_with("em") || part.ends_with("rem")
            || part == "thin" || part == "medium" || part == "thick"
        {
            decls.push(Declaration { name: "outline-width".to_string(), value: part.to_string(), important });
        } else if matches!(part, "none"|"solid"|"dashed"|"dotted"|"double"|"groove"|"ridge"|"inset"|"outset"|"auto") {
            decls.push(Declaration { name: "outline-style".to_string(), value: part.to_string(), important });
        } else {
            decls.push(Declaration { name: "outline-color".to_string(), value: part.to_string(), important });
        }
    }
    decls
}
