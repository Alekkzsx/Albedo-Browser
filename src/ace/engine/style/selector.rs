// Ace Selector Implementation
// Extraído de style/mod.rs para melhor organização

use crate::ace::engine::dom::{AceDOM, AceNodeType};
use cssparser::ToCss;
use precomputed_hash::PrecomputedHash;
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching::{ElementSelectorFlags, MatchingContext, MatchingMode};
use selectors::parser;
use selectors::OpaqueElement;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct AceIdent(pub String);

impl AceIdent {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl ToCss for AceIdent {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(&self.0)
    }
}

impl From<String> for AceIdent {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl<'a> From<&'a str> for AceIdent {
    fn from(s: &'a str) -> Self {
        Self(s.to_string())
    }
}

impl PrecomputedHash for AceIdent {
    fn precomputed_hash(&self) -> u32 {
        0 // Simplificado para ACE Engine
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AceSelectorImpl;

impl parser::SelectorImpl for AceSelectorImpl {
    type ExtraMatchingData<'a> = ();
    type AttrValue = AceIdent;
    type Identifier = AceIdent;
    type LocalName = AceIdent;
    type NamespaceUrl = AceIdent;
    type NamespacePrefix = AceIdent;
    type BorrowedNamespaceUrl = AceIdent;
    type BorrowedLocalName = AceIdent;

    type PseudoElement = AcePseudoElement;
    type NonTSPseudoClass = AceNonTSPseudoClass;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcePseudoElement {
    Before,
    After,
    Placeholder,
    Selection,
    Marker,
}

impl AcePseudoElement {
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

impl parser::PseudoElement for AcePseudoElement {
    type Impl = AceSelectorImpl;
}

impl ToCss for AcePseudoElement {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        write!(dest, "::{}", self.name())
    }
}

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

impl parser::NonTSPseudoClass for AceNonTSPseudoClass {
    type Impl = AceSelectorImpl;
    
    fn is_active_or_hover(&self) -> bool {
        matches!(
            self,
            AceNonTSPseudoClass::Hover | AceNonTSPseudoClass::Active
        )
    }
    
    fn is_user_action_state(&self) -> bool {
        matches!(
            self,
            AceNonTSPseudoClass::Hover | AceNonTSPseudoClass::Focus | AceNonTSPseudoClass::Active
        )
    }
}

impl ToCss for AceNonTSPseudoClass {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(self.name())
    }
}

#[derive(Clone, Debug)]
pub struct AceElement<'a> {
    pub dom: &'a AceDOM,
    pub index: usize,
    pub hovered_element: Option<usize>,
    pub focused_element: Option<usize>,
    pub active_element: Option<usize>,
}

impl<'a> AceElement<'a> {
    pub fn new(
        dom: &'a AceDOM,
        index: usize,
        hovered_element: Option<usize>,
        focused_element: Option<usize>,
        active_element: Option<usize>,
    ) -> Self {
        Self {
            dom,
            index,
            hovered_element,
            focused_element,
            active_element,
        }
    }

    fn get_node(&self) -> Option<&crate::ace::engine::dom::AceNode> {
        self.dom.get_node(self.index)
    }

    fn is_element_node(node: &crate::ace::engine::dom::AceNode) -> bool {
        matches!(&node.node_type, AceNodeType::Element(_))
    }
}

impl<'a> selectors::Element for AceElement<'a> {
    type Impl = AceSelectorImpl;

    fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(self.dom.nodes.get(self.index).unwrap())
    }

    fn parent_element(&self) -> Option<Self> {
        if let Some(node) = self.get_node() {
            if let Some(parent_idx) = node.parent {
                if let Some(parent_node) = self.dom.get_node(parent_idx) {
                    if Self::is_element_node(parent_node) {
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

    fn first_element_child(&self) -> Option<Self> {
        if let Some(node) = self.get_node() {
            for &child_idx in &node.children {
                if let Some(child_node) = self.dom.get_node(child_idx) {
                    if Self::is_element_node(child_node) {
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

    fn prev_sibling_element(&self) -> Option<Self> {
        if let Some(node) = self.get_node() {
            let mut curr_prev = node.prev_sibling;
            while let Some(prev_idx) = curr_prev {
                if let Some(prev_node) = self.dom.get_node(prev_idx) {
                    if Self::is_element_node(prev_node) {
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

    fn next_sibling_element(&self) -> Option<Self> {
        if let Some(node) = self.get_node() {
            let mut curr_next = node.next_sibling;
            while let Some(next_idx) = curr_next {
                if let Some(next_node) = self.dom.get_node(next_idx) {
                    if Self::is_element_node(next_node) {
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

    fn is_empty(&self) -> bool {
        if let Some(node) = self.get_node() {
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

    fn is_root(&self) -> bool {
        if let Some(node) = self.get_node() {
            return node.parent.is_none();
        }
        false
    }

    fn is_html_element_in_html_document(&self) -> bool {
        true
    }

    fn has_local_name(&self, name: &<Self::Impl as parser::SelectorImpl>::BorrowedLocalName) -> bool {
        if let Some(node) = self.get_node() {
            if let AceNodeType::Element(el) = &node.node_type {
                return el.tag == name.0;
            }
        }
        false
    }

    fn has_namespace(
        &self,
        _ns: &NamespaceConstraint<&<Self::Impl as parser::SelectorImpl>::NamespaceUrl>,
    ) -> bool {
        true // Simplificado
    }

    fn is_same_type(&self, other: &Self) -> bool {
        if let (Some(n1), Some(n2)) = (self.get_node(), other.get_node()) {
            if let (AceNodeType::Element(e1), AceNodeType::Element(e2)) =
                (&n1.node_type, &n2.node_type)
            {
                return e1.tag == e2.tag;
            }
        }
        false
    }

    fn attr_matches(
        &self,
        _ns: &NamespaceConstraint<&<Self::Impl as parser::SelectorImpl>::NamespaceUrl>,
        _local_name: &<Self::Impl as parser::SelectorImpl>::LocalName,
        operation: &AttrSelectorOperation<&<Self::Impl as parser::SelectorImpl>::AttrValue>,
    ) -> bool {
        if let Some(node) = self.get_node() {
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

    fn match_non_ts_pseudo_class(
        &self,
        pc: &<Self::Impl as parser::SelectorImpl>::NonTSPseudoClass,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
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
                    // Bubbling para :active
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
                    current = current.unwrap().prev_sibling_element();
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
                let mut current = self.prev_sibling_element();
                while let Some(sibling) = current {
                    if sibling.is_same_type(self) {
                        return false;
                    }
                    current = sibling.prev_sibling_element();
                }
                true
            }
            AceNonTSPseudoClass::LastOfType => {
                let mut current = self.next_sibling_element();
                while let Some(sibling) = current {
                    if sibling.is_same_type(self) {
                        return false;
                    }
                    current = sibling.next_sibling_element();
                }
                true
            }
            AceNonTSPseudoClass::OnlyChild => {
                self.prev_sibling_element().is_none() && self.next_sibling_element().is_none()
            }
            AceNonTSPseudoClass::Link => {
                if let Some(node) = self.get_node() {
                    if let AceNodeType::Element(el) = &node.node_type {
                        return el.tag == "a" && el.attributes.contains_key("href");
                    }
                }
                false
            }
            AceNonTSPseudoClass::Visited => false,
            AceNonTSPseudoClass::Empty => self.is_empty(),
        }
    }

    fn is_link(&self) -> bool {
        if let Some(node) = self.get_node() {
            if let AceNodeType::Element(el) = &node.node_type {
                return el.tag == "a" && el.attributes.contains_key("href");
            }
        }
        false
    }

    fn has_id(
        &self,
        name: &<Self::Impl as parser::SelectorImpl>::Identifier,
    ) -> bool {
        if let Some(node) = self.get_node() {
            if let AceNodeType::Element(el) = &node.node_type {
                if let Some(id) = el.attributes.get("id") {
                    return id == name.as_str();
                }
            }
        }
        false
    }

    fn has_class(
        &self,
        name: &<Self::Impl as parser::SelectorImpl>::ClassName,
        _case_sensitivity: CaseSensitivity,
    ) -> bool {
        if let Some(node) = self.get_node() {
            if let AceNodeType::Element(el) = &node.node_type {
                if let Some(class_attr) = el.attributes.get("class") {
                    return class_attr
                        .split_whitespace()
                        .any(|class| class == name.0.as_str());
                }
            }
        }
        false
    }

    fn is_disabled(&self) -> bool {
        if let Some(node) = self.get_node() {
            if let AceNodeType::Element(el) = &node.node_type {
                return el.attributes.contains_key("disabled");
            }
        }
        false
    }

    fn is_enabled(&self) -> bool {
        !self.is_disabled()
    }

    fn is_checked(&self) -> bool {
        if let Some(node) = self.get_node() {
            if let AceNodeType::Element(el) = &node.node_type {
                return el.attributes.contains_key("checked");
            }
        }
        false
    }

    fn is_indeterminate(&self) -> bool {
        if let Some(node) = self.get_node() {
            if let AceNodeType::Element(el) = &node.node_type {
                return el.attributes.contains_key("indeterminate");
            }
        }
        false
    }

    fn has_custom_state(&self, _name: &AceIdent) -> bool {
        false
    }

    fn imported_part(
        &self,
        _name: &<Self::Impl as parser::SelectorImpl>::Identifier,
    ) -> Option<<Self::Impl as parser::SelectorImpl>::Identifier> {
        None
    }

    fn exported_part(
        &self,
        _name: &<Self::Impl as parser::SelectorImpl>::Identifier,
    ) -> Option<<Self::Impl as parser::SelectorImpl>::Identifier> {
        None
    }

    fn is_part(&self, _name: &<Self::Impl as parser::SelectorImpl>::Identifier) -> bool {
        false
    }

    fn is_slotted(&self, _slotted: &<Self::Impl as parser::SelectorImpl>::Identifier) -> bool {
        false
    }

    fn shadow_root(&self) -> Option<selectors::element::ShadowRoot<&Self>> {
        None
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_content_slot_light(&self) -> bool {
        false
    }

    fn slot_name(&self) -> Option<<Self::Impl as parser::SelectorImpl>::Identifier> {
        None
    }

    fn assigned_slot(&self) -> Option<selectors::element::SlotInfo<&Self>> {
        None
    }
}
