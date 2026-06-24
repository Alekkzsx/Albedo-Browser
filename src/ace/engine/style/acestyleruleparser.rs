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



impl<'i> cssparser::QualifiedRuleParser<'i> for AceStyleRuleParser {
pub(crate) type Prelude = selectors::SelectorList<AceSelectorImpl>;
pub(crate) type QualifiedRule = AceRule;
pub(crate) type Error = selectors::parser::SelectorParseErrorKind<'i>;

pub(crate) fn parse_prelude<'t>(
        &mut self,
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::Prelude, cssparser::ParseError<'i, Self::Error>> {
        selectors::SelectorList::parse(
            &AceSelectorParser,
            input,
            selectors::parser::ParseRelative::No,
        )
    }

pub(crate) fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start_location: &cssparser::ParserState,
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, cssparser::ParseError<'i, Self::Error>> {
        let mut decls = Vec::new();
        while !input.is_exhausted() {
            let ident_res = input.expect_ident();
            tracing::debug!(?ident_res, "expect_ident result");
            if let Ok(name) = ident_res {
                let name = name.to_string();
                let colon_res = input.expect_colon();
                tracing::debug!(?colon_res, "expect_colon result");
                if colon_res.is_ok() {
                    let mut value_raw = String::new();
                    while !input.is_exhausted() {
                        match input.next() {
                            Ok(cssparser::Token::Semicolon) => {
                                tracing::debug!("Semicolon token hit");
                                break;
                            }
                            Ok(token) => {
                                let tok_str = token.to_css_string();
                                tracing::debug!(token = %tok_str, "Value token");
                                value_raw.push_str(&tok_str);
                            }
                            Err(e) => {
                                tracing::debug!(?e, "Error token in inner loop");
                                break;
                            }
                        }
                    }

                    // Clean up value and detect !important
                    let mut important = false;
                    let mut value = value_raw.trim_end_matches(';').trim().to_string();
                    if value.to_lowercase().ends_with("!important") {
                        important = true;
                        value = value[..value.len() - 10].trim().to_string();
                    }

                    // Expand shorthands (Simple implementation)
                    match name.as_str() {
                        "margin" => { decls.extend(expand_box_shorthand("margin", &value, important)); }
                        "padding" => { decls.extend(expand_box_shorthand("padding", &value, important)); }
                        "border" => { decls.extend(expand_border(&value, important)); }
                        "outline" => { decls.extend(expand_outline(&value, important)); }
                        "background" => { decls.extend(expand_background(&value, important)); }
                        "font" => { decls.extend(expand_font(&value, important)); }
                        _ => decls.push(Declaration { name, value, important }),
                    }
                    continue;
                }
            }
            let _ = input.next();
        }

        tracing::debug!(decl_count = decls.len(), "Returning QualifiedRule");
        Ok(AceRule {
            selectors: prelude,
            declarations: decls,
            order: 0,
        })
    }
}

impl<'i> cssparser::AtRuleParser<'i> for AceStyleRuleParser {
pub(crate) type Prelude = ();
pub(crate) type AtRule = AceRule;
pub(crate) type Error = selectors::parser::SelectorParseErrorKind<'i>;

pub(crate) fn parse_prelude<'t>(
        &mut self,
        name: cssparser::CowRcStr<'i>,
        _input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::Prelude, cssparser::ParseError<'i, Self::Error>> {
        Err(cssparser::ParseError {
            kind: cssparser::ParseErrorKind::Custom(
                selectors::parser::SelectorParseErrorKind::UnexpectedIdent(name.clone()),
            ),
            location: cssparser::SourceLocation { line: 0, column: 0 },
        })
    }

pub(crate) fn parse_block<'t>(
        &mut self,
        _prelude: Self::Prelude,
        _start_location: &cssparser::ParserState,
        _input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::AtRule, cssparser::ParseError<'i, Self::Error>> {
        unreachable!()
    }
}
