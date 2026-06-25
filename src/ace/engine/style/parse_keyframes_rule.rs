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





/// Analisa uma regra de animação `@keyframes` e adiciona ao stylesheet correspondente.
pub(crate) fn parse_keyframes_rule(block: &str, stylesheet: &mut Stylesheet) {
    if let Some(name_end) = block[10..].find('{') {
        let name = block[10..10 + name_end].trim().to_string();
        let content = &block[10 + name_end + 1..block.len() - 1];
        let mut keyframes = Vec::new();
        let mut input = ParserInput::new(content);
        let mut p = Parser::new(&mut input);
        while !p.is_exhausted() {
            let pct_str = match p.next() {
                Ok(cssparser::Token::Percentage { unit_value, .. }) => {
                    (unit_value * 100.0).to_string()
                }
                Ok(cssparser::Token::Ident(s)) if *s == "from" => "0".to_string(),
                Ok(cssparser::Token::Ident(s)) if *s == "to" => "100".to_string(),
                _ => {
                    let _ = p.next();
                    continue;
                }
            };
            if p.expect_curly_bracket_block().is_ok() {
                let pct = pct_str.parse::<f32>().unwrap_or(0.0);
                let decls: std::collections::HashMap<String, String> = p
                    .parse_nested_block(|inner_p| {
                        let mut map = HashMap::new();
                        while !inner_p.is_exhausted() {
                            if let Ok(name) = inner_p.expect_ident() {
                                let name_str = name.to_string();
                                if inner_p.expect_colon().is_ok() {
                                    let mut value = String::new();
                                    while let Ok(t) = inner_p.next() {
                                        value.push_str(&t.to_css_string());
                                    }
                                    map.insert(
                                        name_str,
                                        value.trim_end_matches(';').trim().to_string(),
                                    );
                                }
                            } else {
                                let _ = inner_p.next();
                            }
                        }
                        Ok::<
                            std::collections::HashMap<String, String>,
                            cssparser::ParseError<'_, cssparser::BasicParseErrorKind<'_>>,
                        >(map)
                    })
                    .unwrap_or_default();
                keyframes.push(self::css_values::CssKeyframe {
                    percentage: pct,
                    declarations: decls,
                });
            }
        }
        stylesheet.keyframes.insert(name, keyframes);
    }
}
