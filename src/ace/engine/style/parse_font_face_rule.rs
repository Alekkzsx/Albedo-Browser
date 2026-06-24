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



/// Analisa uma regra de fonte `@font-face` e adiciona ao stylesheet correspondente.
pub(crate) fn parse_font_face_rule(block: &str, stylesheet: &mut Stylesheet) {
    if let Some(query_end) = block.find('{') {
        let content = &block[query_end + 1..block.len() - 1];
        let mut props = HashMap::new();
        let mut input = ParserInput::new(content);
        let mut p = Parser::new(&mut input);
        while !p.is_exhausted() {
            if let Ok(name) = p.expect_ident() {
                let name_str = name.to_string();
                if p.expect_colon().is_ok() {
                    let mut value = String::new();
                    while let Ok(t) = p.next() {
                        value.push_str(&t.to_css_string());
                    }
                    props.insert(name_str, value.trim_end_matches(';').trim().to_string());
                }
            } else {
                let _ = p.next();
            }
        }
        stylesheet.font_faces.push(props);
    }
}
