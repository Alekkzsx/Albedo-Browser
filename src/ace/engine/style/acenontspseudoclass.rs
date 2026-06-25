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
pub enum AceNonTSPseudoClass {
    Hover,
    Focus,
    Active,
    FirstChild,
    LastChild,
    NthChild(i32, i32), // a, b
    FirstOfType,
    LastOfType,
    OnlyChild,
    Link,
    Visited,
    Empty,
}

impl AceNonTSPseudoClass {
    /// TODO: add docs
    pub fn name(&self) -> &str {
        match self {
            AceNonTSPseudoClass::Hover => "hover",
            AceNonTSPseudoClass::Focus => "focus",
            AceNonTSPseudoClass::Active => "active",
            AceNonTSPseudoClass::FirstChild => "first-child",
            AceNonTSPseudoClass::LastChild => "last-child",
            AceNonTSPseudoClass::NthChild(_, _) => "nth-child",
            AceNonTSPseudoClass::FirstOfType => "first-of-type",
            AceNonTSPseudoClass::LastOfType => "last-of-type",
            AceNonTSPseudoClass::OnlyChild => "only-child",
            AceNonTSPseudoClass::Link => "link",
            AceNonTSPseudoClass::Visited => "visited",
            AceNonTSPseudoClass::Empty => "empty",
        }
    }
}

impl selectors::parser::NonTSPseudoClass for AceNonTSPseudoClass {
pub(crate) type Impl = AceSelectorImpl;
pub(crate) fn is_active_or_hover(&self) -> bool {
        matches!(
            self,
            AceNonTSPseudoClass::Hover | AceNonTSPseudoClass::Active
        )
    }
pub(crate) fn is_user_action_state(&self) -> bool {
        matches!(
            self,
            AceNonTSPseudoClass::Hover | AceNonTSPseudoClass::Focus | AceNonTSPseudoClass::Active
        )
    }
}
impl ToCss for AceNonTSPseudoClass {
pub(crate) fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result
    where
        W: std::fmt::Write,
    {
        dest.write_str(self.name())
    }
}
