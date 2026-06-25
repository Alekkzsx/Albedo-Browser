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





impl<'a> selectors::Element for AceElement<'a> {
pub(crate) type Impl = AceSelectorImpl;

pub(crate) fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(self.dom.nodes.get(self.index).expect("Node not found"))
    }

pub(crate) fn parent_element(&self) -> Option<Self> {
        if let Some(node) = self.dom.get_node(self.index) {
            if let Some(parent_idx) = node.parent {
                if let Some(parent_node) = self.dom.get_node(parent_idx) {
                    if let AceNodeType::Element(_) = &parent_node.node_type {
                        return Some(AceElement {
                            dom: self.dom,
                            index: parent_idx,
                            hovered_element: self.hovered_element,
                            focused_element: self.focused_element,
                            active_element: self.active_element,
                        });
                    }
                }
            }
        }
        None
    }

pub(crate) fn first_element_child(&self) -> Option<Self> {
        if let Some(node) = self.dom.get_node(self.index) {
            for &child_idx in &node.children {
                if let Some(child_node) = self.dom.get_node(child_idx) {
                    if let AceNodeType::Element(_) = &child_node.node_type {
                        return Some(AceElement {
                            dom: self.dom,
                            index: child_idx,
                            hovered_element: self.hovered_element,
                            focused_element: self.focused_element,
                            active_element: self.active_element,
                        });
                    }
                }
            }
        }
        None
    }

pub(crate) fn prev_sibling_element(&self) -> Option<Self> {
        if let Some(node) = self.dom.get_node(self.index) {
            let mut curr_prev = node.prev_sibling;
            while let Some(prev_idx) = curr_prev {
                if let Some(prev_node) = self.dom.get_node(prev_idx) {
                    if let AceNodeType::Element(_) = &prev_node.node_type {
                        return Some(AceElement {
                            dom: self.dom,
                            index: prev_idx,
                            hovered_element: self.hovered_element,
                            focused_element: self.focused_element,
                            active_element: self.active_element,
                        });
                    }
                    curr_prev = prev_node.prev_sibling;
                } else {
                    break;
                }
            }
        }
        None
    }

pub(crate) fn next_sibling_element(&self) -> Option<Self> {
        if let Some(node) = self.dom.get_node(self.index) {
            let mut curr_next = node.next_sibling;
            while let Some(next_idx) = curr_next {
                if let Some(next_node) = self.dom.get_node(next_idx) {
                    if let AceNodeType::Element(_) = &next_node.node_type {
                        return Some(AceElement {
                            dom: self.dom,
                            index: next_idx,
                            hovered_element: self.hovered_element,
                            focused_element: self.focused_element,
                            active_element: self.active_element,
                        });
                    }
                    curr_next = next_node.next_sibling;
                } else {
                    break;
                }
            }
        }
        None
    }

pub(crate) fn is_empty(&self) -> bool {
        if let Some(node) = self.dom.get_node(self.index) {
            for &child_idx in &node.children {
                if let Some(child) = self.dom.get_node(child_idx) {
                    match &child.node_type {
                        AceNodeType::Element(_) => return false,
                        AceNodeType::Text(t) if !t.as_ref().trim().is_empty() => return false,
                        _ => {}
                    }
                }
            }
        }
        true
    }
}
