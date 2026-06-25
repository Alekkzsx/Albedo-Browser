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





impl<'i> selectors::parser::Parser<'i> for AceSelectorParser {
pub(crate) type Impl = AceSelectorImpl;
pub(crate) type Error = selectors::parser::SelectorParseErrorKind<'i>;

pub(crate) fn parse_non_ts_pseudo_class(
        &self,
        location: SourceLocation,
        name: CowRcStr<'i>,
    ) -> Result<AceNonTSPseudoClass, ParseError<'i, Self::Error>> {
        let name_str = name.as_ref();
        match name_str {
            "hover" => Ok(AceNonTSPseudoClass::Hover),
            "focus" => Ok(AceNonTSPseudoClass::Focus),
            "active" => Ok(AceNonTSPseudoClass::Active),
            "first-child" => Ok(AceNonTSPseudoClass::FirstChild),
            "last-child" => Ok(AceNonTSPseudoClass::LastChild),
            "first-of-type" => Ok(AceNonTSPseudoClass::FirstOfType),
            "last-of-type" => Ok(AceNonTSPseudoClass::LastOfType),
            "only-child" => Ok(AceNonTSPseudoClass::OnlyChild),
            "link" => Ok(AceNonTSPseudoClass::Link),
            "visited" => Ok(AceNonTSPseudoClass::Visited),
            "empty" => Ok(AceNonTSPseudoClass::Empty),
            _ => Err(ParseError {
                kind: cssparser::ParseErrorKind::Custom(
                    selectors::parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement(
                        name,
                    ),
                ),
                location,
            }),
        }
    }

pub(crate) fn parse_non_ts_functional_pseudo_class(
        &self,
        name: CowRcStr<'i>,
        parser: &mut Parser<'i, '_>,
        _is_negated: bool,
    ) -> Result<AceNonTSPseudoClass, ParseError<'i, Self::Error>> {
        match name.as_ref() {
            "nth-child" => {
                let s = parser.expect_ident_or_string()?.to_string().to_lowercase();
                if s == "even" {
                    Ok(AceNonTSPseudoClass::NthChild(2, 0))
                } else if s == "odd" {
                    Ok(AceNonTSPseudoClass::NthChild(2, 1))
                } else if let Ok(n) = s.parse::<i32>() {
                    Ok(AceNonTSPseudoClass::NthChild(0, n))
                } else {
                    // RIGOROUS: Simplified an+b parser for now
                    // In a real engine we'd use cssparser::parse_nth
                    if s.contains('n') {
                        let parts: Vec<&str> = s.split('n').collect();
                        let a = if parts[0].is_empty() {
                            1
                        } else if parts[0] == "-" {
                            -1
                        } else {
                            parts[0].parse().unwrap_or(1)
                        };
                        let b = if parts.len() > 1 && !parts[1].is_empty() {
                            parts[1].parse().unwrap_or(0)
                        } else {
                            0
                        };
                        Ok(AceNonTSPseudoClass::NthChild(a, b))
                    } else {
                        Ok(AceNonTSPseudoClass::NthChild(0, 1))
                    }
                }
            }
            _ => Err(ParseError {
                kind: cssparser::ParseErrorKind::Custom(
                    selectors::parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement(
                        name,
                    ),
                ),
                location: parser.current_source_location(),
            }),
        }
    }

pub(crate) fn parse_pseudo_element(
        &self,
        location: SourceLocation,
        name: CowRcStr<'i>,
    ) -> Result<AcePseudoElement, ParseError<'i, Self::Error>> {
        match name.as_ref() {
            "before" => Ok(AcePseudoElement::Before),
            "after" => Ok(AcePseudoElement::After),
            "placeholder" => Ok(AcePseudoElement::Placeholder),
            "selection" => Ok(AcePseudoElement::Selection),
            "marker" => Ok(AcePseudoElement::Marker),
            _ => Err(ParseError {
                kind: cssparser::ParseErrorKind::Custom(
                    selectors::parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement(
                        name,
                    ),
                ),
                location,
            }),
        }
    }
}
