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





#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcePseudoElement {
    Before,
    After,
    Placeholder,
    Selection,
    Marker,
}

impl AcePseudoElement {
    /// TODO: add docs
    pub fn name(&self) -> &str {
        match self {
            AcePseudoElement::Before => "before",
            AcePseudoElement::After => "after",
            AcePseudoElement::Placeholder => "placeholder",
            AcePseudoElement::Selection => "selection",
            AcePseudoElement::Marker => "marker",
        }
    }
}

impl selectors::parser::PseudoElement for AcePseudoElement {
pub(crate) type Impl = AceSelectorImpl;
}

impl ToCss for AcePseudoElement {
pub(crate) fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result
    where
        W: std::fmt::Write,
    {
        write!(dest, "::{}", self.name())
    }
}
