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



/// Analisa uma regra de suporte `@supports` e adiciona ao stylesheet correspondente.
pub(crate) fn parse_supports_rule(block: &str, stylesheet: &mut Stylesheet) {
    if let Some(query_end) = block.find('{') {
        let condition = block[9..query_end].trim();
        let content = &block[query_end + 1..block.len() - 1];
        let supports_stylesheet = parse_simple(content);
        stylesheet.supports_rules.push(AceSupportsRule {
            condition: condition.to_string(),
            rules: supports_stylesheet.rules,
        });
    }
}
