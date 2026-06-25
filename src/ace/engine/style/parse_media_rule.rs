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





/// Analisa uma regra de mídia `@media` e adiciona ao stylesheet correspondente.
pub(crate) fn parse_media_rule(block: &str, stylesheet: &mut Stylesheet) {
    if let Some(query_end) = block.find('{') {
        let media_query = block[6..query_end].trim();
        let media_content = &block[query_end + 1..block.len() - 1];
        let media_stylesheet = parse_simple(media_content);
        stylesheet.media_rules.push(AceMediaRule {
            media_query: media_query.to_string(),
            rules: media_stylesheet.rules,
        });
    }
}
