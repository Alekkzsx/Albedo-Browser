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





impl selectors::parser::SelectorImpl for AceSelectorImpl {
pub(crate) type ExtraMatchingData<'a> = ();
pub(crate) type AttrValue = AceIdent;
pub(crate) type Identifier = AceIdent;
pub(crate) type LocalName = AceIdent;
pub(crate) type NamespaceUrl = AceIdent;
pub(crate) type NamespacePrefix = AceIdent;
pub(crate) type BorrowedNamespaceUrl = AceIdent;
pub(crate) type BorrowedLocalName = AceIdent;

pub(crate) type PseudoElement = AcePseudoElement;
pub(crate) type NonTSPseudoClass = AceNonTSPseudoClass;
}
