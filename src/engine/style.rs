use cssparser::{Parser, ParserInput, ToCss, SourceLocation, CowRcStr, ParseError};
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching::{ElementSelectorFlags, MatchingContext, MatchingMode};
use selectors::OpaqueElement;
use precomputed_hash::PrecomputedHash;
use crate::engine::dom::{AceDOM, AceNodeType};
use crate::engine::css_values::{
    ComputedStyle, CssLength, CssColor, CssDisplay, CssTextAlign, CssFontWeight,
    CssPosition, CssOverflow, CssFloat, CssFlexDirection, CssJustifyContent,
    CssAlignItems, CssFlexWrap, BoxShadow, TextShadow, BackgroundImage, Gradient, GradientStop,
    CssContent
};

pub struct Stylesheet {
    pub user_agent_rules: Vec<AceRule>,
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
    pub hovered_element: Option<usize>,
    pub focused_element: Option<usize>,
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
                         return Some(AceElement { dom: self.dom, index: parent_idx, hovered_element: self.hovered_element, focused_element: self.focused_element });
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
                        return Some(AceElement { dom: self.dom, index: child_idx, hovered_element: self.hovered_element, focused_element: self.focused_element });
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
                        return Some(AceElement { dom: self.dom, index: prev_idx, hovered_element: self.hovered_element, focused_element: self.focused_element });
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
                        return Some(AceElement { dom: self.dom, index: next_idx, hovered_element: self.hovered_element, focused_element: self.focused_element });
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
        }
        button { background-color: #efefef; border: 1px solid #767676; padding: 1px 6px; }
        input[type=\"text\"] { background-color: white; border: 1px solid #767676; padding: 1px 2px; }
    ";
    
    let mut ss = parse_simple(ua_css);
    // Move rules to user_agent_rules
    let rules = std::mem::take(&mut ss.rules);
    ss.user_agent_rules = rules;
    ss
}

pub fn parse(source: &str) -> Stylesheet {
    let mut stylesheet = get_user_agent_stylesheet();
    
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
    let mut stylesheet = Stylesheet { user_agent_rules: Vec::new(), rules: Vec::new(), media_rules: Vec::new() };
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
    pub specificity: u32,
    pub order: usize,
}

pub struct MatchedRule<'a> {
    pub priority: CascadePriority,
    pub rule: &'a AceRule,
}

impl Stylesheet {
    // Calculate style with inheritance
    pub fn calculate_style(&self, dom: &AceDOM, node_id: usize, parent_style: Option<&ComputedStyle>, root_style: Option<&ComputedStyle>, hovered_element: Option<usize>, focused_element: Option<usize>) -> ComputedStyle {
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
                 let ace_element = AceElement { dom, index: node_id, hovered_element, focused_element };
                 let mut matched_rules = Vec::new();

                   // 1. Process User Agent Rules (Lowest priority)
                   for (order, rule) in self.user_agent_rules.iter().enumerate() {
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
                                       origin: CascadeOrigin::UserAgent,
                                       specificity: selector.specificity(),
                                       order,
                                   },
                                   rule
                               });
                           }
                       }
                   }

                   // 2. Process User Rules
                   for (order, rule) in self.rules.iter().enumerate() {
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
                                       origin: CascadeOrigin::Author,
                                       specificity: selector.specificity(),
                                       order: order + 1000,
                                   },
                                   rule
                               });
                           }
                       }
                   }

                  // 3. Process Media Rules
                  for media_rule in &self.media_rules {
                      let query = &media_rule.media_query;
                      let should_apply = query.is_empty() || 
                          query.contains("all") || 
                          query.contains("screen") ||
                          query.contains("print");
                                            if should_apply {
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

                  // Sort by priority (origin -> specificity -> order)
                  matched_rules.sort_by(|a, b| {
                      a.priority.cmp(&b.priority)
                  });

                  let parent_font_size = parent_style.map(|s| s.font_size).unwrap_or(16.0);
                  let root_font_size = root_style.map(|s| s.font_size).unwrap_or(16.0);

                  // Phase 1: Resolve font-size (because em in other properties depends on it)
                  for match_rule in &matched_rules {
                      for decl in &match_rule.rule.declarations {
                          if decl.name == "font-size" {
                              let val_raw = decl.value.trim();
                              let val = if val_raw.contains("var(") {
                                  resolve_css_variables(val_raw, &style)
                              } else {
                                  val_raw.to_string()
                              };
                              
                              let parsed = parse_length(&val);
                              style.font_size = match parsed {
                                  CssLength::Px(v) => v,
                                  CssLength::Em(v) => v * parent_font_size,
                                  CssLength::Rem(v) => v * root_font_size,
                                  CssLength::Percent(v) => v / 100.0 * parent_font_size,
                                  _ => 16.0,
                              };
                          }
                      }
                  }

                  let current_font_size = style.font_size;

                  // Phase 2: Apply all rules
                  for match_rule in matched_rules {
                      for decl in &match_rule.rule.declarations {
                          let name = decl.name.as_str();
                          let val_raw = decl.value.trim();
                          
                          if name == "font-size" { continue; } // Already handled

                          // Se for uma variável CSS (--name), armazena no HashMap
                          if name.starts_with("--") {
                              style.custom_properties.insert(name.to_string(), val_raw.to_string());
                              continue;
                          }

                          // Lógica de Resolução de var()
                          let val = if val_raw.contains("var(") {
                              resolve_css_variables(val_raw, &style)
                          } else {
                              val_raw.to_string()
                          };
                          let val = val.as_str();

                          // Helper to resolve relative lengths to Px immediately
                          let resolve_rel = |l: CssLength| -> CssLength {
                              match l {
                                  CssLength::Em(v) => CssLength::Px(v * current_font_size),
                                  CssLength::Rem(v) => CssLength::Px(v * root_font_size),
                                  _ => l
                              }
                          };

                          match name {
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
                              "width" => style.width = resolve_rel(parse_length(val)),
                              "height" => style.height = resolve_rel(parse_length(val)),
                              "min-width" => style.min_width = resolve_rel(parse_length(val)),
                              "max-width" => style.max_width = resolve_rel(parse_length(val)),
                              "min-height" => style.min_height = resolve_rel(parse_length(val)),
                              "max-height" => style.max_height = resolve_rel(parse_length(val)),
                              
                              // Position offsets
                              "top" => style.top = resolve_rel(parse_length(val)),
                              "right" => style.right = resolve_rel(parse_length(val)),
                              "bottom" => style.bottom = resolve_rel(parse_length(val)),
                              "left" => style.left = resolve_rel(parse_length(val)),
                              
                              // Margins
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
                                          style.margin_top = resolve_rel(parse_length(parts[0]));
                                          style.margin_bottom = resolve_rel(parse_length(parts[0]));
                                          style.margin_right = resolve_rel(parse_length(parts[1]));
                                          style.margin_left = resolve_rel(parse_length(parts[1]));
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
                              
                              // Padding
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
                                         style.padding_top = resolve_rel(parse_length(parts[0]));
                                         style.padding_bottom = resolve_rel(parse_length(parts[0]));
                                         style.padding_right = resolve_rel(parse_length(parts[1]));
                                         style.padding_left = resolve_rel(parse_length(parts[1]));
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
                             
                             // Typography
                            // Typography
                             // "font-size" handled in Phase 1
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
                              
                              // Grid Layout
                              "grid-template-columns" => style.grid_template_columns = parse_length_list(val).into_iter().map(resolve_rel).collect(),
                              "grid-template-rows" => style.grid_template_rows = parse_length_list(val).into_iter().map(resolve_rel).collect(),
                              "grid-column-start" => style.grid_column_start = resolve_rel(parse_length(val)),
                              "grid-column-end" => style.grid_column_end = resolve_rel(parse_length(val)),
                              "grid-row-start" => style.grid_row_start = resolve_rel(parse_length(val)),
                              "grid-row-end" => style.grid_row_end = resolve_rel(parse_length(val)),
                              "grid-column-gap" => style.grid_column_gap = resolve_rel(parse_length(val)),
                              "grid-row-gap" => style.grid_row_gap = resolve_rel(parse_length(val)),
                              "grid-gap" | "gap" => {
                                  let parts: Vec<&str> = val.split_whitespace().collect();
                                  let g = resolve_rel(parse_length(parts[0]));
                                  style.grid_column_gap = g.clone();
                                  style.grid_row_gap = g;
                                  if parts.len() > 1 {
                                      style.grid_row_gap = resolve_rel(parse_length(parts[1]));
                                  }
                              },
                              "grid-column" => {
                                  let parts: Vec<&str> = val.split('/').collect();
                                  if !parts.is_empty() {
                                      style.grid_column_start = resolve_rel(parse_length(parts[0].trim()));
                                  }
                                  if parts.len() > 1 {
                                      style.grid_column_end = resolve_rel(parse_length(parts[1].trim()));
                                  }
                              },
                              "grid-row" => {
                                  let parts: Vec<&str> = val.split('/').collect();
                                  if !parts.is_empty() {
                                      style.grid_row_start = resolve_rel(parse_length(parts[0].trim()));
                                  }
                                  if parts.len() > 1 {
                                      style.grid_row_end = resolve_rel(parse_length(parts[1].trim()));
                                  }
                              },
                             
                             // Visual
                             "opacity" => {
                                 if let Ok(n) = val.parse::<f32>() { style.opacity = n.clamp(0.0, 1.0); }
                             },
                             
                             // Pseudo-element content
                             "content" => {
                                 style.content = parse_content(val);
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
                             
                             // MELHORIA: Box-shadow
                             "box-shadow" => style.box_shadow = parse_box_shadow(val),
                             
                             // MELHORIA: Text-shadow
                             "text-shadow" => style.text_shadow = parse_text_shadow(val),
                             
                             // MELHORIA: Background-image (gradients)
                             "background-image" => style.background_image = parse_background_image(val),
                             
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

    pub fn calculate_pseudo_style(&self, dom: &AceDOM, node_id: usize, pseudo: &AcePseudoElement, hovered_element: Option<usize>, focused_element: Option<usize>) -> ComputedStyle {
        let mut style = ComputedStyle::default();
        
        // Default display for pseudo-elements is inline
        style.display = CssDisplay::Inline;
        
        if let Some(node) = dom.get_node(node_id) {
             if let AceNodeType::Element(_) = &node.node_type {
                 let ace_element = AceElement { dom, index: node_id, hovered_element, focused_element };
                 let mut matched_rules = Vec::new();

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
                                     priority: CascadePriority { origin: CascadeOrigin::UserAgent, specificity: selector.specificity(), order },
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
                                     priority: CascadePriority { origin: CascadeOrigin::Author, specificity: selector.specificity(), order: order + 1000 },
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
                             "font-size" => style.font_size = resolve_length(&parse_length(val), 16.0, 16.0, 1024.0, 768.0),
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
    if let Some(n) = val.strip_suffix("fr") {
        if let Ok(num) = n.trim().parse::<f32>() { return CssLength::Fr(num); }
    }
    // Fallback unknown
    CssLength::Auto
}

fn parse_length_list(val: &str) -> Vec<CssLength> {
    val.split_whitespace().map(parse_length).collect()
}

fn parse_content(val: &str) -> CssContent {
    use crate::engine::css_values::CssContent;
    
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
    let mut resolved = val.to_string();
    
    while let Some(start_idx) = resolved.find("var(") {
        let rest = &resolved[start_idx + 4..];
        if let Some(end_idx) = rest.find(')') {
            let var_name = rest[..end_idx].trim();
            // Buscar valor da variável
            let var_value = style.custom_properties.get(var_name)
                .map(|v| v.as_str())
                .unwrap_or("");
                
            let full_var = &resolved[start_idx..start_idx + 4 + end_idx + 1];
            resolved = resolved.replace(full_var, var_value);
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
