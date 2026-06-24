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



pub(crate) fn expand_box_shorthand(
    prefix: &str,
    value: &str,
    important: bool,
) -> Vec<Declaration> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let suffixes = ["top", "right", "bottom", "left"];
    match parts.len() {
        1 => suffixes
            .iter()
            .map(|s| Declaration {
                name: format!("{}-{}", prefix, s),
                value: parts[0].to_string(),
                important,
            })
            .collect(),
        2 => vec![
            Declaration { name: format!("{}-top", prefix), value: parts[0].to_string(), important },
            Declaration { name: format!("{}-bottom", prefix), value: parts[0].to_string(), important },
            Declaration { name: format!("{}-right", prefix), value: parts[1].to_string(), important },
            Declaration { name: format!("{}-left", prefix), value: parts[1].to_string(), important },
        ],
        4 => vec![
            Declaration { name: format!("{}-top", prefix), value: parts[0].to_string(), important },
            Declaration { name: format!("{}-right", prefix), value: parts[1].to_string(), important },
            Declaration { name: format!("{}-bottom", prefix), value: parts[2].to_string(), important },
            Declaration { name: format!("{}-left", prefix), value: parts[3].to_string(), important },
        ],
        _ => vec![Declaration {
            name: prefix.to_string(),
            value: value.to_string(),
            important,
        }],
    }
}
