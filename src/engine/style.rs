use cssparser::{Parser, ParserInput, ToCss, SourceLocation, CowRcStr, ParseError};
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching::{ElementSelectorFlags, MatchingContext, MatchingMode};
use selectors::OpaqueElement;
use precomputed_hash::PrecomputedHash;
use crate::engine::dom::{AceDOM, AceNodeType};
use crate::engine::css_values::{
    ComputedStyle, CssLength, CssColor, CssDisplay, CssTextAlign, CssFontWeight,
    CssPosition, CssOverflow, CssFloat, CssFlexDirection, CssJustifyContent,
    CssAlignItems, CssFlexWrap, BoxShadow,
};

pub struct Stylesheet {
    pub rules: Vec<AceRule>,
    pub media_rules: Vec<AceMediaRule>,
}

#[derive(Debug, Clone)]
pub struct AceMediaRule {
    pub media_query: String,
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
pub enum AceNonTSPseudoClass {
    Hover,
    Focus,
    Active,
    FirstChild,
    LastChild,
    NthChild(i32), // positive for "nth", 0 for "even", -1 for "odd"
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
            AceNonTSPseudoClass::NthChild(_) => "nth-child",
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
        pc: &<Self::Impl as selectors::parser::SelectorImpl>::NonTSPseudoClass,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        // FASE 5: Implement pseudo-class matching
        match pc {
            AceNonTSPseudoClass::Hover => {
                // For now, we can't track hover state without JS interop
                // Return false - would need event system integration
                false
            },
            AceNonTSPseudoClass::Focus => {
                // Would need focus tracking
                false
            },
            AceNonTSPseudoClass::Active => {
                // Would need active state tracking
                false
            },
            AceNonTSPseudoClass::FirstChild => {
                self.prev_sibling_element().is_none()
            },
            AceNonTSPseudoClass::LastChild => {
                self.next_sibling_element().is_none()
            },
            AceNonTSPseudoClass::NthChild(_) => {
                // For NthChild, count position among siblings
                // This is simplified - full implementation would handle (an+b) formulas
                let mut count = 0;
                let mut current = self.prev_sibling_element();
                while current.is_some() {
                    count += 1;
                    current = current.unwrap().prev_sibling_element();
                }
                // Match first child for now (simplified)
                count == 0
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

pub fn parse(source: &str) -> Stylesheet {
    let mut stylesheet = Stylesheet { rules: Vec::new(), media_rules: Vec::new() };
    
    // FASE 5: Simple @media query support - find @media blocks
    // Extract @media blocks and parse them separately
    let mut remaining_source = source.to_string();
    
    // Find and extract @media blocks
    while let Some(media_start) = remaining_source.find("@media") {
        // Get the part before @media
        let before_media = &remaining_source[..media_start];
        // Parse regular rules from before part
        if !before_media.trim().is_empty() {
            let before_stylesheet = parse_simple(before_media);
            stylesheet.rules.extend(before_stylesheet.rules);
        }
        
        // Find the @media block
        let rest = &remaining_source[media_start..];
        if let Some(brace_start) = rest.find('{') {
            let mut brace_count = 1;
            let mut block_end = brace_start + 1;
            while block_end < rest.len() && brace_count > 0 {
                if rest[block_end..].starts_with('{') {
                    brace_count += 1;
                    block_end += 1;
                } else if rest[block_end..].starts_with('}') {
                    brace_count -= 1;
                    if brace_count == 0 {
                        break;
                    }
                    block_end += 1;
                } else {
                    block_end += 1;
                }
            }
            
            // Extract @media query and content
            let media_query_and_content = &rest[..block_end + 1];
            if let Some(query_end) = media_query_and_content.find('{') {
                let media_query = media_query_and_content[6..query_end].trim(); // Skip "@media"
                let media_content = &media_query_and_content[query_end + 1..block_end];
                
                println!("ENGINE: Parsing @media {} ", media_query);
                
                // Parse media content
                let media_stylesheet = parse_simple(media_content);
                stylesheet.media_rules.push(AceMediaRule {
                    media_query: media_query.to_string(),
                    rules: media_stylesheet.rules,
                });
            }
            
            remaining_source = if block_end + 1 < rest.len() {
                rest[block_end + 1..].to_string()
            } else {
                String::new()
            };
        } else {
            break;
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
    let mut stylesheet = Stylesheet { rules: Vec::new(), media_rules: Vec::new() };
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

    fn parse_non_ts_pseudo_class(&self, location: SourceLocation, name: CowRcStr<'i>) -> Result<AceNonTSPseudoClass, ParseError<'i, Self::Error>> {
        let name_str = name.as_ref();
        match name_str {
            "hover" => Ok(AceNonTSPseudoClass::Hover),
            "focus" => Ok(AceNonTSPseudoClass::Focus),
            "active" => Ok(AceNonTSPseudoClass::Active),
            "first-child" => Ok(AceNonTSPseudoClass::FirstChild),
            "last-child" => Ok(AceNonTSPseudoClass::LastChild),
            "nth-child" => {
                // For now, accept nth-child without argument - default to all children
                // A full implementation would parse (an+b) syntax
                Ok(AceNonTSPseudoClass::NthChild(1))
            },
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
            s.font_family = parent.font_family.clone();
            s.font_weight = parent.font_weight.clone();
            s.text_align = parent.text_align.clone();
            s.line_height = parent.line_height.clone();
            s.opacity = parent.opacity;
            // Inherit custom properties
            s.custom_properties = parent.custom_properties.clone();
            
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
                    
                    let mut best_specificity: Option<u32> = None;
                    
                    // Use matches_selector_list to check if any selector matches
                    if selectors::matching::matches_selector_list(
                        &rule.selectors,
                        &ace_element,
                        &mut context,
                    ) {
                        // If any selector matches, get the highest specificity
                        // Since we can't iterate easily, just use order as specificity proxy
                        best_specificity = Some(order as u32);
                    }

                    if let Some(spec) = best_specificity {
                         matched_rules.push(MatchedRule {
                             specificity: spec,
                             order,
                             rule
                         });
                    }
                 }

                 // FASE 5: Apply media rules (simplified - apply all for now)
                 // A full implementation would evaluate media query conditions against viewport
                 for media_rule in &self.media_rules {
                     // Simple media query evaluation - check if query matches
                     let query = &media_rule.media_query;
                     let should_apply = query.is_empty() || 
                         query.contains("all") || 
                         query.contains("screen") ||
                         query.contains("print");
                     
                     if should_apply {
                         for (order, rule) in media_rule.rules.iter().enumerate() {
                             let mut caches = selectors::matching::SelectorCaches::default();
                             let mut context = MatchingContext::new(
                                 MatchingMode::Normal,
                                 None,
                                 &mut caches,
                                 selectors::matching::QuirksMode::NoQuirks,
                                 selectors::matching::NeedsSelectorFlags::No,
                                 selectors::matching::MatchingForInvalidation::No,
                             );

                             if selectors::matching::matches_selector_list(&rule.selectors, &ace_element, &mut context) {
                                 matched_rules.push(MatchedRule {
                                     specificity: order as u32 + 1000, // Higher specificity for media rules
                                     order: order + 1000,
                                     rule
                                 });
                             }
                         }
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
                             // Display & Layout
                             "background-color" | "background" => style.background_color = parse_color(val),
                             "color" => style.color = parse_color(val),
                             "display" => style.display = parse_display(val),
                             
                             // Position & Layout
                             "position" => style.position = parse_position(val),
                             "overflow" => style.overflow = parse_overflow(val),
                             "z-index" | "zIndex" => {
                                 if val == "auto" {
                                     style.z_index = 0;
                                 } else if let Ok(n) = val.parse::<i32>() {
                                     style.z_index = n;
                                 }
                             },
                             "float" => style.float = parse_float(val),
                             
                             // Dimensions
                             "width" => style.width = parse_length(val),
                             "height" => style.height = parse_length(val),
                             "min-width" => style.min_width = parse_length(val),
                             "max-width" => style.max_width = parse_length(val),
                             "min-height" => style.min_height = parse_length(val),
                             "max-height" => style.max_height = parse_length(val),
                             
                             // Position offsets
                             "top" => style.top = parse_length(val),
                             "right" => style.right = parse_length(val),
                             "bottom" => style.bottom = parse_length(val),
                             "left" => style.left = parse_length(val),
                             
                             // Margins
                             "margin-top" => style.margin_top = parse_length(val),
                             "margin-right" => style.margin_right = parse_length(val),
                             "margin-bottom" => style.margin_bottom = parse_length(val),
                             "margin-left" => style.margin_left = parse_length(val),
                             "margin" => {
                                 let parts: Vec<&str> = val.split_whitespace().collect();
                                 match parts.len() {
                                     1 => {
                                         let m = parse_length(parts[0]);
                                         style.margin_top = m.clone();
                                         style.margin_right = m.clone();
                                         style.margin_bottom = m.clone();
                                         style.margin_left = m;
                                     },
                                     2 => {
                                         style.margin_top = parse_length(parts[0]);
                                         style.margin_bottom = parse_length(parts[0]);
                                         style.margin_right = parse_length(parts[1]);
                                         style.margin_left = parse_length(parts[1]);
                                     },
                                     4 => {
                                         style.margin_top = parse_length(parts[0]);
                                         style.margin_right = parse_length(parts[1]);
                                         style.margin_bottom = parse_length(parts[2]);
                                         style.margin_left = parse_length(parts[3]);
                                     },
                                     _ => {}
                                 }
                             },
                             
                             // Padding
                             "padding-top" => style.padding_top = parse_length(val),
                             "padding-right" => style.padding_right = parse_length(val),
                             "padding-bottom" => style.padding_bottom = parse_length(val),
                             "padding-left" => style.padding_left = parse_length(val),
                             "padding" => {
                                 let parts: Vec<&str> = val.split_whitespace().collect();
                                 match parts.len() {
                                     1 => {
                                         let p = parse_length(parts[0]);
                                         style.padding_top = p.clone();
                                         style.padding_right = p.clone();
                                         style.padding_bottom = p.clone();
                                         style.padding_left = p;
                                     },
                                     2 => {
                                         style.padding_top = parse_length(parts[0]);
                                         style.padding_bottom = parse_length(parts[0]);
                                         style.padding_right = parse_length(parts[1]);
                                         style.padding_left = parse_length(parts[1]);
                                     },
                                     4 => {
                                         style.padding_top = parse_length(parts[0]);
                                         style.padding_right = parse_length(parts[1]);
                                         style.padding_bottom = parse_length(parts[2]);
                                         style.padding_left = parse_length(parts[3]);
                                     },
                                     _ => {}
                                 }
                             },
                             
                             // Typography
                             "font-size" => style.font_size = parse_length(val),
                             "font-family" => style.font_family = val.to_string(),
                             "font-weight" => style.font_weight = parse_font_weight(val),
                             "text-align" => style.text_align = parse_text_align(val),
                             "line-height" => style.line_height = parse_length(val),
                             
                             // Flexbox
                             "flex-direction" => style.flex_direction = parse_flex_direction(val),
                             "justify-content" => style.justify_content = parse_justify_content(val),
                             "align-items" => style.align_items = parse_align_items(val),
                             "flex-wrap" => style.flex_wrap = parse_flex_wrap(val),
                             "flex-grow" => {
                                 if let Ok(n) = val.parse::<f32>() { style.flex_grow = n; }
                             },
                             "flex-shrink" => {
                                 if let Ok(n) = val.parse::<f32>() { style.flex_shrink = n; }
                             },
                             "flex-basis" => style.flex_basis = parse_length(val),
                             "flex" => {
                                 // Shorthand: flex-grow flex-shrink flex-basis
                                 let parts: Vec<&str> = val.split_whitespace().collect();
                                 if !parts.is_empty() {
                                     if let Ok(n) = parts[0].parse::<f32>() { style.flex_grow = n; }
                                 }
                                 if parts.len() > 1 {
                                     if let Ok(n) = parts[1].parse::<f32>() { style.flex_shrink = n; }
                                 }
                                 if parts.len() > 2 {
                                     style.flex_basis = parse_length(parts[2]);
                                 }
                             },
                             
                             // Visual
                             "opacity" => {
                                 if let Ok(n) = val.parse::<f32>() { style.opacity = n.clamp(0.0, 1.0); }
                             },
                             "border-radius" => {
                                 let parts: Vec<&str> = val.split_whitespace().collect();
                                 let mut has_width = false;
                                 let mut has_color = false;
                                 for part in parts {
                                     if part.ends_with("px") && !has_width {
                                         let w = parse_length(part);
                                         style.border_width_top = w.clone();
                                         style.border_width_right = w.clone();
                                         style.border_width_bottom = w.clone();
                                         style.border_width_left = w;
                                         has_width = true;
                                     } else if !part.ends_with("px") && !has_color {
                                         // Assume it's a color if not a px value
                                         let c = parse_color(part);
                                         style.border_color_top = c.clone();
                                         style.border_color_right = c.clone();
                                         style.border_color_bottom = c.clone();
                                         style.border_color_left = c;
                                         has_color = true;
                                     }
                                 }
                             },
                             "border-width" => {
                                 let w = parse_length(val);
                                 style.border_width_top = w.clone();
                                 style.border_width_right = w.clone();
                                 style.border_width_bottom = w.clone();
                                 style.border_width_left = w;
                             },
                             "border-color" => {
                                 let c = parse_color(val);
                                 style.border_color_top = c.clone();
                                 style.border_color_right = c.clone();
                                 style.border_color_bottom = c.clone();
                                 style.border_color_left = c;
                             },
                             "border-top-width" => style.border_width_top = parse_length(val),
                             "border-right-width" => style.border_width_right = parse_length(val),
                             "border-bottom-width" => style.border_width_bottom = parse_length(val),
                             "border-left-width" => style.border_width_left = parse_length(val),
                             "border-top-color" => style.border_color_top = parse_color(val),
                             "border-right-color" => style.border_color_right = parse_color(val),
                             "border-bottom-color" => style.border_color_bottom = parse_color(val),
                             "border-left-color" => style.border_color_left = parse_color(val),
                             "border-top-left-radius" => style.border_radius_top_left = parse_border_radius(val),
                             "border-top-right-radius" => style.border_radius_top_right = parse_border_radius(val),
                             "border-bottom-right-radius" => style.border_radius_bottom_right = parse_border_radius(val),
                             "border-bottom-left-radius" => style.border_radius_bottom_left = parse_border_radius(val),
                             
                             // CSS Custom Properties (variables)
                             name if name.starts_with("--") => {
                                 style.custom_properties.insert(name.to_string(), val.to_string());
                             },
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
    
    // Handle calc() - simplified: just try to extract a number
    if val.starts_with("calc(") {
        // Simple calc() support - just try to find a px value
        if let Some(px_idx) = val.find("px") {
            let start = px_idx.saturating_sub(10);
            let num_str: String = val[start..px_idx].chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(num) = num_str.parse::<f32>() {
                return CssLength::Px(num);
            }
        }
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
    // Fallback unknown
    CssLength::Auto
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
        _ => CssAlignItems::Stretch,
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
