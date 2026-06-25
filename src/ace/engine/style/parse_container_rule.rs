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





/// Analisa uma regra de container `@container` e adiciona ao stylesheet correspondente.
pub(crate) fn parse_container_rule(block: &str, stylesheet: &mut Stylesheet) {
    if let Some(query_end) = block.find('{') {
        let full_query = block[10..query_end].trim();
        let (name, condition) = if let Some(n_end) = full_query.find(' ') {
            (
                Some(full_query[..n_end].trim().to_string()),
                full_query[n_end..].trim().to_string(),
            )
        } else {
            (None, full_query.to_string())
        };
        let content = &block[query_end + 1..block.len() - 1];
        let container_stylesheet = parse_simple(content);
        stylesheet.container_rules.push(AceContainerRule {
            name,
            condition,
            rules: container_stylesheet.rules,
        });
    }
}
