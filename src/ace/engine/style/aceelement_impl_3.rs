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



impl<'a> selectors::Element for AceElement<'a> {

pub(crate) fn match_pseudo_element(
        &self,
        _pe: &<Self::Impl as selectors::parser::SelectorImpl>::PseudoElement,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

pub(crate) fn apply_selector_flags(&self, _flags: ElementSelectorFlags) {}

pub(crate) fn is_link(&self) -> bool {
        self.has_local_name(&AceIdent("a".to_string()))
    }

pub(crate) fn has_id(
        &self,
        id: &<Self::Impl as selectors::parser::SelectorImpl>::Identifier,
        case_sensitivity: CaseSensitivity,
    ) -> bool {
        if let Some(node) = self.dom.get_node(self.index) {
            if let AceNodeType::Element(el) = &node.node_type {
                if let Some(val) = el.attributes.get("id") {
                    return case_sensitivity.eq(val.as_bytes(), id.as_bytes());
                }
            }
        }
        false
    }

pub(crate) fn has_class(
        &self,
        name: &<Self::Impl as selectors::parser::SelectorImpl>::Identifier,
        case_sensitivity: CaseSensitivity,
    ) -> bool {
        if let Some(node) = self.dom.get_node(self.index) {
            if let AceNodeType::Element(el) = &node.node_type {
                if let Some(val) = el.attributes.get("class") {
                    return val
                        .split_whitespace()
                        .any(|c| case_sensitivity.eq(c.as_bytes(), name.as_bytes()));
                }
            }
        }
        false
    }

pub(crate) fn is_part(&self, _name: &<Self::Impl as selectors::parser::SelectorImpl>::Identifier) -> bool {
        false
    }
pub(crate) fn imported_part(
        &self,
        _name: &<Self::Impl as selectors::parser::SelectorImpl>::Identifier,
    ) -> Option<<Self::Impl as selectors::parser::SelectorImpl>::Identifier> {
        None
    }

pub(crate) fn parent_node_is_shadow_root(&self) -> bool {
        false
    }
pub(crate) fn containing_shadow_host(&self) -> Option<Self> {
        None
    }
pub(crate) fn is_pseudo_element(&self) -> bool {
        false
    }
pub(crate) fn is_html_slot_element(&self) -> bool {
        false
    }
pub(crate) fn has_custom_state(&self, _name: &AceIdent) -> bool {
        false
    }
pub(crate) fn add_element_unique_hashes(&self, _filter: &mut selectors::bloom::BloomFilter) -> bool {
        false
    }
}
