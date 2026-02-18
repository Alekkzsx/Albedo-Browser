use cssparser::{Parser, ParserInput, ToCss, SourceLocation, CowRcStr, ParseError};
use std::collections::HashMap;
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching::{ElementSelectorFlags, MatchingContext, MatchingMode};
use selectors::OpaqueElement;
use precomputed_hash::PrecomputedHash;
use crate::engine::dom::{AceDOM, AceNodeType};
pub mod css_values;
use self::css_values::{
    ComputedStyle, CssLength, CssColor, CssDisplay, CssTextAlign, CssFontWeight,
    CssPosition, CssOverflow, CssFloat, CssFlexDirection, CssJustifyContent,
    CssAlignItems, CssAlignContent, CssFlexWrap, BoxShadow, TextShadow, BackgroundImage, Gradient, GradientStop,
    CssContent, CssBoxSizing, CssVisibility, CssCursor, CssObjectFit,
    CssObjectPosition, CssPointerEvents, CssBlendMode
};
use crate::engine::CssFilter;
use crate::engine::TransformFunction;

pub mod animation;
#[cfg(test)]
mod grid_tests;
#[path = "../dom_tests.rs"]
mod dom_tests;

pub struct Stylesheet {
    pub user_agent_rules: Vec<AceRule>,
    pub rules: Vec<AceRule>,
    pub media_rules: Vec<AceMediaRule>,
    pub supports_rules: Vec<AceSupportsRule>,
    pub container_rules: Vec<AceContainerRule>,
    pub font_faces: Vec<std::collections::HashMap<String, String>>,
    pub keyframes: HashMap<String, Vec<self::css_values::CssKeyframe>>,
    pub user_agent_rule_map: RuleMap,
    pub author_rule_map: RuleMap,
}

#[derive(Debug, Clone)]
pub struct AceMediaRule {
    pub media_query: String,
    pub rules: Vec<AceRule>,
}

#[derive(Debug, Clone)]
pub struct AceSupportsRule {
    pub condition: String,
    pub rules: Vec<AceRule>,
}

#[derive(Debug, Clone)]
pub struct AceContainerRule {
    pub name: Option<String>,
    pub condition: String,
    pub rules: Vec<AceRule>,
}

#[derive(Debug, Clone)]
pub struct AceRule {
    pub selectors: selectors::SelectorList<AceSelectorImpl>,
    pub declarations: Vec<Declaration>,
}

#[derive(Default, Clone)]
pub struct RuleMap {
    pub id_rules: HashMap<String, Vec<AceRule>>,
    pub class_rules: HashMap<String, Vec<AceRule>>,
    pub tag_rules: HashMap<String, Vec<AceRule>>,
    pub universal_rules: Vec<AceRule>,
}

impl RuleMap {
    pub fn add_rule(&mut self, rule: AceRule) {
        for selector in rule.selectors.slice().iter() {
            let mut indexed = false;
            // Iterate in matching order (right-to-left) to find the most specific clue
            for component in selector.iter_raw_match_order() {
                match component {
                    selectors::parser::Component::ID(id) => {
                        self.id_rules.entry(id.0.clone()).or_default().push(rule.clone());
                        indexed = true;
                        break;
                    }
                    selectors::parser::Component::Class(class) => {
                        self.class_rules.entry(class.0.clone()).or_default().push(rule.clone());
                        indexed = true;
                        break;
                    }
                    selectors::parser::Component::LocalName(name) => {
                        self.tag_rules.entry(name.name.0.clone()).or_default().push(rule.clone());
                        indexed = true;
                        break;
                    }
                    _ => {}
                }
            }
            if !indexed {
                self.universal_rules.push(rule.clone());
            }
        }
    }

    pub fn match_element<'a>(&'a self, element: &AceElement, matched_rules: &mut Vec<MatchedRule<'a>>, origin: CascadeOrigin, base_order: usize) {
        let mut try_match = |rule: &'a AceRule, order: usize| {
            for selector in rule.selectors.slice() {
                let mut caches = selectors::matching::SelectorCaches::default();
                let mut context = MatchingContext::new(
                    MatchingMode::Normal, None, &mut caches,
                    selectors::matching::QuirksMode::NoQuirks,
                    selectors::matching::NeedsSelectorFlags::No,
                    selectors::matching::MatchingForInvalidation::No,
                );
                if selectors::matching::matches_selector(selector, 0, None, element, &mut context) {
                    matched_rules.push(MatchedRule {
                        priority: CascadePriority { origin: origin.clone(), important: false, specificity: selector.specificity(), order: base_order + order },
                        rule
                    });
                }
            }
        };

        // 1. Check Universal rules
        for (i, rule) in self.universal_rules.iter().enumerate() {
            try_match(rule, i);
        }

        if let AceNodeType::Element(el) = &element.dom.get_node(element.index).unwrap().node_type {
            // 2. Check Tag rules
            if let Some(rules) = self.tag_rules.get(el.tag_name()) {
                for (i, rule) in rules.iter().enumerate() { try_match(rule, i); }
            }
            // 3. Check ID rules
            if let Some(id) = el.attributes.get("id") {
                if let Some(rules) = self.id_rules.get(id) {
                    for (i, rule) in rules.iter().enumerate() { try_match(rule, i); }
                }
            }
            // 4. Check Class rules
            if let Some(class_attr) = el.attributes.get("class") {
                for class in class_attr.split_whitespace() {
                    if let Some(rules) = self.class_rules.get(class) {
                        for (i, rule) in rules.iter().enumerate() { try_match(rule, i); }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub name: String,
    pub value: String,
    pub important: bool,
}

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
    fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result where W: std::fmt::Write {
        dest.write_str(&self.0)
    }
}

impl From<String> for AceIdent {
    fn from(s: String) -> Self { Self(s) }
}

impl<'a> From<&'a str> for AceIdent {
    fn from(s: &'a str) -> Self { Self(s.to_string()) }
}

impl PrecomputedHash for AceIdent {
    fn precomputed_hash(&self) -> u32 {
        0 // Simplificado para ACE Engine
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AceSelectorImpl;

impl selectors::parser::SelectorImpl for AceSelectorImpl {
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

impl selectors::parser::PseudoElement for AcePseudoElement {
    type Impl = AceSelectorImpl;
}

impl ToCss for AcePseudoElement {
    fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result where W: std::fmt::Write {
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

impl selectors::parser::NonTSPseudoClass for AceNonTSPseudoClass {
    type Impl = AceSelectorImpl;
    fn is_active_or_hover(&self) -> bool {
        matches!(self, AceNonTSPseudoClass::Hover | AceNonTSPseudoClass::Active)
    }
    fn is_user_action_state(&self) -> bool {
        matches!(self, AceNonTSPseudoClass::Hover | AceNonTSPseudoClass::Focus | AceNonTSPseudoClass::Active)
    }
}
impl ToCss for AceNonTSPseudoClass {
    fn to_css<W>(&self, dest: &mut W) -> std::fmt::Result where W: std::fmt::Write {
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

impl<'a> selectors::Element for AceElement<'a> {
    type Impl = AceSelectorImpl;

    fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(self.dom.nodes.get(self.index).unwrap())
    }

    fn parent_element(&self) -> Option<Self> {
        if let Some(node) = self.dom.get_node(self.index) {
            if let Some(parent_idx) = node.parent {
                if let Some(parent_node) = self.dom.get_node(parent_idx) {
                    if let AceNodeType::Element(_) = &parent_node.node_type {
                         return Some(AceElement { dom: self.dom, index: parent_idx, hovered_element: self.hovered_element, focused_element: self.focused_element, active_element: self.active_element });
                    }
                }
            }
        }
        None
    }

    fn first_element_child(&self) -> Option<Self> {
        if let Some(node) = self.dom.get_node(self.index) {
            for &child_idx in &node.children {
                if let Some(child_node) = self.dom.get_node(child_idx) {
                    if let AceNodeType::Element(_) = &child_node.node_type {
                        return Some(AceElement { dom: self.dom, index: child_idx, hovered_element: self.hovered_element, focused_element: self.focused_element, active_element: self.active_element });
                    }
                }
            }
        }
        None
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        if let Some(node) = self.dom.get_node(self.index) {
            let mut curr_prev = node.prev_sibling;
            while let Some(prev_idx) = curr_prev {
                if let Some(prev_node) = self.dom.get_node(prev_idx) {
                    if let AceNodeType::Element(_) = &prev_node.node_type {
                        return Some(AceElement { dom: self.dom, index: prev_idx, hovered_element: self.hovered_element, focused_element: self.focused_element, active_element: self.active_element });
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
        if let Some(node) = self.dom.get_node(self.index) {
            let mut curr_next = node.next_sibling;
            while let Some(next_idx) = curr_next {
                if let Some(next_node) = self.dom.get_node(next_idx) {
                    if let AceNodeType::Element(_) = &next_node.node_type {
                        return Some(AceElement { dom: self.dom, index: next_idx, hovered_element: self.hovered_element, focused_element: self.focused_element, active_element: self.active_element });
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
        if let Some(node) = self.dom.get_node(self.index) {
            for &child_idx in &node.children {
                if let Some(child) = self.dom.get_node(child_idx) {
                    match &child.node_type {
                        AceNodeType::Element(_) => return false,
                        AceNodeType::Text(t) if !t.trim().is_empty() => return false,
                        _ => {}
                    }
                }
            }
        }
        true
    }

    fn is_root(&self) -> bool {
        if let Some(node) = self.dom.get_node(self.index) {
            return node.parent.is_none();
        }
        false
    }

    fn is_html_element_in_html_document(&self) -> bool { true }

    fn has_local_name(&self, name: &<Self::Impl as selectors::parser::SelectorImpl>::BorrowedLocalName) -> bool {
        if let Some(node) = self.dom.get_node(self.index) {
            if let AceNodeType::Element(el) = &node.node_type {
                return el.tag == name.0;
            }
        }
        false
    }

    fn has_namespace(&self, _ns: &<Self::Impl as selectors::parser::SelectorImpl>::BorrowedNamespaceUrl) -> bool {
        true // Simplificado
    }

    fn is_same_type(&self, other: &Self) -> bool {
        if let (Some(n1), Some(n2)) = (self.dom.get_node(self.index), other.dom.get_node(other.index)) {
             if let (AceNodeType::Element(e1), AceNodeType::Element(e2)) = (&n1.node_type, &n2.node_type) {
                 return e1.tag == e2.tag;
             }
        }
        false
    }

    fn attr_matches(
        &self,
        _ns: &NamespaceConstraint<&<Self::Impl as selectors::parser::SelectorImpl>::NamespaceUrl>,
        _local_name: &<Self::Impl as selectors::parser::SelectorImpl>::LocalName,
        operation: &AttrSelectorOperation<&<Self::Impl as selectors::parser::SelectorImpl>::AttrValue>,
    ) -> bool {
        if let Some(node) = self.dom.get_node(self.index) {
            if let AceNodeType::Element(el) = &node.node_type {
                if let Some(val) = el.attributes.get(_local_name.as_str()) {
                    return match operation {
                        AttrSelectorOperation::Exists => true,
                        AttrSelectorOperation::WithValue { operator, value, .. } => {
                            match operator {
                                selectors::attr::AttrSelectorOperator::Equal => val == value.as_str(),
                                selectors::attr::AttrSelectorOperator::Includes => val.split_whitespace().any(|v| v == value.as_str()),
                                selectors::attr::AttrSelectorOperator::DashMatch => val == value.as_str() || val.starts_with(&format!("{}-", value.as_str())),
                                selectors::attr::AttrSelectorOperator::Prefix => val.starts_with(value.as_str()),
                                selectors::attr::AttrSelectorOperator::Suffix => val.ends_with(value.as_str()),
                                selectors::attr::AttrSelectorOperator::Substring => val.contains(value.as_str()),
                            }
                        }
                    };
                }
            }
        }
        false
    }

    fn match_non_ts_pseudo_class(
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
            },
            AceNonTSPseudoClass::Focus => {
                if let Some(focused) = self.focused_element {
                    focused == self.index
                } else {
                    false
                }
            },
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
            },
            AceNonTSPseudoClass::FirstChild => {
                self.prev_sibling_element().is_none()
            },
            AceNonTSPseudoClass::LastChild => {
                self.next_sibling_element().is_none()
            },
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
            },
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
            },
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
            },
            AceNonTSPseudoClass::OnlyChild => {
                self.prev_sibling_element().is_none() && self.next_sibling_element().is_none()
            },
            AceNonTSPseudoClass::Link => {
                self.has_local_name(&AceIdent("a".to_string()))
            },
            AceNonTSPseudoClass::Visited => {
                // Visited requires history API integration - treat as link for now
                false
            },
            AceNonTSPseudoClass::Empty => {
                self.is_empty()
            },
        }
    }

    fn match_pseudo_element(
        &self,
        _pe: &<Self::Impl as selectors::parser::SelectorImpl>::PseudoElement,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

    fn apply_selector_flags(&self, _flags: ElementSelectorFlags) {}

    fn is_link(&self) -> bool {
        self.has_local_name(&AceIdent("a".to_string()))
    }

    fn has_id(&self, id: &<Self::Impl as selectors::parser::SelectorImpl>::Identifier, case_sensitivity: CaseSensitivity) -> bool {
        if let Some(node) = self.dom.get_node(self.index) {
            if let AceNodeType::Element(el) = &node.node_type {
                if let Some(val) = el.attributes.get("id") {
                    return case_sensitivity.eq(val.as_bytes(), id.as_bytes());
                }
            }
        }
        false
    }

    fn has_class(&self, name: &<Self::Impl as selectors::parser::SelectorImpl>::Identifier, case_sensitivity: CaseSensitivity) -> bool {
        if let Some(node) = self.dom.get_node(self.index) {
            if let AceNodeType::Element(el) = &node.node_type {
                 if let Some(val) = el.attributes.get("class") {
                    return val.split_whitespace().any(|c| case_sensitivity.eq(c.as_bytes(), name.as_bytes()));
                }
            }
        }
        false
    }

    fn is_part(&self, _name: &<Self::Impl as selectors::parser::SelectorImpl>::Identifier) -> bool { false }
    fn imported_part(&self, _name: &<Self::Impl as selectors::parser::SelectorImpl>::Identifier) -> Option<<Self::Impl as selectors::parser::SelectorImpl>::Identifier> { None }

    fn parent_node_is_shadow_root(&self) -> bool { false }
    fn containing_shadow_host(&self) -> Option<Self> { None }
    fn is_pseudo_element(&self) -> bool { false }
    fn is_html_slot_element(&self) -> bool { false }
    fn has_custom_state(&self, _name: &AceIdent) -> bool { false }
    fn add_element_unique_hashes(&self, _filter: &mut selectors::bloom::BloomFilter) -> bool { false }
}

pub fn get_user_agent_stylesheet() -> Stylesheet {
    let ua_css = "
        header, footer, main, section, article, nav, aside, 
        details, summary, figure, figcaption, h1, h2, h3, h4, 
        h5, h6, p, ul, ol, li, div, blockquote, address { display: block; }
        
        template { display: none; }
        
        h1 { font-size: 2em; font-weight: bold; margin: 0.67em 0; }
        h2 { font-size: 1.5em; font-weight: bold; margin: 0.83em 0; }
        h3 { font-size: 1.17em; font-weight: bold; margin: 1em 0; }
        h4 { font-size: 1em; font-weight: bold; margin: 1.33em 0; }
        h5 { font-size: 0.83em; font-weight: bold; margin: 1.67em 0; }
        h6 { font-size: 0.67em; font-weight: bold; margin: 2.33em 0; }
        
        b, strong { font-weight: bold; }
        i, em { font-style: italic; }
        
        ul { list-style-type: disc; margin: 1em 0; padding-left: 40px; }
        ol { list-style-type: decimal; margin: 1em 0; padding-left: 40px; }
        
        button, input, select, textarea { 
            display: inline-block; 
            margin: 0; 
            font: inherit; 
            box-sizing: border-box;
        }
        button { background-color: #efefef; border: 1px solid #767676; padding: 1px 6px; }
        input[type=\"text\"], input[type=\"password\"], input[type=\"email\"], input[type=\"number\"] { 
            background-color: white; 
            border: 1px solid #767676; 
            padding: 1px 2px; 
            min-height: 1.2em;
        }
        textarea {
            background-color: white; 
            border: 1px solid #767676; 
            padding: 2px;
            min-height: 2em;
        }
        select {
            background-color: white; 
            border: 1px solid #767676; 
            padding: 1px 2px;
        }

        /* Table Default Styles */
        table { display: table; border-collapse: separate; border-spacing: 2px; border-color: gray; }
        thead { display: table-header-group; vertical-align: middle; border-color: inherit; }
        tbody { display: table-row-group; vertical-align: middle; border-color: inherit; }
        tfoot { display: table-footer-group; vertical-align: middle; border-color: inherit; }
        tr { display: table-row; vertical-align: inherit; border-color: inherit; }
        td, th { display: table-cell; vertical-align: inherit; }
        th { font-weight: bold; text-align: center; }
    ";
    
    let mut ss = parse_simple(ua_css);
    // Move rules to user_agent_rules
    let rules = std::mem::take(&mut ss.rules);
    ss.user_agent_rules = rules;
    ss.keyframes = HashMap::new();
    ss.supports_rules = Vec::new();
    ss.container_rules = Vec::new();
    ss.font_faces = Vec::new();
    
    // Build optimization maps
    ss.build_rule_maps();
    
    ss
}

pub fn parse(source: &str) -> Stylesheet {
    let mut stylesheet = get_user_agent_stylesheet();
    let mut remaining_source = source.to_string();
    
    // Process top-level @-rules
    while let Some(index) = remaining_source.find('@') {
        let before = &remaining_source[..index];
        if !before.trim().is_empty() {
            let before_stylesheet = parse_simple(before);
            stylesheet.rules.extend(before_stylesheet.rules);
        }
        
        let rest = &remaining_source[index..];
        if let Some(brace_start) = rest.find('{') {
            let mut brace_count = 1;
            let mut block_end = brace_start + 1;
            while block_end < rest.len() && brace_count > 0 {
                if rest[block_end..].starts_with('{') { brace_count += 1; }
                else if rest[block_end..].starts_with('}') { brace_count -= 1; }
                block_end += 1;
            }
            
            let block = &rest[..block_end];
            if rest.starts_with("@media") {
                if let Some(query_end) = block.find('{') {
                    let media_query = block[6..query_end].trim();
                    let media_content = &block[query_end + 1..block.len() - 1];
                    let media_stylesheet = parse_simple(media_content);
                    stylesheet.media_rules.push(AceMediaRule {
                        media_query: media_query.to_string(),
                        rules: media_stylesheet.rules,
                    });
                }
            } else if rest.starts_with("@supports") {
                if let Some(query_end) = block.find('{') {
                    let condition = block[9..query_end].trim();
                    let content = &block[query_end + 1..block.len() - 1];
                    let supports_stylesheet = parse_simple(content);
                    stylesheet.supports_rules.push(AceSupportsRule {
                        condition: condition.to_string(),
                        rules: supports_stylesheet.rules,
                    });
                }
            } else if rest.starts_with("@container") {
                if let Some(query_end) = block.find('{') {
                    let full_query = block[10..query_end].trim();
                    let (name, condition) = if let Some(n_end) = full_query.find(' ') {
                         (Some(full_query[..n_end].trim().to_string()), full_query[n_end..].trim().to_string())
                    } else {
                         (None, full_query.to_string())
                    };
                    let content = &block[query_end + 1..block.len() - 1];
                    let container_stylesheet = parse_simple(content);
                    stylesheet.container_rules.push(AceContainerRule {
                        name,
                        condition,
                        rules: container_stylesheet.rules,
                    });
                }
            } else if rest.starts_with("@font-face") {
                if let Some(query_end) = block.find('{') {
                    let content = &block[query_end + 1..block.len() - 1];
                    let mut props = HashMap::new();
                    let mut input = ParserInput::new(content);
                    let mut p = Parser::new(&mut input);
                    while !p.is_exhausted() {
                        if let Ok(name) = p.expect_ident() {
                            let name_str = name.to_string();
                            if p.expect_colon().is_ok() {
                                let mut val = String::new();
                                while let Ok(t) = p.next() {
                                    val.push_str(&t.to_css_string());
                                }
                                props.insert(name_str, val.trim_end_matches(';').trim().to_string());
                            }
                        } else { let _ = p.next(); }
                    }
                    stylesheet.font_faces.push(props);
                }
            } else if rest.starts_with("@keyframes") {
                if let Some(name_end) = block[10..].find('{') {
                    let name = block[10..10+name_end].trim().to_string();
                    let content = &block[10+name_end+1..block.len()-1];
                    let mut keyframes = Vec::new();
                    let mut input = ParserInput::new(content);
                    let mut p = Parser::new(&mut input);
                    while !p.is_exhausted() {
                        let pct_str = match p.next() {
                            Ok(cssparser::Token::Percentage { unit_value, .. }) => (unit_value * 100.0).to_string(),
                            Ok(cssparser::Token::Ident(s)) if *s == "from" => "0".to_string(),
                            Ok(cssparser::Token::Ident(s)) if *s == "to" => "100".to_string(),
                            _ => { let _ = p.next(); continue; }
                        };
                        if p.expect_curly_bracket_block().is_ok() {
                            let pct = pct_str.parse::<f32>().unwrap_or(0.0);
                            let decls: std::collections::HashMap<String, String> = p.parse_nested_block(|inner_p| {
                                let mut map = HashMap::new();
                                while !inner_p.is_exhausted() {
                                    if let Ok(name) = inner_p.expect_ident() {
                                        let name_str = name.to_string();
                                        if inner_p.expect_colon().is_ok() {
                                            let mut val = String::new();
                                            while let Ok(t) = inner_p.next() {
                                                val.push_str(&t.to_css_string());
                                            }
                                            map.insert(name_str, val.trim_end_matches(';').trim().to_string());
                                        }
                                    } else { let _ = inner_p.next(); }
                                }
                                Ok::<std::collections::HashMap<String, String>, cssparser::ParseError<'_, cssparser::BasicParseErrorKind<'_>>>(map)
                            }).unwrap_or_default();
                            keyframes.push(self::css_values::CssKeyframe { percentage: pct, declarations: decls });
                        }
                    }
                    stylesheet.keyframes.insert(name, keyframes);
                }
            }
            
            remaining_source = rest[block_end..].to_string();
        } else {
            // Probably a single @rule without block or error
            remaining_source = rest[1..].to_string();
        }
    }
    
    // Parse remaining regular rules
    if !remaining_source.trim().is_empty() {
        let remaining_stylesheet = parse_simple(&remaining_source);
        stylesheet.rules.extend(remaining_stylesheet.rules);
    }

    stylesheet
}

fn parse_simple(source: &str) -> Stylesheet {
    let mut stylesheet = Stylesheet { 
        user_agent_rules: Vec::new(), rules: Vec::new(), 
        media_rules: Vec::new(), supports_rules: Vec::new(), 
        container_rules: Vec::new(), 
        font_faces: Vec::new(),
        keyframes: HashMap::new(),
        user_agent_rule_map: RuleMap::default(),
        author_rule_map: RuleMap::default(),
    };
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);

    while !parser.is_exhausted() {
        let selectors = match selectors::SelectorList::<AceSelectorImpl>::parse(&AceSelectorParser, &mut parser, selectors::parser::ParseRelative::No) {
            Ok(s) => s,
            Err(_) => {
                let _ = parser.next();
                continue;
            }
        };
        
        if parser.expect_curly_bracket_block().is_ok() {
            let decls = parser.parse_nested_block(|p| {
                    let mut decls = Vec::new();
                    while !p.is_exhausted() {
                        if let Ok(name) = p.expect_ident() {
                            let name = name.to_string();
                            if p.expect_colon().is_ok() {
                                let mut value_raw = String::new();
                                while let Ok(token) = p.next() {
                                    value_raw.push_str(&token.to_css_string());
                                }
                                
                                // Clean up value and detect !important
                                let mut important = false;
                                let mut value = value_raw.trim_end_matches(';').trim().to_string();
                                if value.to_lowercase().ends_with("!important") {
                                    important = true;
                                    value = value[..value.len() - 10].trim().to_string();
                                }
                                
                                // Expand shorthands (Simple implementation)
                                // Only margin and padding for now
                                match name.as_str() {
                                    "margin" => {
                                        let parts: Vec<&str> = value.split_whitespace().collect();
                                        match parts.len() {
                                            1 => {
                                                for suffix in &["top", "right", "bottom", "left"] {
                                                    decls.push(Declaration { name: format!("margin-{}", suffix), value: parts[0].to_string(), important: important });
                                                }
                                            },
                                            2 => {
                                                decls.push(Declaration { name: "margin-top".to_string(), value: parts[0].to_string(), important: important });
                                                decls.push(Declaration { name: "margin-bottom".to_string(), value: parts[0].to_string(), important: important });
                                                decls.push(Declaration { name: "margin-right".to_string(), value: parts[1].to_string(), important: important });
                                                decls.push(Declaration { name: "margin-left".to_string(), value: parts[1].to_string(), important: important });
                                            },
                                            4 => {
                                                 decls.push(Declaration { name: "margin-top".to_string(), value: parts[0].to_string(), important: important });
                                                 decls.push(Declaration { name: "margin-right".to_string(), value: parts[1].to_string(), important: important });
                                                 decls.push(Declaration { name: "margin-bottom".to_string(), value: parts[2].to_string(), important: important });
                                                 decls.push(Declaration { name: "margin-left".to_string(), value: parts[3].to_string(), important: important });
                                            },
                                            _ => {} // Ignore invalid syntax
                                        }
                                    },
                                    "padding" => {
                                        let parts: Vec<&str> = value.split_whitespace().collect();
                                        match parts.len() {
                                            1 => {
                                                for suffix in &["top", "right", "bottom", "left"] {
                                                    decls.push(Declaration { name: format!("padding-{}", suffix), value: parts[0].to_string(), important: important });
                                                }
                                            },
                                            2 => {
                                                decls.push(Declaration { name: "padding-top".to_string(), value: parts[0].to_string(), important: important });
                                                decls.push(Declaration { name: "padding-bottom".to_string(), value: parts[0].to_string(), important: important });
                                                decls.push(Declaration { name: "padding-right".to_string(), value: parts[1].to_string(), important: important });
                                                decls.push(Declaration { name: "padding-left".to_string(), value: parts[1].to_string(), important: important });
                                            },
                                            4 => {
                                                decls.push(Declaration { name: "padding-top".to_string(), value: parts[0].to_string(), important: important });
                                                decls.push(Declaration { name: "padding-right".to_string(), value: parts[1].to_string(), important: important });
                                                decls.push(Declaration { name: "padding-bottom".to_string(), value: parts[2].to_string(), important: important });
                                                decls.push(Declaration { name: "padding-left".to_string(), value: parts[3].to_string(), important: important });
                                            },
                                            _ => {}
                                        }
                                    },
                                    "border" => {
                                        let parts: Vec<&str> = value.split_whitespace().collect();
                                        for part in parts {
                                            if part.ends_with("px") || part.ends_with("em") || part.ends_with("rem") || part == "0" || part == "thin" || part == "medium" || part == "thick" {
                                                for suffix in &["top", "right", "bottom", "left"] {
                                                    decls.push(Declaration { name: format!("border-{}-width", suffix), value: part.to_string(), important: important });
                                                }
                                            } else if part == "solid" || part == "dashed" || part == "dotted" || part == "double" || part == "none" {
                                                for suffix in &["top", "right", "bottom", "left"] {
                                                    decls.push(Declaration { name: format!("border-{}-style", suffix), value: part.to_string(), important: important });
                                                }
                                            } else {
                                                // Assume it's a color
                                                for suffix in &["top", "right", "bottom", "left"] {
                                                    decls.push(Declaration { name: format!("border-{}-color", suffix), value: part.to_string(), important: important });
                                                }
                                            }
                                        }
                                    },
                                    "background" => {
                                        let parts: Vec<&str> = value.split_whitespace().collect();
                                        for part in parts {
                                            if part.starts_with("url(") || part.starts_with("linear-gradient(") || part.starts_with("radial-gradient(") {
                                                decls.push(Declaration { name: "background-image".to_string(), value: part.to_string(), important: important });
                                            } else if part == "no-repeat" || part == "repeat" || part == "repeat-x" || part == "repeat-y" {
                                                decls.push(Declaration { name: "background-repeat".to_string(), value: part.to_string(), important: important });
                                            } else if part == "center" || part == "top" || part == "bottom" || part == "left" || part == "right" || part.ends_with("%") || part.ends_with("px") {
                                                decls.push(Declaration { name: "background-position".to_string(), value: part.to_string(), important: important });
                                            } else {
                                                // Assume color
                                                decls.push(Declaration { name: "background-color".to_string(), value: part.to_string(), important: important });
                                            }
                                        }
                                    },
                                    "font" => {
                                        let parts: Vec<&str> = value.split_whitespace().collect();
                                        for part in parts {
                                            if part == "italic" || part == "oblique" {
                                                decls.push(Declaration { name: "font-style".to_string(), value: part.to_string(), important: important });
                                            } else if part == "bold" || part == "bolder" || part == "lighter" || part.parse::<f32>().is_ok() {
                                                decls.push(Declaration { name: "font-weight".to_string(), value: part.to_string(), important: important });
                                            } else if part.ends_with("px") || part.ends_with("em") || part.ends_with("rem") || part.ends_with("%") || part.ends_with("pt") {
                                                decls.push(Declaration { name: "font-size".to_string(), value: part.to_string(), important: important });
                                            } else {
                                                decls.push(Declaration { name: "font-family".to_string(), value: part.to_string(), important: important });
                                            }
                                        }
                                    },
                                    _ => decls.push(Declaration { name, value, important: important }),
                                }
                            }
                        }
                        let _ = p.expect_semicolon();
                    }
                    Ok::<_, ParseError<'_, selectors::parser::SelectorParseErrorKind<'_>>>(decls)
                }).unwrap_or_default();
                stylesheet.rules.push(AceRule { selectors, declarations: decls });
        }
    }

    stylesheet.build_rule_maps();
    stylesheet
}

impl Stylesheet {
    pub fn new() -> Self {
        Self {
            user_agent_rules: Vec::new(),
            rules: Vec::new(),
            media_rules: Vec::new(),
            supports_rules: Vec::new(),
            container_rules: Vec::new(),
            font_faces: Vec::new(),
            keyframes: HashMap::new(),
            user_agent_rule_map: RuleMap::default(),
            author_rule_map: RuleMap::default(),
        }
    }
    
    pub fn build_rule_maps(&mut self) {
        let mut ua_map = RuleMap::default();
        for rule in &self.user_agent_rules {
            ua_map.add_rule(rule.clone());
        }
        self.user_agent_rule_map = ua_map;

        let mut author_map = RuleMap::default();
        for rule in &self.rules {
            author_map.add_rule(rule.clone());
        }
        self.author_rule_map = author_map;
    }
}

struct AceSelectorParser;

impl<'i> selectors::parser::Parser<'i> for AceSelectorParser {
    type Impl = AceSelectorImpl;
    type Error = selectors::parser::SelectorParseErrorKind<'i>;

    fn parse_non_ts_pseudo_class(&self, location: SourceLocation, name: CowRcStr<'i>) -> Result<AceNonTSPseudoClass, ParseError<'i, Self::Error>> {
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
                kind: cssparser::ParseErrorKind::Custom(selectors::parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement(name)),
                location,
            })
        }
    }

    fn parse_non_ts_functional_pseudo_class(&self, name: CowRcStr<'i>, parser: &mut Parser<'i, '_>, _is_negated: bool) -> Result<AceNonTSPseudoClass, ParseError<'i, Self::Error>> {
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
                        let a = if parts[0].is_empty() { 1 } 
                                else if parts[0] == "-" { -1 }
                                else { parts[0].parse().unwrap_or(1) };
                        let b = if parts.len() > 1 && !parts[1].is_empty() {
                            parts[1].parse().unwrap_or(0)
                        } else { 0 };
                        Ok(AceNonTSPseudoClass::NthChild(a, b))
                    } else {
                        Ok(AceNonTSPseudoClass::NthChild(0, 1))
                    }
                }
            }
            _ => Err(ParseError {
                kind: cssparser::ParseErrorKind::Custom(selectors::parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement(name)),
                location: parser.current_source_location(),
            })
        }
    }
    
    fn parse_pseudo_element(
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
                    selectors::parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement(name)
                ),
                location,
            })
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CascadeOrigin {
    UserAgent = 0,
    Author = 1,
    AuthorMedia = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CascadePriority {
    pub origin: CascadeOrigin,
    pub important: bool,
    pub specificity: u32,
    pub order: usize,
}

pub struct MatchedRule<'a> {
    pub priority: CascadePriority,
    pub rule: &'a AceRule,
}

impl Stylesheet {
    // Calculate style with inheritance
    pub fn calculate_style(&self, 
        dom: &AceDOM, 
        node_id: usize, 
        parent_style: Option<&ComputedStyle>, 
        root_style: Option<&ComputedStyle>, 
        hovered_element: Option<usize>, 
        focused_element: Option<usize>, 
        active_element: Option<usize>,
        animation_manager: Option<&self::animation::AnimationManager>,
        current_time: f64,
        vw: f32, 
        vh: f32, 
        color_scheme: &str) -> ComputedStyle {
        
        let node = dom.get_node(node_id).unwrap();
        let element_data = match &node.node_type {
            AceNodeType::Element(e) => e,
            _ => return ComputedStyle::default(), 
        };
        let mut style = if let Some(parent) = parent_style {
            let mut s = ComputedStyle::default();
            // Inherited properties
            s.color = parent.color.clone();
            s.font_size = parent.font_size.clone();
            s.font_family = parent.font_family.clone();
            s.font_weight = parent.font_weight.clone();
            s.text_align = parent.text_align.clone();
            s.line_height = parent.line_height.clone();
            s.opacity = parent.opacity;
            // Inherited properties Fase 2
            s.visibility = parent.visibility.clone();
            s.cursor = parent.cursor.clone();
            s.pointer_events = parent.pointer_events.clone();
            
            // Inherit custom properties
            s.custom_properties = parent.custom_properties.clone();
            
            // Non-inherited properties (layout, background, borders) are already reset to default() in s
            s
        } else {
            ComputedStyle::default()
        };

        if let Some(node) = dom.get_node(node_id) {
             if let AceNodeType::Element(el) = &node.node_type {
                 let ace_element = AceElement { dom, index: node_id, hovered_element, focused_element, active_element };
                 let mut matched_rules = Vec::new();

                 // 1. Process User Agent Rules (via RuleMap)
                 self.user_agent_rule_map.match_element(&ace_element, &mut matched_rules, CascadeOrigin::UserAgent, 0);

                 // 2. Process Author Rules (via RuleMap)
                 self.author_rule_map.match_element(&ace_element, &mut matched_rules, CascadeOrigin::Author, 1000);

                  // 3. Process Media Rules
                  for media_rule in &self.media_rules {
                      if matches_media_query(&media_rule.media_query, vw, vh, color_scheme) {
                           for (order, rule) in media_rule.rules.iter().enumerate() {
                               for selector in rule.selectors.slice() {
                                   let mut caches = selectors::matching::SelectorCaches::default();
                                   let mut context = MatchingContext::new(
                                       MatchingMode::Normal,
                                       None,
                                       &mut caches,
                                       selectors::matching::QuirksMode::NoQuirks,
                                       selectors::matching::NeedsSelectorFlags::No,
                                       selectors::matching::MatchingForInvalidation::No,
                                   );

                                   if selectors::matching::matches_selector(selector, 0, None, &ace_element, &mut context) {
                                       matched_rules.push(MatchedRule {
                                           priority: CascadePriority {
                                               origin: CascadeOrigin::AuthorMedia,
                                               important: false,
                                               specificity: selector.specificity(),
                                               order: order + 2000,
                                           },
                                           rule
                                       });
                                   }
                               }
                           }
                       }
                   }

                   // 4. Process Supports Rules
                   for (i, rule_block) in self.supports_rules.iter().enumerate() {
                       if matches_supports(&rule_block.condition) {
                           for (order, rule) in rule_block.rules.iter().enumerate() {
                               for selector in rule.selectors.slice() {
                                   let mut caches = selectors::matching::SelectorCaches::default();
                                   let mut context = MatchingContext::new(MatchingMode::Normal, None, &mut caches, selectors::matching::QuirksMode::NoQuirks, selectors::matching::NeedsSelectorFlags::No, selectors::matching::MatchingForInvalidation::No);
                                   if selectors::matching::matches_selector(selector, 0, None, &ace_element, &mut context) {
                                       matched_rules.push(MatchedRule {
                                           priority: CascadePriority { origin: CascadeOrigin::AuthorMedia, important: false, specificity: selector.specificity(), order: 3000 + i * 100 + order },
                                           rule
                                       });
                                   }
                               }
                           }
                       }
                   }

                   // 5. Process Container Rules (Simple matching stub)
                   for (i, rule_block) in self.container_rules.iter().enumerate() {
                       if matches_container(&rule_block.condition, vw) {
                           for (order, rule) in rule_block.rules.iter().enumerate() {
                               for selector in rule.selectors.slice() {
                                   let mut caches = selectors::matching::SelectorCaches::default();
                                   let mut context = MatchingContext::new(MatchingMode::Normal, None, &mut caches, selectors::matching::QuirksMode::NoQuirks, selectors::matching::NeedsSelectorFlags::No, selectors::matching::MatchingForInvalidation::No);
                                   if selectors::matching::matches_selector(selector, 0, None, &ace_element, &mut context) {
                                       matched_rules.push(MatchedRule {
                                           priority: CascadePriority { origin: CascadeOrigin::AuthorMedia, important: false, specificity: selector.specificity(), order: 4000 + i * 100 + order },
                                           rule
                                       });
                                   }
                               }
                           }
                       }
                   }

                  // Sort by priority (origin -> specificity -> order)
                  matched_rules.sort_by(|a, b| {
                      a.priority.cmp(&b.priority)
                  });

                  let parent_font_size = parent_style.map(|s| s.font_size).unwrap_or(16.0);
                  let root_font_size = root_style.map(|s| s.font_size).unwrap_or(16.0);

                  // Phase 1: Resolve font-size (because em in other properties depends on it)
                  for match_rule in &matched_rules {
                      for decl in &match_rule.rule.declarations {
                          apply_single_declaration(&mut style, decl, parent_font_size, root_font_size, true);
                      }
                  }

                  // REAL CSS: Estilos inline para font-size (Phase 1)
                  if let AceNodeType::Element(el) = &node.node_type {
                      if let Some(inline_str) = el.attributes.get("style") {
                          let inline_decls = parse_inline_declarations(inline_str);
                          for decl in &inline_decls {
                              apply_single_declaration(&mut style, decl, parent_font_size, root_font_size, true);
                          }
                      }
                  }

                  let current_font_size = style.font_size;

                  // Phase 2: Apply all rules
                  // Store property importance to handle !important correctly
                  let mut property_importance = std::collections::HashMap::new();

                  for match_rule in matched_rules {
                      for decl in &match_rule.rule.declarations {
                          let prop_name = decl.name.clone();
                          let is_important = decl.important;
                          
                          // Cascading logic for !important:
                          // Important Author wins over Normal Author.
                          // Normal Author wins over Normal UA.
                          // But wait, the standard order is:
                          // Normal UA < Normal Author < Important Author < Important UA
                          
                          let current_weight = match (match_rule.priority.origin, is_important) {
                              (CascadeOrigin::UserAgent, false) => 1,
                              (CascadeOrigin::Author, false) | (CascadeOrigin::AuthorMedia, false) => 2,
                              (CascadeOrigin::Author, true) | (CascadeOrigin::AuthorMedia, true) => 3,
                              (CascadeOrigin::UserAgent, true) => 4,
                          };

                          let prev_weight = *property_importance.get(&prop_name).unwrap_or(&0);
                          
                          if current_weight > prev_weight || (current_weight == prev_weight && match_rule.priority.specificity >= 0) {
                               // Specificity check is implicit if we sort rules by specificity first, 
                               // but here we are iterating over already sorted rules.
                               // However, the current_weight handles the origin/importance jump.
                               apply_single_declaration(&mut style, decl, current_font_size, root_font_size, false);
                               property_importance.insert(prop_name, current_weight);
                          }
                      }
                  }
                  
                  // Aplicar outline padrão para :focus se nenhum outline foi explicitamente definido
                  if focused_element == Some(node_id) && style.outline.is_none() {
                      style.outline = Some(crate::engine::style::css_values::Outline {
                          width: 2.0,
                          color: crate::engine::style::css_values::CssColor::Named("#0066ff".to_string()),
                          style: "solid".to_string(),
                          offset: 2.0,
                      });
                  }
                     }
                    }

        style
    }

    pub fn calculate_pseudo_style(&self, dom: &AceDOM, node_id: usize, pseudo: &AcePseudoElement, hovered_element: Option<usize>, focused_element: Option<usize>, active_element: Option<usize>, vw: f32, vh: f32, color_scheme: &str) -> ComputedStyle {
        let mut style = ComputedStyle::default();
        
        // Default display for pseudo-elements is inline
        style.display = CssDisplay::Inline;
        
        if let Some(node) = dom.get_node(node_id) {
             if let AceNodeType::Element(_) = &node.node_type {
                 let ace_element = AceElement { dom, index: node_id, hovered_element, focused_element, active_element };
                 let mut matched_rules = Vec::new();

                 // Injetar lógicas de cálculo com viewport aqui se necessário, 
                 // mas o principal é passar adiante para resolve_length.
                 
                 // ... rest of matching ... (simplified for this edit)

                 // 1. Process User Agent Rules
                 for (order, rule) in self.user_agent_rules.iter().enumerate() {
                     for selector in rule.selectors.slice() {
                         if selector.pseudo_element() == Some(pseudo) {
                              let mut caches = selectors::matching::SelectorCaches::default();
                              let mut context = MatchingContext::new(
                                 MatchingMode::Normal,
                                 None,
                                 &mut caches,
                                 selectors::matching::QuirksMode::NoQuirks,
                                 selectors::matching::NeedsSelectorFlags::No,
                                 selectors::matching::MatchingForInvalidation::No,
                             );
                             if selectors::matching::matches_selector(selector, 0, None, &ace_element, &mut context) {
                                 matched_rules.push(MatchedRule {
                                     priority: CascadePriority { origin: CascadeOrigin::UserAgent, important: false, specificity: selector.specificity(), order },
                                     rule
                                 });
                             }
                         }
                     }
                 }
                 
                 // 2. Process User Rules
                 for (order, rule) in self.rules.iter().enumerate() {
                     for selector in rule.selectors.slice() {
                         if selector.pseudo_element() == Some(pseudo) {
                              let mut caches = selectors::matching::SelectorCaches::default();
                              let mut context = MatchingContext::new(
                                 MatchingMode::Normal,
                                 None,
                                 &mut caches,
                                 selectors::matching::QuirksMode::NoQuirks,
                                 selectors::matching::NeedsSelectorFlags::No,
                                 selectors::matching::MatchingForInvalidation::No,
                             );
                             if selectors::matching::matches_selector(selector, 0, None, &ace_element, &mut context) {
                                 matched_rules.push(MatchedRule {
                                     priority: CascadePriority { origin: CascadeOrigin::Author, important: false, specificity: selector.specificity(), order: order + 1000 },
                                     rule
                                 });
                             }
                         }
                     }
                 }
                 
                 matched_rules.sort_by(|a, b| a.priority.cmp(&b.priority));
                 
                 for match_rule in &matched_rules {
                     for decl in &match_rule.rule.declarations {
                         let val = decl.value.trim();
                         match decl.name.as_str() {
                             "content" => style.content = parse_content(val),
                             "color" => style.color = parse_color(val),
                             "font-size" => style.font_size = resolve_length(&parse_length(val), 16.0, 16.0, vw, vh),
                             "display" => {
                                 style.display = match val {
                                     "block" => CssDisplay::Block,
                                     "flex" => CssDisplay::Flex,
                                     "grid" => CssDisplay::Grid,
                                     "none" => CssDisplay::None,
                                     _ => CssDisplay::Inline,
                                 };
                             },
                             "background-color" => style.background_color = parse_color(val),
                             "width" => style.width = parse_length(val),
                             "height" => style.height = parse_length(val),
                             "opacity" => if let Ok(n) = val.parse::<f32>() { style.opacity = n.clamp(0.0, 1.0); },
                             "z-index" => if let Ok(n) = val.parse::<i32>() { style.z_index = n; },
                             _ => {}
                         }
                     }
                 }
              }
        }
        style
    }
}

// Parsing Helpers
fn parse_length(val: &str) -> CssLength {
    let val = val.trim();
    if val == "auto" { return CssLength::Auto; }
    if val == "0" { return CssLength::Zero; }
    
    if val.starts_with("clamp(") && val.ends_with(")") {
        let inner = &val[6..val.len()-1];
        let parts = split_comma_top_level(inner);
        if parts.len() == 3 {
            return CssLength::Clamp(
                Box::new(parse_length(parts[0])),
                Box::new(parse_length(parts[1])),
                Box::new(parse_length(parts[2]))
            );
        }
    }

    if val.starts_with("min(") && val.ends_with(")") {
        let inner = &val[4..val.len()-1];
        let parts = split_comma_top_level(inner);
        return CssLength::Min(parts.iter().map(|p| parse_length(p)).collect());
    }

    if val.starts_with("max(") && val.ends_with(")") {
        let inner = &val[4..val.len()-1];
        let parts = split_comma_top_level(inner);
        return CssLength::Max(parts.iter().map(|p| parse_length(p)).collect());
    }

    if val.starts_with("calc(") && val.ends_with(")") {
        let inner = &val[5..val.len()-1];
        // Support simple A + B or A - B if units are same
        if inner.contains(" + ") || inner.contains(" - ") || inner.contains(" * ") || inner.contains(" / ") {
            // Very basic calc parser for same-unit additions
            let parts: Vec<&str> = if inner.contains(" + ") {
                inner.split(" + ").collect()
            } else if inner.contains(" - ") {
                inner.split(" - ").collect()
            } else if inner.contains(" * ") {
                inner.split(" * ").collect()
            } else {
                inner.split(" / ").collect()
            };
            
            if parts.len() == 2 {
                let l1 = parse_length(parts[0].trim());
                let l2 = parse_length(parts[1].trim());
                if let (CssLength::Px(v1), CssLength::Px(v2)) = (&l1, &l2) {
                    if inner.contains(" + ") { return CssLength::Px(v1 + v2); }
                    if inner.contains(" - ") { return CssLength::Px(v1 - v2); }
                }
                // Handle multiplication/division by unitless number
                if let CssLength::Px(v1) = l1 {
                    if let Ok(v2) = parts[1].trim().parse::<f32>() {
                        if inner.contains(" * ") { return CssLength::Px(v1 * v2); }
                        if inner.contains(" / ") && v2 != 0.0 { return CssLength::Px(v1 / v2); }
                    }
                }
            }
        }
        return CssLength::Calc(inner.to_string());
    }
    
    if let Some(n) = val.strip_suffix("px") {
        if let Ok(num) = n.trim().parse::<f32>() { return CssLength::Px(num); }
    }
    if let Some(n) = val.strip_suffix("%") {
        if let Ok(num) = n.trim().parse::<f32>() { return CssLength::Percent(num); }
    }
    if let Some(n) = val.strip_suffix("vw") {
        if let Ok(num) = n.trim().parse::<f32>() { return CssLength::Vw(num); }
    }
    if let Some(n) = val.strip_suffix("vh") {
        if let Ok(num) = n.trim().parse::<f32>() { return CssLength::Vh(num); }
    }
    if let Some(n) = val.strip_suffix("rem") {
        if let Ok(num) = n.trim().parse::<f32>() { return CssLength::Rem(num); }
    }
    if let Some(n) = val.strip_suffix("em") {
        if let Ok(num) = n.trim().parse::<f32>() { return CssLength::Em(num); }
    }
    if let Some(n) = val.strip_suffix("fr") {
        if let Ok(num) = n.trim().parse::<f32>() { return CssLength::Fr(num); }
    }
    
    if val == "min-content" { return CssLength::MinContent; }
    if val == "max-content" { return CssLength::MaxContent; }
    if val == "auto-fill" { return CssLength::AutoFill; }
    if val == "auto-fit" { return CssLength::AutoFit; }
    
    CssLength::Auto
}

fn parse_grid_placement(val: &str) -> CssLength {
    let val = val.trim();
    if let Ok(num) = val.parse::<f32>() {
        return CssLength::Number(num);
    }
    if val.starts_with("span ") {
        if let Ok(num) = val[5..].trim().parse::<u16>() {
            return CssLength::Span(num);
        }
    }
    // Default to Name for identifiers
    if !val.is_empty() && val != "auto" {
        return CssLength::Name(val.to_string());
    }
    CssLength::Auto
}

fn split_comma_top_level(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (i, c) in s.chars().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(s[start..].trim());
    parts
}

fn parse_length_list(val: &str) -> Vec<CssLength> {
    val.split_whitespace().map(parse_length).collect()
}

fn split_spaces_top_level(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut brace_depth = 0;
    for (i, c) in s.chars().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            '[' => brace_depth += 1,
            ']' => brace_depth -= 1,
            ' ' | '\t' | '\n' | '\r' if depth == 0 && brace_depth == 0 => {
                if i > start {
                    parts.push(s[start..i].trim());
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        let final_part = s[start..].trim();
        if !final_part.is_empty() {
            parts.push(final_part);
        }
    }
    parts
}

fn parse_grid_track_list(val: &str) -> Vec<CssLength> {
    let parts = split_spaces_top_level(val);
    let mut tracks = Vec::new();
    
    for part in parts {
        let part = part.trim();
        if part.is_empty() { continue; }

        if part == "subgrid" {
            tracks.push(CssLength::Subgrid);
            continue;
        }
        
        if part.starts_with('[') && part.ends_with(']') {
            let names = part[1..part.len()-1].split_whitespace().map(|s| s.to_string()).collect();
            tracks.push(CssLength::LineNames(names));
            continue;
        }

        if part.starts_with("repeat(") && part.ends_with(")") {
             let inner = &part[7..part.len()-1];
             let args = split_comma_top_level(inner);
             if args.len() >= 2 {
                 let count_str = args[0].trim();
                 let track_str = args[1].trim();
                 
                 let repeat_mode = count_str.to_string();
                 let sub_tracks = parse_grid_track_list(track_str);
                 
                 if let Ok(n) = count_str.parse::<usize>() {
                     for _ in 0..n {
                         tracks.extend(sub_tracks.clone());
                     }
                 } else {
                     tracks.push(CssLength::Repeat(repeat_mode, sub_tracks));
                 }
             }
        } else if part.starts_with("minmax(") && part.ends_with(")") {
            let inner = &part[7..part.len()-1];
            let args = split_comma_top_level(inner);
            if args.len() == 2 {
                let min = parse_length(args[0]);
                let max = parse_length(args[1]);
                tracks.push(CssLength::MinMax(Box::new(min), Box::new(max)));
            }
        } else {
            tracks.push(parse_length(part));
        }
    }
    tracks
}

fn parse_grid_template_areas(val: &str) -> Vec<String> {
   let mut areas = Vec::new();
   let mut in_quote = false;
   let mut current = String::new();
   for c in val.chars() {
       if c == '"' || c == '\'' {
           if in_quote {
               if !current.trim().is_empty() {
                   areas.push(current.clone());
               }
               current.clear();
               in_quote = false;
           } else {
               in_quote = true;
           }
       } else if in_quote {
           current.push(c);
       }
   }
   areas
}

fn parse_content(val: &str) -> CssContent {
    use self::css_values::CssContent;
    
    let val = val.trim();
    match val {
        "none" => CssContent::None,
        "normal" => CssContent::Normal,
        _ => {
            // Handle quoted strings: "text" or 'text'
            if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')) {
                let text = &val[1..val.len()-1];
                CssContent::String(text.to_string())
            } else {
                // Default to normal if not recognized
                CssContent::Normal
            }
        }
    }
}

fn parse_color(val: &str) -> CssColor {
    let val = val.trim();
    match val {
        "transparent" => CssColor::Transparent,
        "currentcolor" => CssColor::CurrentColor,
        _ => {
            // Handle rgba() and rgb()
            if val.starts_with("rgba(") {
                let inner = &val[5..val.len()-1];
                let parts: Vec<&str> = inner.split(',').collect();
                if parts.len() >= 4 {
                    let r = parts[0].trim().parse::<u8>().unwrap_or(0);
                    let g = parts[1].trim().parse::<u8>().unwrap_or(0);
                    let b = parts[2].trim().parse::<u8>().unwrap_or(0);
                    let a = parts[3].trim().parse::<f32>().unwrap_or(1.0);
                    return CssColor::Rgba(r, g, b, a);
                }
            }
            if val.starts_with("rgb(") {
                let inner = &val[4..val.len()-1];
                let parts: Vec<&str> = inner.split(',').collect();
                if parts.len() >= 3 {
                    let r = parts[0].trim().parse::<u8>().unwrap_or(0);
                    let g = parts[1].trim().parse::<u8>().unwrap_or(0);
                    let b = parts[2].trim().parse::<u8>().unwrap_or(0);
                    return CssColor::Rgba(r, g, b, 1.0);
                }
            }
            // Handle hex colors
            if val.starts_with('#') && val.len() == 7 {
                let r = u8::from_str_radix(&val[1..3], 16).unwrap_or(0);
                let g = u8::from_str_radix(&val[3..5], 16).unwrap_or(0);
                let b = u8::from_str_radix(&val[5..7], 16).unwrap_or(0);
                return CssColor::Rgba(r, g, b, 1.0);
            }
            CssColor::Named(val.to_string())
        }
    }
}

fn parse_display(val: &str) -> CssDisplay {
    match val.trim() {
        "none" => CssDisplay::None,
        "block" => CssDisplay::Block,
        "inline-block" => CssDisplay::InlineBlock,
        "inline" => CssDisplay::Inline,
        "flex" => CssDisplay::Flex,
        "inline-flex" => CssDisplay::InlineFlex,
        "grid" => CssDisplay::Grid,
        "contents" => CssDisplay::Contents,
        "table" => CssDisplay::Table,
        "table-row" => CssDisplay::TableRow,
        "table-cell" => CssDisplay::TableCell,
        "table-header-group" | "table-header" => CssDisplay::TableHeader,
        _ => CssDisplay::Inline,
    }
}

fn parse_position(val: &str) -> CssPosition {
    match val.trim() {
        "static" => CssPosition::Static,
        "relative" => CssPosition::Relative,
        "absolute" => CssPosition::Absolute,
        "fixed" => CssPosition::Fixed,
        "sticky" => CssPosition::Sticky,
        _ => CssPosition::Static,
    }
}

fn parse_overflow(val: &str) -> CssOverflow {
    match val.trim() {
        "visible" => CssOverflow::Visible,
        "hidden" => CssOverflow::Hidden,
        "scroll" => CssOverflow::Scroll,
        "auto" => CssOverflow::Auto,
        _ => CssOverflow::Visible,
    }
}

fn parse_float(val: &str) -> CssFloat {
    match val.trim() {
        "left" => CssFloat::Left,
        "right" => CssFloat::Right,
        "none" => CssFloat::None,
        _ => CssFloat::None,
    }
}

fn parse_flex_direction(val: &str) -> CssFlexDirection {
    match val.trim() {
        "row" => CssFlexDirection::Row,
        "row-reverse" => CssFlexDirection::RowReverse,
        "column" => CssFlexDirection::Column,
        "column-reverse" => CssFlexDirection::ColumnReverse,
        _ => CssFlexDirection::Row,
    }
}

fn parse_justify_content(val: &str) -> CssJustifyContent {
    match val.trim() {
        "flex-start" => CssJustifyContent::FlexStart,
        "flex-end" => CssJustifyContent::FlexEnd,
        "center" => CssJustifyContent::Center,
        "space-between" => CssJustifyContent::SpaceBetween,
        "space-around" => CssJustifyContent::SpaceAround,
        "space-evenly" => CssJustifyContent::SpaceEvenly,
        _ => CssJustifyContent::FlexStart,
    }
}

fn parse_align_items(val: &str) -> CssAlignItems {
    match val.trim() {
        "flex-start" => CssAlignItems::FlexStart,
        "flex-end" => CssAlignItems::FlexEnd,
        "center" => CssAlignItems::Center,
        "baseline" => CssAlignItems::Baseline,
        "stretch" => CssAlignItems::Stretch,
        "auto" => CssAlignItems::Auto,
        _ => CssAlignItems::Stretch,
    }
}

fn parse_align_content(val: &str) -> CssAlignContent {
    match val.trim() {
        "flex-start" => CssAlignContent::FlexStart,
        "flex-end" => CssAlignContent::FlexEnd,
        "center" => CssAlignContent::Center,
        "space-between" => CssAlignContent::SpaceBetween,
        "space-around" => CssAlignContent::SpaceAround,
        "space-evenly" => CssAlignContent::SpaceEvenly,
        "stretch" => CssAlignContent::Stretch,
        _ => CssAlignContent::Stretch,
    }
}

fn parse_flex_wrap(val: &str) -> CssFlexWrap {
    match val.trim() {
        "nowrap" => CssFlexWrap::NoWrap,
        "wrap" => CssFlexWrap::Wrap,
        "wrap-reverse" => CssFlexWrap::WrapReverse,
        _ => CssFlexWrap::NoWrap,
    }
}

fn parse_border_radius(val: &str) -> f32 {
    let val = val.trim();
    if let Some(n) = val.strip_suffix("px") {
        if let Ok(num) = n.trim().parse::<f32>() {
            return num;
        }
    }
    if let Some(n) = val.strip_suffix("%") {
        if let Ok(num) = n.trim().parse::<f32>() {
            return num;
        }
    }
    0.0
}

fn parse_text_align(val: &str) -> CssTextAlign {
    match val.trim() {
        "left" => CssTextAlign::Left,
        "right" => CssTextAlign::Right,
        "center" => CssTextAlign::Center,
        "justify" => CssTextAlign::Justify,
        "start" => CssTextAlign::Start,
        "end" => CssTextAlign::End,
        _ => CssTextAlign::Left,
    }
}

fn parse_font_weight(val: &str) -> CssFontWeight {
    match val.trim() {
        "normal" => CssFontWeight::Normal,
        "bold" => CssFontWeight::Bold,
        "lighter" => CssFontWeight::Lighter,
        "bolder" => CssFontWeight::Bolder,
        _ => {
            // Try to parse as number
            if let Ok(n) = val.parse::<f32>() {
                CssFontWeight::Weight(n)
            } else {
                CssFontWeight::Normal
            }
        }
    }
}

fn parse_box_sizing(val: &str) -> CssBoxSizing {
    match val.trim() {
        "border-box" => CssBoxSizing::BorderBox,
        "content-box" => CssBoxSizing::ContentBox,
        _ => CssBoxSizing::ContentBox,
    }
}

fn parse_visibility(val: &str) -> CssVisibility {
    match val.trim() {
        "visible" => CssVisibility::Visible,
        "hidden" => CssVisibility::Hidden,
        "collapse" => CssVisibility::Collapse,
        _ => CssVisibility::Visible,
    }
}

fn parse_cursor(val: &str) -> CssCursor {
    match val.trim() {
        "auto" => CssCursor::Auto,
        "default" => CssCursor::Default,
        "pointer" => CssCursor::Pointer,
        "text" => CssCursor::Text,
        "wait" => CssCursor::Wait,
        "help" => CssCursor::Help,
        "not-allowed" => CssCursor::NotAllowed,
        "grab" => CssCursor::Grab,
        "grabbing" => CssCursor::Grabbing,
        _ => CssCursor::Auto,
    }
}

fn parse_object_fit(val: &str) -> CssObjectFit {
    match val.trim() {
        "fill" => CssObjectFit::Fill,
        "contain" => CssObjectFit::Contain,
        "cover" => CssObjectFit::Cover,
        "none" => CssObjectFit::None,
        "scale-down" => CssObjectFit::ScaleDown,
        _ => CssObjectFit::Fill,
    }
}

fn parse_object_position(val: &str) -> CssObjectPosition {
    let parts: Vec<&str> = val.split_whitespace().collect();
    if parts.len() >= 2 {
        CssObjectPosition {
            x: parse_length(parts[0]),
            y: parse_length(parts[1]),
        }
    } else if parts.len() == 1 {
        CssObjectPosition {
            x: parse_length(parts[0]),
            y: CssLength::Percent(50.0),
        }
    } else {
        CssObjectPosition::default()
    }
}

fn parse_pointer_events(val: &str) -> CssPointerEvents {
    match val.trim() {
        "auto" => CssPointerEvents::Auto,
        "none" => CssPointerEvents::None,
        _ => CssPointerEvents::Auto,
    }
}

fn parse_filters(val: &str) -> Vec<CssFilter> {
    let mut filters = Vec::new();
    let val = val.trim();
    if val == "none" || val.is_empty() { return filters; }

    // Simple parser for filter functions: blur(5px) grayscale(50%) etc.
    let mut i = 0;
    let chars: Vec<char> = val.chars().collect();
    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == ',') { i += 1; }
        if i >= chars.len() { break; }

        let start = i;
        while i < chars.len() && chars[i] != '(' { i += 1; }
        if i >= chars.len() { break; }
        
        let func_name = val[start..i].trim();
        i += 1; // skip '('
        
        let arg_start = i;
        let mut depth = 1;
        while i < chars.len() && depth > 0 {
            if chars[i] == '(' { depth += 1; }
            else if chars[i] == ')' { depth -= 1; }
            i += 1;
        }
        if depth == 0 {
            let arg = &val[arg_start..i-1].trim();
            match func_name {
                "blur" => filters.push(CssFilter::Blur(parse_length(arg))),
                "brightness" => if let Ok(n) = arg.strip_suffix('%').unwrap_or(arg).parse::<f32>() {
                    filters.push(CssFilter::Brightness(if arg.ends_with('%') { n / 100.0 } else { n }));
                },
                "grayscale" => if let Ok(n) = arg.strip_suffix('%').unwrap_or(arg).parse::<f32>() {
                    filters.push(CssFilter::Grayscale(if arg.ends_with('%') { n / 100.0 } else { n }));
                },
                "invert" => if let Ok(n) = arg.strip_suffix('%').unwrap_or(arg).parse::<f32>() {
                    filters.push(CssFilter::Invert(if arg.ends_with('%') { n / 100.0 } else { n }));
                },
                "sepia" => if let Ok(n) = arg.strip_suffix('%').unwrap_or(arg).parse::<f32>() {
                    filters.push(CssFilter::Sepia(if arg.ends_with('%') { n / 100.0 } else { n }));
                },
                "opacity" => if let Ok(n) = arg.strip_suffix('%').unwrap_or(arg).parse::<f32>() {
                    filters.push(CssFilter::Opacity(if arg.ends_with('%') { n / 100.0 } else { n }));
                },
                _ => {}
            }
        }
    }
    filters
}

fn parse_blend_mode(val: &str) -> CssBlendMode {
    match val.trim() {
        "multiply" => CssBlendMode::Multiply,
        "screen" => CssBlendMode::Screen,
        "overlay" => CssBlendMode::Overlay,
        "darken" => CssBlendMode::Darken,
        "lighten" => CssBlendMode::Lighten,
        "color-dodge" => CssBlendMode::ColorDodge,
        "color-burn" => CssBlendMode::ColorBurn,
        "hard-light" => CssBlendMode::HardLight,
        "soft-light" => CssBlendMode::SoftLight,
        "difference" => CssBlendMode::Difference,
        "exclusion" => CssBlendMode::Exclusion,
        "hue" => CssBlendMode::Hue,
        "saturation" => CssBlendMode::Saturation,
        "color" => CssBlendMode::Color,
        "luminosity" => CssBlendMode::Luminosity,
        _ => CssBlendMode::Normal,
    }
}

fn parse_transitions(val: &str) -> Vec<self::css_values::CssTransition> {
    let mut transitions = Vec::new();
    for part in val.split(',') {
        let segments: Vec<&str> = part.trim().split_whitespace().collect();
        if segments.is_empty() { continue; }
        
        let mut t = self::css_values::CssTransition {
            property: segments[0].to_string(),
            duration_ms: 0,
            timing_function: "ease".to_string(),
            delay_ms: 0,
        };
        
        for segment in &segments[1..] {
            if segment.ends_with("ms") {
                if let Ok(ms) = segment.trim_end_matches("ms").parse::<u32>() {
                    if t.duration_ms == 0 { t.duration_ms = ms; } else { t.delay_ms = ms; }
                }
            } else if segment.ends_with('s') {
                if let Ok(s) = segment.trim_end_matches('s').parse::<f32>() {
                    let ms = (s * 1000.0) as u32;
                    if t.duration_ms == 0 { t.duration_ms = ms; } else { t.delay_ms = ms; }
                }
            } else {
                t.timing_function = segment.to_string();
            }
        }
        transitions.push(t);
    }
    transitions
}

fn parse_animations(val: &str) -> Vec<self::css_values::CssAnimation> {
    let mut animations = Vec::new();
    for part in val.split(',') {
        let segments: Vec<&str> = part.trim().split_whitespace().collect();
        if segments.is_empty() { continue; }
        
        let mut anim = self::css_values::CssAnimation {
            name: segments[0].to_string(),
            duration_ms: 0,
            timing_function: "ease".to_string(),
            delay_ms: 0,
            iteration_count: "1".to_string(),
            direction: "normal".to_string(),
            fill_mode: "none".to_string(),
        };
        
        for segment in &segments[1..] {
            if segment.ends_with("ms") || segment.ends_with('s') {
                let ms = if segment.ends_with("ms") {
                    segment.trim_end_matches("ms").parse::<u32>().unwrap_or(0)
                } else {
                    (segment.trim_end_matches('s').parse::<f32>().unwrap_or(0.0) * 1000.0) as u32
                };
                if anim.duration_ms == 0 { anim.duration_ms = ms; } else { anim.delay_ms = ms; }
            } else if *segment == "infinite" || segment.chars().all(|c| c.is_ascii_digit()) {
                anim.iteration_count = segment.to_string();
            } else if matches!(*segment, "normal" | "reverse" | "alternate" | "alternate-reverse") {
                anim.direction = segment.to_string();
            } else if matches!(*segment, "none" | "forwards" | "backwards" | "both") {
                anim.fill_mode = segment.to_string();
            } else if matches!(*segment, "ease" | "linear" | "ease-in" | "ease-out" | "ease-in-out" | "step-start" | "step-end") {
                anim.timing_function = segment.to_string();
            } else {
                // Could be name if first wasn't or additional prop
            }
        }
        animations.push(anim);
    }
    animations
}

fn parse_transform(val: &str) -> Vec<TransformFunction> {
    let mut transforms = Vec::new();
    let val = val.trim();
    if val == "none" || val.is_empty() {
        return transforms;
    }

    // Simple parser for function(args)
    let mut i = 0;
    let chars: Vec<char> = val.chars().collect();
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() { i += 1; }
        if i >= chars.len() { break; }

        let start = i;
        while i < chars.len() && chars[i] != '(' { i += 1; }
        let func_name = val[start..i].trim();
        
        if i < chars.len() && chars[i] == '(' {
            i += 1;
            let arg_start = i;
            let mut brace_count = 1;
            while i < chars.len() && brace_count > 0 {
                if chars[i] == '(' { brace_count += 1; }
                else if chars[i] == ')' { brace_count -= 1; }
                i += 1;
            }
            let args_str = &val[arg_start..i-1];
            let args: Vec<&str> = args_str.split(',').collect();

            match func_name {
                "translate" => {
                    if args.len() >= 2 {
                        transforms.push(TransformFunction::Translate(parse_length(args[0]), parse_length(args[1])));
                    } else if args.len() == 1 {
                        transforms.push(TransformFunction::Translate(parse_length(args[0]), CssLength::Zero));
                    }
                }
                "translateX" => {
                    if !args.is_empty() { transforms.push(TransformFunction::TranslateX(parse_length(args[0]))); }
                }
                "translateY" => {
                    if !args.is_empty() { transforms.push(TransformFunction::TranslateY(parse_length(args[0]))); }
                }
                "scale" => {
                    if args.len() >= 2 {
                        let sx = args[0].trim().parse().unwrap_or(1.0);
                        let sy = args[1].trim().parse().unwrap_or(sx);
                        transforms.push(TransformFunction::Scale(sx, sy));
                    } else if args.len() == 1 {
                        let s = args[0].trim().parse().unwrap_or(1.0);
                        transforms.push(TransformFunction::Scale(s, s));
                    }
                }
                "rotate" => {
                    if !args.is_empty() {
                        let deg_str = args[0].trim().strip_suffix("deg").unwrap_or(args[0].trim());
                        transforms.push(TransformFunction::Rotate(deg_str.parse().unwrap_or(0.0)));
                    }
                }
                "rotateX" => {
                    if !args.is_empty() {
                        let deg_str = args[0].trim().strip_suffix("deg").unwrap_or(args[0].trim());
                        transforms.push(TransformFunction::RotateX(deg_str.parse().unwrap_or(0.0)));
                    }
                }
                "rotateY" => {
                    if !args.is_empty() {
                        let deg_str = args[0].trim().strip_suffix("deg").unwrap_or(args[0].trim());
                        transforms.push(TransformFunction::RotateY(deg_str.parse().unwrap_or(0.0)));
                    }
                }
                "rotateZ" => {
                    if !args.is_empty() {
                        let deg_str = args[0].trim().strip_suffix("deg").unwrap_or(args[0].trim());
                        transforms.push(TransformFunction::RotateZ(deg_str.parse().unwrap_or(0.0)));
                    }
                }
                _ => {}
            }
        } else {
            i += 1;
        }
    }
    transforms
}

// MELHORIA: Parse box-shadow
fn parse_box_shadow(val: &str) -> Vec<BoxShadow> {
    let val = val.trim();
    if val == "none" || val.is_empty() {
        return Vec::new();
    }
    
    let mut shadows = Vec::new();
    
    // Split by comma for multiple shadows
    for shadow_val in val.split(',') {
        let shadow_val = shadow_val.trim();
        if shadow_val.is_empty() || shadow_val == "none" {
            continue;
        }
        
        let mut offset_x = 0.0f32;
        let mut offset_y = 0.0f32;
        let mut blur = 0.0f32;
        let mut spread = 0.0f32;
        let mut color = CssColor::Named("black".to_string());
        let mut inset = false;
        
        let parts: Vec<&str> = shadow_val.split_whitespace().collect();
        let mut i = 0;
        
        while i < parts.len() {
            let part = parts[i];
            
            // Check for inset keyword
            if part == "inset" {
                inset = true;
                i += 1;
                continue;
            }
            
            // Try to parse as length
            if let Some(n) = part.strip_suffix("px") {
                if let Ok(num) = n.parse::<f32>() {
                    if i == 0 || (i > 0 && parts.get(i - 1) == Some(&"inset")) {
                        offset_x = num;
                        i += 1;
                        if i < parts.len() {
                            if let Some(n2) = parts[i].strip_suffix("px") {
                                if let Ok(num2) = n2.parse::<f32>() {
                                    offset_y = num2;
                                    i += 1;
                                    continue;
                                }
                            }
                        }
                    } else if i > 0 {
                        if let Ok(num2) = part.parse::<f32>() {
                            if blur == 0.0 {
                                blur = num2;
                            } else {
                                spread = num2;
                            }
                            i += 1;
                            continue;
                        }
                    }
                }
            }
            
            // If not a number, might be a color
            if !part.ends_with("px") && !part.ends_with("em") && !part.ends_with("rem") {
                color = parse_color(part);
            }
            i += 1;
        }
        
        shadows.push(BoxShadow {
            offset_x,
            offset_y,
            blur,
            spread,
            color,
            inset,
        });
    }
    
    shadows
}

// MELHORIA: Parse text-shadow
fn parse_text_shadow(val: &str) -> Vec<TextShadow> {
    let val = val.trim();
    if val == "none" || val.is_empty() {
        return Vec::new();
    }
    
    let mut shadows = Vec::new();
    
    // Split by comma for multiple shadows
    for shadow_val in val.split(',') {
        let shadow_val = shadow_val.trim();
        if shadow_val.is_empty() || shadow_val == "none" {
            continue;
        }
        
        let mut offset_x = 0.0f32;
        let mut offset_y = 0.0f32;
        let mut blur = 0.0f32;
        let mut color = CssColor::Named("black".to_string());
        
        let parts: Vec<&str> = shadow_val.split_whitespace().collect();
        let mut i = 0;
        
        while i < parts.len() {
            let part = parts[i];
            
            // Try to parse as length
            if let Some(n) = part.strip_suffix("px") {
                if let Ok(num) = n.parse::<f32>() {
                    if i == 0 {
                        offset_x = num;
                    } else if i == 1 {
                        offset_y = num;
                    } else {
                        blur = num;
                    }
                    i += 1;
                    continue;
                }
            }
            
            // If not a number, might be a color
            if !part.ends_with("px") && !part.ends_with("em") && !part.ends_with("rem") {
                color = parse_color(part);
            }
            i += 1;
        }
        
        shadows.push(TextShadow {
            offset_x,
            offset_y,
            blur,
            color,
        });
    }
    
    shadows
}

// MELHORIA: Parse background-image (supports gradients)
fn parse_background_image(val: &str) -> BackgroundImage {
    let val = val.trim();
    
    if val == "none" || val.is_empty() {
        return BackgroundImage::None;
    }
    
    // Check for linear-gradient
    if val.starts_with("linear-gradient(") {
        if let Some(gradient) = parse_linear_gradient(val) {
            return BackgroundImage::Gradient(gradient);
        }
    }
    
    // Check for radial-gradient
    if val.starts_with("radial-gradient(") {
        if let Some(gradient) = parse_radial_gradient(val) {
            return BackgroundImage::Gradient(gradient);
        }
    }
    
    // Check for url()
    if val.starts_with("url(") {
        if let Some(end) = val.find(')') {
            let url = &val[4..end];
            return BackgroundImage::Url(url.to_string());
        }
    }
    
    // Check if it's a solid color
    if !val.contains("gradient") && !val.contains("url(") {
        return BackgroundImage::Color(parse_color(val));
    }
    
    BackgroundImage::None
}

fn parse_linear_gradient(val: &str) -> Option<Gradient> {
    let inner = val.strip_prefix("linear-gradient(")?.strip_suffix(')')?;
    
    let mut angle = 180.0f32; // Default is to bottom (180 degrees)
    let mut stops = Vec::new();
    
    // Parse angle if present
    let mut remaining = inner;
    if remaining.starts_with("to ") {
        // Handle "to right", "to bottom right", etc.
        let parts: Vec<&str> = remaining.split_whitespace().collect();
        if parts.len() >= 2 {
            let direction = parts[1];
            angle = match direction {
                "top" => 0.0,
                "right" => 90.0,
                "bottom" => 180.0,
                "left" => 270.0,
                _ => {
                    if parts.len() >= 3 {
                        let dir2 = parts[2];
                        match (direction, dir2) {
                            ("top", "left") => 315.0,
                            ("top", "right") => 45.0,
                            ("bottom", "left") => 225.0,
                            ("bottom", "right") => 135.0,
                            _ => 180.0,
                        }
                    } else {
                        180.0
                    }
                }
            };
            remaining = remaining.splitn(2, ')').nth(1).unwrap_or("");
        }
    } else if remaining.starts_with("deg") {
        if let Some(deg) = remaining.split_whitespace().next() {
            if let Some(n) = deg.strip_suffix("deg") {
                if let Ok(a) = n.parse::<f32>() {
                    angle = a;
                }
            }
        }
        // Find the first color stop
        if let Some(idx) = remaining.find(',') {
            remaining = &remaining[idx + 1..];
        }
    } else if let Some(idx) = remaining.find(',') {
        // Check if first part is angle in degrees
        let first = remaining[..idx].trim();
        if let Ok(a) = first.parse::<f32>() {
            angle = a;
            remaining = &remaining[idx + 1..];
        }
    }
    
    // Parse color stops
    parse_color_stops(remaining, &mut stops);
    
    if stops.is_empty() {
        // Add default stops
        stops.push(GradientStop { color: CssColor::Transparent, position: Some(0.0) });
        stops.push(GradientStop { color: CssColor::Named("black".to_string()), position: Some(1.0) });
    }
    
    Some(Gradient::Linear { angle, stops })
}

fn parse_radial_gradient(val: &str) -> Option<Gradient> {
    let inner = val.strip_prefix("radial-gradient(")?.strip_suffix(')')?;
    
    let mut shape = "circle".to_string();
    let mut stops = Vec::new();
    
    // Parse shape
    if inner.contains("circle") {
        shape = "circle".to_string();
    } else if inner.contains("ellipse") {
        shape = "ellipse".to_string();
    }
    
    // Find the color stops (after shape specification)
    let remaining = if let Some(idx) = inner.find(',') {
        &inner[idx + 1..]
    } else {
        inner
    };
    
    parse_color_stops(remaining, &mut stops);
    
    if stops.is_empty() {
        stops.push(GradientStop { color: CssColor::Transparent, position: Some(0.0) });
        stops.push(GradientStop { color: CssColor::Named("black".to_string()), position: Some(1.0) });
    }
    
    Some(Gradient::Radial { shape, stops })
}

fn parse_color_stops(input: &str, stops: &mut Vec<GradientStop>) {
    let parts: Vec<&str> = input.split(',').collect();
    
    for (i, part) in parts.iter().enumerate() {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        
        let mut color = CssColor::Named("black".to_string());
        let mut position = None;
        
        // Split by space to find color and position
        let tokens: Vec<&str> = part.split_whitespace().collect();
        let has_tokens = !tokens.is_empty();
        
        for token in &tokens {
            if token.ends_with("%") {
                if let Ok(p) = token.strip_suffix("%").unwrap().parse::<f32>() {
                    position = Some(p / 100.0);
                }
            } else if !token.is_empty() {
                color = parse_color(token);
            }
        }
        
        // If no position specified, calculate based on index
        if position.is_none() && has_tokens {
            position = Some(i as f32 / parts.len() as f32);
        }
        
        stops.push(GradientStop { color, position });
    }
}

fn resolve_css_variables(val: &str, style: &ComputedStyle) -> String {
    // Start resolution with a recursion limit
    resolve_css_variables_recursive(val, style, 0)
}

fn resolve_css_variables_recursive(val: &str, style: &ComputedStyle, depth: usize) -> String {
    if depth > 16 { // Recursion limit
        return val.to_string();
    }

    let mut resolved = val.to_string();
    let mut changed = true;
    let mut iteration = 0;
    
    // Iterative replacement for current level, but recursive for nested vars if needed
    // Actually, simple iterative string replacement is still the easiest way to handle "var(--a, var(--b))" in one string
    // But we need to be careful.
    
    while let Some(start_idx) = resolved.find("var(") {
        iteration += 1;
        if iteration > 32 { break; } // Safety break for complex single-string nested vars

        let rest = &resolved[start_idx + 4..];
        let mut brace_count = 1;
        let mut end_idx = 0;
        for (i, c) in rest.chars().enumerate() {
            if c == '(' { brace_count += 1; }
            else if c == ')' { brace_count -= 1; }
            if brace_count == 0 {
                end_idx = i;
                break;
            }
        }
        
        if end_idx > 0 {
            let inner = rest[..end_idx].trim();
            // Handle fallback: var(--name, fallback)
            let mut parts = inner.splitn(2, ',');
            let var_name = parts.next().unwrap_or("").trim();
            let fallback = parts.next().map(|s| s.trim()).unwrap_or("");
            
            // Buscar valor da variável
            let replacement = style.custom_properties.get(var_name)
                .map(|v| resolve_css_variables_recursive(v.as_str(), style, depth + 1)) // Recurse here
                .unwrap_or_else(|| resolve_css_variables_recursive(fallback, style, depth + 1));
                
            let full_var = &resolved[start_idx..start_idx + 4 + end_idx + 1];
            resolved = resolved.replace(full_var, &replacement);
        } else {
            break;
        }
    }
    
    resolved
}

// Helper to resolve lengths to pixels
fn resolve_length(
    length: &CssLength,
    parent_font_size: f32,
    root_font_size: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> f32 {
    match length {
        CssLength::Px(v) => *v,
        CssLength::Em(v) => v * parent_font_size,
        CssLength::Rem(v) => v * root_font_size,
        CssLength::Percent(v) => v / 100.0 * parent_font_size, // Default to parent font size for font-size property, caller handles context
        CssLength::Vw(v) => v / 100.0 * viewport_width,
        CssLength::Vh(v) => v / 100.0 * viewport_height,
        // For others, return 0 or default
        _ => 0.0,
    }
}

fn parse_inline_declarations(style_str: &str) -> Vec<Declaration> {
    let mut decls = Vec::new();
    let parts: Vec<&str> = style_str.split(';').collect();
    for part in parts {
        let part = part.trim();
        if part.is_empty() { continue; }
        
        let subparts: Vec<&str> = part.splitn(2, ':').collect();
        if subparts.len() == 2 {
            decls.push(Declaration {
                name: subparts[0].trim().to_string(),
                value: subparts[1].trim().to_string(),
                important: false,
            });
        }
    }
    decls
}

pub fn apply_single_declaration(
    style: &mut ComputedStyle,
    decl: &Declaration,
    current_font_size: f32,
    root_font_size: f32,
    phase_1_only: bool,
) {
    let name = decl.name.as_str();
    let val_raw = decl.value.trim();
    
    if phase_1_only {
        if name == "font-size" {
            let val = if val_raw.contains("var(") {
                resolve_css_variables(val_raw, style)
            } else {
                val_raw.to_string()
            };
            
            let parsed = parse_length(&val);
            style.font_size = match parsed {
                CssLength::Px(v) => v,
                CssLength::Em(v) => v * current_font_size, 
                CssLength::Rem(v) => v * root_font_size,
                CssLength::Percent(v) => v / 100.0 * current_font_size,
                _ => style.font_size, 
            };
        }
        return;
    }

    if name == "font-size" { return; } 

    if name.starts_with("--") {
        style.custom_properties.insert(name.to_string(), val_raw.to_string());
        return;
    }

    let val_string = if val_raw.contains("var(") {
        resolve_css_variables(val_raw, style)
    } else {
        val_raw.to_string()
    };
    let val = val_string.as_str();

    let mut resolve_rel_recursive = |l: CssLength, f: &dyn Fn(CssLength) -> CssLength| -> CssLength {
        match l {
            CssLength::Em(v) => CssLength::Px(v * current_font_size),
            CssLength::Rem(v) => CssLength::Px(v * root_font_size),
            CssLength::Clamp(min, val, max) => CssLength::Clamp(
                Box::new(f(*min)),
                Box::new(f(*val)),
                Box::new(f(*max))
            ),
            CssLength::Min(vals) => CssLength::Min(vals.into_iter().map(|v| f(v)).collect()),
            CssLength::Max(vals) => CssLength::Max(vals.into_iter().map(|v| f(v)).collect()),
            _ => l
        }
    };

    // Fix: We need a way to call it recursively. Since closures can't easily recurse without help:
    fn resolve_rel_static(l: CssLength, current_font_size: f32, root_font_size: f32) -> CssLength {
        match l {
            CssLength::Em(v) => CssLength::Px(v * current_font_size),
            CssLength::Rem(v) => CssLength::Px(v * root_font_size),
            CssLength::Clamp(min, val, max) => CssLength::Clamp(
                Box::new(resolve_rel_static(*min, current_font_size, root_font_size)),
                Box::new(resolve_rel_static(*val, current_font_size, root_font_size)),
                Box::new(resolve_rel_static(*max, current_font_size, root_font_size))
            ),
            CssLength::Min(vals) => CssLength::Min(vals.into_iter().map(|v| resolve_rel_static(v, current_font_size, root_font_size)).collect()),
            CssLength::Max(vals) => CssLength::Max(vals.into_iter().map(|v| resolve_rel_static(v, current_font_size, root_font_size)).collect()),
            _ => l
        }
    }

    let resolve_rel = |l: CssLength| -> CssLength {
        resolve_rel_static(l, current_font_size, root_font_size)
    };

    match name {
        "background-color" | "background" => style.background_color = parse_color(val),
        "color" => style.color = parse_color(val),
        "display" => style.display = parse_display(val),
        "position" => style.position = parse_position(val),
        "overflow" => style.overflow = parse_overflow(val),
        "z-index" | "zIndex" => {
            if val == "auto" {
                style.z_index = i32::MIN;
            } else if let Ok(n) = val.parse::<i32>() {
                style.z_index = n;
            }
        },
        "width" => style.width = resolve_rel(parse_length(val)),
        "height" => style.height = resolve_rel(parse_length(val)),
        "top" => style.top = resolve_rel(parse_length(val)),
        "right" => style.right = resolve_rel(parse_length(val)),
        "bottom" => style.bottom = resolve_rel(parse_length(val)),
        "left" => style.left = resolve_rel(parse_length(val)),
        
        "margin-top" => style.margin_top = resolve_rel(parse_length(val)),
        "margin-right" => style.margin_right = resolve_rel(parse_length(val)),
        "margin-bottom" => style.margin_bottom = resolve_rel(parse_length(val)),
        "margin-left" => style.margin_left = resolve_rel(parse_length(val)),
        "margin" => {
            let parts: Vec<&str> = val.split_whitespace().collect();
            match parts.len() {
                1 => {
                    let m = resolve_rel(parse_length(parts[0]));
                    style.margin_top = m.clone();
                    style.margin_right = m.clone();
                    style.margin_bottom = m.clone();
                    style.margin_left = m;
                },
                2 => {
                    let v = resolve_rel(parse_length(parts[0]));
                    let h = resolve_rel(parse_length(parts[1]));
                    style.margin_top = v.clone();
                    style.margin_bottom = v;
                    style.margin_right = h.clone();
                    style.margin_left = h;
                },
                4 => {
                    style.margin_top = resolve_rel(parse_length(parts[0]));
                    style.margin_right = resolve_rel(parse_length(parts[1]));
                    style.margin_bottom = resolve_rel(parse_length(parts[2]));
                    style.margin_left = resolve_rel(parse_length(parts[3]));
                },
                _ => {}
            }
        },
        
        "padding-top" => style.padding_top = resolve_rel(parse_length(val)),
        "padding-right" => style.padding_right = resolve_rel(parse_length(val)),
        "padding-bottom" => style.padding_bottom = resolve_rel(parse_length(val)),
        "padding-left" => style.padding_left = resolve_rel(parse_length(val)),
        "padding" => {
            let parts: Vec<&str> = val.split_whitespace().collect();
            match parts.len() {
                1 => {
                    let p = resolve_rel(parse_length(parts[0]));
                    style.padding_top = p.clone();
                    style.padding_right = p.clone();
                    style.padding_bottom = p.clone();
                    style.padding_left = p;
                },
                2 => {
                    let v = resolve_rel(parse_length(parts[0]));
                    let h = resolve_rel(parse_length(parts[1]));
                    style.padding_top = v.clone();
                    style.padding_bottom = v;
                    style.padding_right = h.clone();
                    style.padding_left = h;
                },
                4 => {
                    style.padding_top = resolve_rel(parse_length(parts[0]));
                    style.padding_right = resolve_rel(parse_length(parts[1]));
                    style.padding_bottom = resolve_rel(parse_length(parts[2]));
                    style.padding_left = resolve_rel(parse_length(parts[3]));
                },
                _ => {}
            }
        },
        
        "border-radius" => style.border_radius_top_left = parse_border_radius(val),
        "box-shadow" => style.box_shadow = parse_box_shadow(val),
        "text-shadow" => style.text_shadow = parse_text_shadow(val),
        "background-image" => style.background_image = parse_background_image(val),
        "content" => style.content = parse_content(val),
        "aspect-ratio" => {
            if let Ok(ratio) = val.parse::<f32>() {
                style.aspect_ratio = Some(ratio);
            } else if val.contains('/') {
                let parts: Vec<&str> = val.split('/').collect();
                if parts.len() == 2 {
                    if let (Ok(w), Ok(h)) = (parts[0].trim().parse::<f32>(), parts[1].trim().parse::<f32>()) {
                        style.aspect_ratio = Some(w / h);
                    }
                }
            }
        },
        "box-sizing" | "boxSizing" => style.box_sizing = parse_box_sizing(val),
        "visibility" => style.visibility = parse_visibility(val),
        "cursor" => style.cursor = parse_cursor(val),
        "pointer-events" | "pointerEvents" => style.pointer_events = parse_pointer_events(val),
        "transform" => style.transform = parse_transform(val),
        "opacity" => {
            if let Ok(n) = val.parse::<f32>() {
                style.opacity = n.clamp(0.0, 1.0);
            }
        },
        "object-fit" | "objectFit" => style.object_fit = parse_object_fit(val),
        "object-position" | "objectPosition" => style.object_position = parse_object_position(val),
        "filter" => style.filters = parse_filters(val),
        "backdrop-filter" | "backdropFilter" => style.backdrop_filters = parse_filters(val),
        "mix-blend-mode" | "mixBlendMode" => style.mix_blend_mode = parse_blend_mode(val),
        "transition" => style.transitions = parse_transitions(val),
        "animation" => style.animations = parse_animations(val),
        "clip-path" | "clipPath" => style.clip_path = Some(val.trim().to_string()),
        
        // Flexbox & Grid Alignment
        "order" => {
            if let Ok(n) = val.parse::<i32>() {
                style.order = n;
            }
        },
        "align-self" | "alignSelf" => style.align_self = parse_align_items(val),
        "align-content" | "alignContent" => style.align_content = parse_align_content(val),
        
        // Grid Template
        "grid-template-columns" | "gridTemplateColumns" => style.grid_template_columns = parse_grid_track_list(val),
        "grid-template-rows" | "gridTemplateRows" => style.grid_template_rows = parse_grid_track_list(val),
        "grid-template-areas" | "gridTemplateAreas" => style.grid_template_areas = parse_grid_template_areas(val),
        "grid-column-gap" | "column-gap" => style.grid_column_gap = parse_length(val),
        "grid-row-gap" | "row-gap" => style.grid_row_gap = parse_length(val),
        "gap" => {
            let parts = split_spaces_top_level(val);
            if parts.len() == 1 {
                let gap = parse_length(parts[0]);
                style.grid_row_gap = gap.clone();
                style.grid_column_gap = gap;
            } else if parts.len() >= 2 {
                style.grid_row_gap = parse_length(parts[0]);
                style.grid_column_gap = parse_length(parts[1]);
            }
        },

        // Grid Item Properties
        "grid-column-start" | "gridColumnStart" => style.grid_column_start = parse_grid_placement(val),
        "grid-column-end" | "gridColumnEnd" => style.grid_column_end = parse_grid_placement(val),
        "grid-row-start" | "gridRowStart" => style.grid_row_start = parse_grid_placement(val),
        "grid-row-end" | "gridRowEnd" => style.grid_row_end = parse_grid_placement(val),
        
        "grid-column" | "gridColumn" => {
            let parts: Vec<&str> = val.split('/').collect();
            if parts.len() == 1 {
                style.grid_column_start = parse_grid_placement(parts[0]);
                style.grid_column_end = CssLength::Auto;
            } else if parts.len() >= 2 {
                style.grid_column_start = parse_grid_placement(parts[0]);
                style.grid_column_end = parse_grid_placement(parts[1]);
            }
        },
        "grid-row" | "gridRow" => {
            let parts: Vec<&str> = val.split('/').collect();
            if parts.len() == 1 {
                style.grid_row_start = parse_grid_placement(parts[0]);
                style.grid_row_end = CssLength::Auto;
            } else if parts.len() >= 2 {
                style.grid_row_start = parse_grid_placement(parts[0]);
                style.grid_row_end = parse_grid_placement(parts[1]);
            }
        },
        "grid-area" | "gridArea" => {
            let parts: Vec<&str> = val.split('/').collect();
            if parts.len() == 1 {
                // Could be an area name
                let name = parse_grid_placement(parts[0]);
                style.grid_row_start = name.clone();
                style.grid_column_start = name.clone();
                style.grid_row_end = name.clone();
                style.grid_column_end = name;
            } else if parts.len() >= 4 {
                style.grid_row_start = parse_grid_placement(parts[0]);
                style.grid_column_start = parse_grid_placement(parts[1]);
                style.grid_row_end = parse_grid_placement(parts[2]);
                style.grid_column_end = parse_grid_placement(parts[3]);
            }
        },
        
        _ => {}
    }
}

// Media Query Helpers
fn matches_media_query(query: &str, vw: f32, vh: f32, color_scheme: &str) -> bool {
    let query = query.trim();
    if query.is_empty() { return true; }
    
    // OR logic (comma separated)
    for part in query.split(',') {
        if matches_media_query_part(part, vw, vh, color_scheme) {
            return true;
        }
    }
    false
}

fn matches_media_query_part(part: &str, vw: f32, vh: f32, color_scheme: &str) -> bool {
    let normalized = part.to_lowercase();
    // Helper to handle "and" splitting safely would be better, but simple split acts as "good enough" for now
    let segments: Vec<&str> = normalized.split(" and ").collect();
    
    for (i, segment) in segments.iter().enumerate() {
        let seg = segment.trim();
        if i == 0 {
            // First segment might have modifier/type
            let mut s = seg;
            let mut negate = false;
            
            if s.starts_with("not ") {
                negate = true;
                s = &s[4..].trim();
            } else if s.starts_with("only ") {
                s = &s[5..].trim();
            }
            
            let seg_match = if s.starts_with('(') {
                 matches_feature(s, vw, vh, color_scheme)
            } else {
                 s == "screen" || s == "all"
            };
            
            if negate == seg_match { return false; }
        } else {
            // Subsequent segments are features
             if !matches_feature(seg, vw, vh, color_scheme) {
                 return false;
             }
        }
    }
    true
}

fn matches_feature(feature: &str, vw: f32, vh: f32, color_scheme: &str) -> bool {
    // Remove outer parens
    let f = feature.trim();
    let f = if f.starts_with('(') && f.ends_with(')') { &f[1..f.len()-1] } else { f };
    
    if let Some(idx) = f.find(':') {
        let name = f[..idx].trim();
        let val = f[idx+1..].trim();
        
        let get_px = |v: &str| -> f32 {
             if let Some(p) = v.strip_suffix("px") {
                 p.parse::<f32>().unwrap_or(0.0)
             } else { 0.0 }
        };

        match name {
            "min-width" => vw >= get_px(val),
            "max-width" => vw <= get_px(val),
            "min-height" => vh >= get_px(val),
            "max-height" => vh <= get_px(val),
            "orientation" => {
                 if val == "landscape" { vw >= vh }
                 else if val == "portrait" { vh >= vw }
                 else { false }
            },
            "prefers-color-scheme" => {
                val == color_scheme
            }
            _ => true 
        }
    } else {
        true 
    }
}

fn matches_supports(condition: &str) -> bool {
    let c = condition.trim();
    let c = if c.starts_with('(') && c.ends_with(')') { &c[1..c.len()-1] } else { c };
    if let Some(idx) = c.find(':') {
        let name = c[..idx].trim();
        match name {
            "display" | "color" | "background-color" | "width" | "height" | "position" | 
            "flex-direction" | "grid-template-columns" | "aspect-ratio" | "object-fit" | 
            "filter" | "backdrop-filter" | "mix-blend-mode" | "transition" | "animation" |
            "pointer-events" | "cursor" | "visibility" | "opacity" | "z-index" | "box-shadow" | "transform" => true,
            _ => false
        }
    } else { true }
}

fn matches_container(condition: &str, vw: f32) -> bool {
    // Basic stub: use vw for container size as a fallback
    let c = condition.trim();
    let c = if c.starts_with('(') && c.ends_with(')') { &c[1..c.len()-1] } else { c };
    if let Some(idx) = c.find(':') {
        let name = c[..idx].trim();
        let val = c[idx+1..].trim();
        let get_px = |v: &str| -> f32 {
             if let Some(p) = v.strip_suffix("px") { p.parse::<f32>().unwrap_or(0.0) } else { 0.0 }
        };
        match name {
            "min-width" => vw >= get_px(val),
            "max-width" => vw <= get_px(val),
            _ => true
        }
    } else { true }
}
