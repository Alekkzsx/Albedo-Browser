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

pub(crate) fn is_root(&self) -> bool {
        if let Some(node) = self.dom.get_node(self.index) {
            return node.parent.is_none();
        }
        false
    }

pub(crate) fn is_html_element_in_html_document(&self) -> bool {
        true
    }

pub(crate) fn has_local_name(
        &self,
        name: &<Self::Impl as selectors::parser::SelectorImpl>::BorrowedLocalName,
    ) -> bool {
        if let Some(node) = self.dom.get_node(self.index) {
            if let AceNodeType::Element(el) = &node.node_type {
                return el.tag == name.0;
            }
        }
        false
    }

pub(crate) fn has_namespace(
        &self,
        _ns: &<Self::Impl as selectors::parser::SelectorImpl>::BorrowedNamespaceUrl,
    ) -> bool {
        true // Simplificado
    }

pub(crate) fn is_same_type(&self, other: &Self) -> bool {
        if let (Some(n1), Some(n2)) = (
            self.dom.get_node(self.index),
            other.dom.get_node(other.index),
        ) {
            if let (AceNodeType::Element(e1), AceNodeType::Element(e2)) =
                (&n1.node_type, &n2.node_type)
            {
                return e1.tag == e2.tag;
            }
        }
        false
    }

pub(crate) fn attr_matches(
        &self,
        _ns: &NamespaceConstraint<&<Self::Impl as selectors::parser::SelectorImpl>::NamespaceUrl>,
        _local_name: &<Self::Impl as selectors::parser::SelectorImpl>::LocalName,
        operation: &AttrSelectorOperation<
            &<Self::Impl as selectors::parser::SelectorImpl>::AttrValue,
        >,
    ) -> bool {
        if let Some(node) = self.dom.get_node(self.index) {
            if let AceNodeType::Element(el) = &node.node_type {
                if let Some(val) = el.attributes.get(_local_name.as_str()) {
                    return match operation {
                        AttrSelectorOperation::Exists => true,
                        AttrSelectorOperation::WithValue {
                            operator, value, ..
                        } => match operator {
                            selectors::attr::AttrSelectorOperator::Equal => val == value.as_str(),
                            selectors::attr::AttrSelectorOperator::Includes => {
                                val.split_whitespace().any(|v| v == value.as_str())
                            }
                            selectors::attr::AttrSelectorOperator::DashMatch => {
                                val == value.as_str()
                                    || val.starts_with(&format!("{}-", value.as_str()))
                            }
                            selectors::attr::AttrSelectorOperator::Prefix => {
                                val.starts_with(value.as_str())
                            }
                            selectors::attr::AttrSelectorOperator::Suffix => {
                                val.ends_with(value.as_str())
                            }
                            selectors::attr::AttrSelectorOperator::Substring => {
                                val.contains(value.as_str())
                            }
                        },
                    };
                }
            }
        }
        false
    }

pub(crate) fn match_non_ts_pseudo_class(
        &self,
        pc: &<Self::Impl as selectors::parser::SelectorImpl>::NonTSPseudoClass,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        // FASE 5: Implement pseudo-class matching
        match pc {
            AceNonTSPseudoClass::Hover => {
                if let Some(hovered_idx) = self.hovered_element {
                    if hovered_idx == self.index {
                        return true;
                    }
                    // Bubbling: check if current element is an ancestor of the hovered element
                    let mut curr = self.dom.get_node(hovered_idx).and_then(|n| n.parent);
                    while let Some(idx) = curr {
                        if idx == self.index {
                            return true;
                        }
                        curr = self.dom.get_node(idx).and_then(|n| n.parent);
                    }
                }
                false
            }
            AceNonTSPseudoClass::Focus => {
                if let Some(focused) = self.focused_element {
                    focused == self.index
                } else {
                    false
                }
            }
            AceNonTSPseudoClass::Active => {
                if let Some(active_idx) = self.active_element {
                    if active_idx == self.index {
                        return true;
                    }
                    // Bubbling (opcional para :active, mas geralmente não propaga como hover,
                    // porém em alguns browsers sim. Vamos manter estrito por enquanto ou bubbling se desejar)
                    // Padrão web: :active geralmente aplica ao elemento sendo ativado e seus ancestrais.
                    let mut curr = self.dom.get_node(active_idx).and_then(|n| n.parent);
                    while let Some(idx) = curr {
                        if idx == self.index {
                            return true;
                        }
                        curr = self.dom.get_node(idx).and_then(|n| n.parent);
                    }
                }
                false
            }
            AceNonTSPseudoClass::FirstChild => self.prev_sibling_element().is_none(),
            AceNonTSPseudoClass::LastChild => self.next_sibling_element().is_none(),
            AceNonTSPseudoClass::NthChild(a, b) => {
                let mut count = 0;
                let mut current = self.prev_sibling_element();
                while current.is_some() {
                    count += 1;
                    current = current.expect("Current element expected").prev_sibling_element();
                }

                let index = count + 1;
                if *a == 0 {
                    index == *b
                } else {
                    let diff = index - *b;
                    if *a > 0 {
                        diff >= 0 && diff % *a == 0
                    } else {
                        diff <= 0 && diff % *a == 0
                    }
                }
            }
            AceNonTSPseudoClass::FirstOfType => {
                // Find previous sibling with same tag name
                if let Some(node) = self.dom.get_node(self.index) {
                    if let AceNodeType::Element(el) = &node.node_type {
                        let tag = &el.tag;
                        let mut prev = self.prev_sibling_element();
                        while let Some(sibling) = prev {
                            if let Some(sibling_node) = self.dom.get_node(sibling.index) {
                                if let AceNodeType::Element(sibling_el) = &sibling_node.node_type {
                                    if sibling_el.tag == *tag {
                                        return false;
                                    }
                                }
                            }
                            prev = sibling.prev_sibling_element();
                        }
                        return true;
                    }
                }
                false
            }
            AceNonTSPseudoClass::LastOfType => {
                if let Some(node) = self.dom.get_node(self.index) {
                    if let AceNodeType::Element(el) = &node.node_type {
                        let tag = &el.tag;
                        let mut next = self.next_sibling_element();
                        while let Some(sibling) = next {
                            if let Some(sibling_node) = self.dom.get_node(sibling.index) {
                                if let AceNodeType::Element(sibling_el) = &sibling_node.node_type {
                                    if sibling_el.tag == *tag {
                                        return false;
                                    }
                                }
                            }
                            next = sibling.next_sibling_element();
                        }
                        return true;
                    }
                }
                false
            }
            AceNonTSPseudoClass::OnlyChild => {
                self.prev_sibling_element().is_none() && self.next_sibling_element().is_none()
            }
            AceNonTSPseudoClass::Link => self.has_local_name(&AceIdent("a".to_string())),
            AceNonTSPseudoClass::Visited => {
                // Visited requires history API integration - treat as link for now
                false
            }
            AceNonTSPseudoClass::Empty => self.is_empty(),
        }
    }
}
