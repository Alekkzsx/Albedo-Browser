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





/// Analisa uma folha de estilo CSS e retorna o `Stylesheet` correspondente.
pub fn parse(source: &str) -> Stylesheet {
    let mut stylesheet = get_user_agent_stylesheet();
    let mut remaining_source = source.to_string();

    // Processa regras @ de nível superior
    while let Some(index) = remaining_source.find('@') {
        let before = &remaining_source[..index];
        if !before.trim().is_empty() {
            let before_stylesheet = parse_simple(before);
            stylesheet.rules.extend(before_stylesheet.rules);
        }

        let rest = &remaining_source[index..];
        if let Some(brace_start) = rest.find('{') {
            let mut brace_count = 1;
            let mut block_end = brace_start + 1;
            while block_end < rest.len() && brace_count > 0 {
                if rest[block_end..].starts_with('{') {
                    brace_count += 1;
                } else if rest[block_end..].starts_with('}') {
                    brace_count -= 1;
                }
                block_end += 1;
            }

            let block = &rest[..block_end];
            if rest.starts_with("@media") {
                parse_media_rule(block, &mut stylesheet);
            } else if rest.starts_with("@supports") {
                parse_supports_rule(block, &mut stylesheet);
            } else if rest.starts_with("@container") {
                parse_container_rule(block, &mut stylesheet);
            } else if rest.starts_with("@font-face") {
                parse_font_face_rule(block, &mut stylesheet);
            } else if rest.starts_with("@keyframes") {
                parse_keyframes_rule(block, &mut stylesheet);
            }

            remaining_source = rest[block_end..].to_string();
        } else {
            // Provavelmente uma regra @ única sem bloco ou erro
            remaining_source = rest[1..].to_string();
        }
    }

    // Analisa as regras regulares restantes
    if !remaining_source.trim().is_empty() {
        let remaining_stylesheet = parse_simple(&remaining_source);
        stylesheet.rules.extend(remaining_stylesheet.rules);
    }

    stylesheet
}
