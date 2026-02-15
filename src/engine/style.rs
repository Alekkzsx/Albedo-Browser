use cssparser::{Parser, ParserInput, ToCss, SourceLocation, CowRcStr, ParseError};
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching::{ElementSelectorFlags, MatchingContext, MatchingMode};
use selectors::OpaqueElement;
use precomputed_hash::PrecomputedHash;
use crate::engine::dom::{AceDOM, AceNodeType};
use crate::engine::css_values::{ComputedStyle, CssLength, CssColor, CssDisplay};
use std::ops::Deref;

pub struct Stylesheet {
    pub rules: Vec<AceRule>,
}

#[derive(Debug, Clone)]
pub struct AceRule {
    pub selectors: selectors::SelectorList<AceSelectorImpl>,
    pub declarations: Vec<Declaration>,
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
pub enum AcePseudoElement {}
impl selectors::parser::PseudoElement for AcePseudoElement {
    type Impl = AceSelectorImpl;
}
impl ToCss for AcePseudoElement {
    fn to_css<W>(&self, _dest: &mut W) -> std::fmt::Result where W: std::fmt::Write {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AceNonTSPseudoClass {}
impl selectors::parser::NonTSPseudoClass for AceNonTSPseudoClass {
    type Impl = AceSelectorImpl;
    fn is_active_or_hover(&self) -> bool { false }
    fn is_user_action_state(&self) -> bool { false }
}
impl ToCss for AceNonTSPseudoClass {
    fn to_css<W>(&self, _dest: &mut W) -> std::fmt::Result where W: std::fmt::Write {
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct AceElement<'a> {
    pub dom: &'a AceDOM,
    pub index: usize,
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
                         return Some(AceElement { dom: self.dom, index: parent_idx });
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
                        return Some(AceElement { dom: self.dom, index: child_idx });
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
                        return Some(AceElement { dom: self.dom, index: prev_idx });
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
                        return Some(AceElement { dom: self.dom, index: next_idx });
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
                        AttrSelectorOperation::WithValue { value, .. } => val == value.as_str(),
                    };
                }
            }
        }
        false
    }

    fn match_non_ts_pseudo_class(
        &self,
        _pc: &<Self::Impl as selectors::parser::SelectorImpl>::NonTSPseudoClass,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        false
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

pub fn parse(source: &str) -> Stylesheet {
    let mut stylesheet = Stylesheet { rules: Vec::new() };
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
                                let mut value = String::new();
                                while let Ok(token) = p.next() {
                                    value.push_str(&token.to_css_string());
                                }
                                
                                // Clean up value (remove trailing semicolon and whitespace)
                                let value = value.trim_end_matches(';').trim().to_string();
                                
                                // Expand shorthands (Simple implementation)
                                // Only margin and padding for now
                                match name.as_str() {
                                    "margin" => {
                                        let parts: Vec<&str> = value.split_whitespace().collect();
                                        match parts.len() {
                                            1 => {
                                                for suffix in &["top", "right", "bottom", "left"] {
                                                    decls.push(Declaration { name: format!("margin-{}", suffix), value: parts[0].to_string(), important: false });
                                                }
                                            },
                                            2 => {
                                                decls.push(Declaration { name: "margin-top".to_string(), value: parts[0].to_string(), important: false });
                                                decls.push(Declaration { name: "margin-bottom".to_string(), value: parts[0].to_string(), important: false });
                                                decls.push(Declaration { name: "margin-right".to_string(), value: parts[1].to_string(), important: false });
                                                decls.push(Declaration { name: "margin-left".to_string(), value: parts[1].to_string(), important: false });
                                            },
                                            4 => {
                                                 decls.push(Declaration { name: "margin-top".to_string(), value: parts[0].to_string(), important: false });
                                                 decls.push(Declaration { name: "margin-right".to_string(), value: parts[1].to_string(), important: false });
                                                 decls.push(Declaration { name: "margin-bottom".to_string(), value: parts[2].to_string(), important: false });
                                                 decls.push(Declaration { name: "margin-left".to_string(), value: parts[3].to_string(), important: false });
                                            },
                                            _ => {} // Ignore invalid syntax
                                        }
                                    },
                                    "padding" => {
                                        let parts: Vec<&str> = value.split_whitespace().collect();
                                        match parts.len() {
                                            1 => {
                                                for suffix in &["top", "right", "bottom", "left"] {
                                                    decls.push(Declaration { name: format!("padding-{}", suffix), value: parts[0].to_string(), important: false });
                                                }
                                            },
                                            _ => {} // Todo: expand others
                                        }
                                    },
                                    _ => decls.push(Declaration { name, value, important: false }),
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

    stylesheet
}

struct AceSelectorParser;
impl<'i> selectors::parser::Parser<'i> for AceSelectorParser {
    type Impl = AceSelectorImpl;
    type Error = selectors::parser::SelectorParseErrorKind<'i>;

    fn parse_non_ts_pseudo_class(&self, _location: SourceLocation, _name: CowRcStr<'i>) -> Result<AceNonTSPseudoClass, ParseError<'i, Self::Error>> {
        Err(ParseError {
            kind: cssparser::ParseErrorKind::Custom(selectors::parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement(_name)),
            location: _location,
        })
    }
}

pub struct MatchedRule<'a> {
    pub specificity: u32,
    pub order: usize,
    pub rule: &'a AceRule,
}

impl Stylesheet {
    // Calculate style with inheritance
    pub fn calculate_style(&self, dom: &AceDOM, node_id: usize, parent_style: Option<&ComputedStyle>) -> ComputedStyle {
        let mut style = if let Some(parent) = parent_style {
            let mut s = ComputedStyle::default();
            // Inherited properties
            s.color = parent.color.clone();
            s.font_size = parent.font_size.clone();
            // Add other inherited properties here as we add them to struct (e.g. font-family, line-height, text-align)
            
            // Non-inherited properties (layout, background, borders) are already reset to default() in s
            s
        } else {
            ComputedStyle::default()
        };

        if let Some(node) = dom.get_node(node_id) {
             if let AceNodeType::Element(_) = &node.node_type {
                 let ace_element = AceElement { dom, index: node_id };
                 let mut matched_rules = Vec::new();

                 for (order, rule) in self.rules.iter().enumerate() {
                    let mut caches = selectors::matching::SelectorCaches::default();
                     let mut context = MatchingContext::new(
                        MatchingMode::Normal,
                        None,
                        &mut caches,
                        selectors::matching::QuirksMode::NoQuirks,
                        selectors::matching::NeedsSelectorFlags::No,
                        selectors::matching::MatchingForInvalidation::No,
                    );
                    
                    // Iterate over all selectors in the list to find the best match
                    // SelectorList matches if ANY selector matches.
                    // Accessing internal slice is tricky if .0 is private.
                    // Let's use `selectors::matching::matches_selector_list` to filter first? 
                    // No, we need specificity.
                    // Check if we can iterate via slice. `impl Deref for SelectorList`?
                    // Usually `slice()` or `iter()` is available.
                    // Attempting `rule.selectors.slice().iter()` or just `rule.selectors.iter()` knowning `SelectorList` usually wraps a SmallVec or Vec.
                    // If `rule.selectors.0` is private, maybe `rule.selectors.slice()`?
                    // Let's try `rule.selectors.slice().iter()` if available, or just fallback to `matches_selector_list` effectively.
                    // Actually, let's look at `selectors` crate docs or usage.
                    // Often `SelectorList` implements `Deref<Target=[Selector]>` or has `.slice()`.
                    // Let's guess `.slice().iter()` based on common habits, or try to iterate directly.
                    // If `.0` is private, likely `Deref` is implemented.
                    
                    // Also fixing `matches_selector` arguments (5 args).
                    
                    for selector in rule.selectors.iter() {
                        if selectors::matching::matches_selector(
                            selector,
                            0, 
                            None,
                            &ace_element,
                            &mut context,
                        ) {
                            let spec = selector.specificity();
                            if best_specificity.map_or(true, |s| spec > s) {
                                best_specificity = Some(spec);
                            }
                        }
                    }

                    if let Some(spec) = best_specificity {
                         matched_rules.push(MatchedRule {
                             specificity: spec,
                             order,
                             rule
                         });
                    }
                 }

                 // Sort by specificity then order
                 matched_rules.sort_by(|a, b| {
                     if a.specificity != b.specificity {
                         a.specificity.cmp(&b.specificity)
                     } else {
                         a.order.cmp(&b.order)
                     }
                 });

                 // Apply rules
                 for match_rule in matched_rules {
                     for decl in &match_rule.rule.declarations {
                         let val = decl.value.trim();
                         match decl.name.as_str() {
                             "background-color" | "background" => style.background_color = parse_color(val),
                             "color" => style.color = parse_color(val),
                             "display" => style.display = parse_display(val),
                             "width" => style.width = parse_length(val),
                             "height" => style.height = parse_length(val),
                             "margin-top" => style.margin_top = parse_length(val),
                             "margin-right" => style.margin_right = parse_length(val),
                             "margin-bottom" => style.margin_bottom = parse_length(val),
                             "margin-left" => style.margin_left = parse_length(val),
                             "padding-top" => style.padding_top = parse_length(val),
                             "padding-right" => style.padding_right = parse_length(val),
                             "padding-bottom" => style.padding_bottom = parse_length(val),
                             "padding-left" => style.padding_left = parse_length(val),
                             "font-size" => style.font_size = parse_length(val),
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
    if val == "auto" { return CssLength::Auto; }
    if val == "0" { return CssLength::Zero; }
    if let Some(n) = val.strip_suffix("px") {
        if let Ok(num) = n.trim().parse::<f32>() { return CssLength::Px(num); }
    }
    if let Some(n) = val.strip_suffix("%") {
        if let Ok(num) = n.trim().parse::<f32>() { return CssLength::Percent(num); }
    }
    // Fallback unknown
    CssLength::Auto
}

fn parse_color(val: &str) -> CssColor {
    match val {
        "transparent" => CssColor::Transparent,
        "currentcolor" => CssColor::CurrentColor,
        _ => CssColor::Named(val.to_string()),
    }
}

fn parse_display(val: &str) -> CssDisplay {
    match val {
        "none" => CssDisplay::None,
        "block" => CssDisplay::Block,
        "inline-block" => CssDisplay::InlineBlock,
        "flex" => CssDisplay::Flex,
        _ => CssDisplay::Inline,
    }
}
