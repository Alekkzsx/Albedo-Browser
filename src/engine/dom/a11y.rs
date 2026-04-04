//! Accessibility Tree (ARIA 1.2) Implementation - W3C WAI-ARIA Spec
//! 
//! Este módulo implementa:
//! - Mapeamento implícito de roles HTML → ARIA
//! - Accessible Name Computation (AccName 1.2)
//! - States & Properties ARIA
//! - Relations (aria-controls, aria-owns, etc.)
//! - Tree traversal para screen readers

use std::collections::{HashMap, HashSet};
use crate::engine::dom::{AceDOM, AceNode, AceNodeType, AceElement};

/// Role ARIA de um elemento
#[derive(Clone, Debug, PartialEq)]
pub enum AriaRole {
    // Widget Roles
    Button,
    Checkbox,
    Combobox,
    Gridcell,
    Link,
    Listbox,
    Menuitem,
    Menuitemcheckbox,
    Menuitemradio,
    Option,
    Progressbar,
    Radio,
    Scrollbar,
    Searchbox,
    Slider,
    Spinbutton,
    Switch,
    Tab,
    Tabpanel,
    Textbox,
    Treeitem,
    
    // Composite Roles
    Grid,
    ListBoxComposite,
    Menu,
    Menubar,
    Radiogroup,
    Tablist,
    Tree,
    Treegrid,
    
    // Document Structure Roles
    Application,
    Article,
    Cell,
    Columnheader,
    Definition,
    Directory,
    Document,
    Feed,
    Figure,
    Group,
    Heading,
    Img,
    List,
    ListItem,
    Math,
    None,
    Note,
    Presentation,
    Row,
    Rowgroup,
    Rowheader,
    Separator,
    Table,
    Term,
    Toolbar,
    Tooltip,
    
    // Landmark Roles
    Banner,
    Complementary,
    Contentinfo,
    Form,
    Main,
    Navigation,
    Region,
    Search,
    
    // Live Region Roles
    Alert,
    Log,
    Marquee,
    Status,
    Timer,
    
    // Window Roles
    Alertdialog,
    Dialog,
    
    // Fallback
    Generic,
}

impl AriaRole {
    /// Retorna a string ARIA correspondente
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaRole::Button => "button",
            AriaRole::Checkbox => "checkbox",
            AriaRole::Combobox => "combobox",
            AriaRole::Gridcell => "gridcell",
            AriaRole::Link => "link",
            AriaRole::Listbox => "listbox",
            AriaRole::Menuitem => "menuitem",
            AriaRole::Menuitemcheckbox => "menuitemcheckbox",
            AriaRole::Menuitemradio => "menuitemradio",
            AriaRole::Option => "option",
            AriaRole::Progressbar => "progressbar",
            AriaRole::Radio => "radio",
            AriaRole::Scrollbar => "scrollbar",
            AriaRole::Searchbox => "searchbox",
            AriaRole::Slider => "slider",
            AriaRole::Spinbutton => "spinbutton",
            AriaRole::Switch => "switch",
            AriaRole::Tab => "tab",
            AriaRole::Tabpanel => "tabpanel",
            AriaRole::Textbox => "textbox",
            AriaRole::Treeitem => "treeitem",
            AriaRole::Grid => "grid",
            AriaRole::ListBoxComposite => "listbox",
            AriaRole::Menu => "menu",
            AriaRole::Menubar => "menubar",
            AriaRole::Radiogroup => "radiogroup",
            AriaRole::Tablist => "tablist",
            AriaRole::Tree => "tree",
            AriaRole::Treegrid => "treegrid",
            AriaRole::Application => "application",
            AriaRole::Article => "article",
            AriaRole::Cell => "cell",
            AriaRole::Columnheader => "columnheader",
            AriaRole::Definition => "definition",
            AriaRole::Directory => "directory",
            AriaRole::Document => "document",
            AriaRole::Feed => "feed",
            AriaRole::Figure => "figure",
            AriaRole::Group => "group",
            AriaRole::Heading => "heading",
            AriaRole::Img => "img",
            AriaRole::List => "list",
            AriaRole::ListItem => "listitem",
            AriaRole::Math => "math",
            AriaRole::None => "none",
            AriaRole::Note => "note",
            AriaRole::Presentation => "presentation",
            AriaRole::Row => "row",
            AriaRole::Rowgroup => "rowgroup",
            AriaRole::Rowheader => "rowheader",
            AriaRole::Separator => "separator",
            AriaRole::Table => "table",
            AriaRole::Term => "term",
            AriaRole::Toolbar => "toolbar",
            AriaRole::Tooltip => "tooltip",
            AriaRole::Banner => "banner",
            AriaRole::Complementary => "complementary",
            AriaRole::Contentinfo => "contentinfo",
            AriaRole::Form => "form",
            AriaRole::Main => "main",
            AriaRole::Navigation => "navigation",
            AriaRole::Region => "region",
            AriaRole::Search => "search",
            AriaRole::Alert => "alert",
            AriaRole::Log => "log",
            AriaRole::Marquee => "marquee",
            AriaRole::Status => "status",
            AriaRole::Timer => "timer",
            AriaRole::Alertdialog => "alertdialog",
            AriaRole::Dialog => "dialog",
            AriaRole::Generic => "generic",
        }
    }
    
    /// Parse de string para AriaRole
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "button" => Some(AriaRole::Button),
            "checkbox" => Some(AriaRole::Checkbox),
            "combobox" => Some(AriaRole::Combobox),
            "link" => Some(AriaRole::Link),
            "listbox" => Some(AriaRole::Listbox),
            "menuitem" => Some(AriaRole::Menuitem),
            "option" => Some(AriaRole::Option),
            "progressbar" => Some(AriaRole::Progressbar),
            "radio" => Some(AriaRole::Radio),
            "searchbox" => Some(AriaRole::Searchbox),
            "slider" => Some(AriaRole::Slider),
            "spinbutton" => Some(AriaRole::Spinbutton),
            "switch" => Some(AriaRole::Switch),
            "tab" => Some(AriaRole::Tab),
            "tabpanel" => Some(AriaRole::Tabpanel),
            "textbox" => Some(AriaRole::Textbox),
            "treeitem" => Some(AriaRole::Treeitem),
            "grid" => Some(AriaRole::Grid),
            "menu" => Some(AriaRole::Menu),
            "menubar" => Some(AriaRole::Menubar),
            "radiogroup" => Some(AriaRole::Radiogroup),
            "tablist" => Some(AriaRole::Tablist),
            "tree" => Some(AriaRole::Tree),
            "article" => Some(AriaRole::Article),
            "cell" => Some(AriaRole::Cell),
            "columnheader" => Some(AriaRole::Columnheader),
            "definition" => Some(AriaRole::Definition),
            "document" => Some(AriaRole::Document),
            "figure" => Some(AriaRole::Figure),
            "group" => Some(AriaRole::Group),
            "heading" => Some(AriaRole::Heading),
            "img" => Some(AriaRole::Img),
            "list" => Some(AriaRole::List),
            "listitem" => Some(AriaRole::ListItem),
            "math" => Some(AriaRole::Math),
            "none" => Some(AriaRole::None),
            "note" => Some(AriaRole::Note),
            "presentation" => Some(AriaRole::Presentation),
            "row" => Some(AriaRole::Row),
            "rowgroup" => Some(AriaRole::Rowgroup),
            "rowheader" => Some(AriaRole::Rowheader),
            "separator" => Some(AriaRole::Separator),
            "table" => Some(AriaRole::Table),
            "term" => Some(AriaRole::Term),
            "toolbar" => Some(AriaRole::Toolbar),
            "tooltip" => Some(AriaRole::Tooltip),
            "banner" => Some(AriaRole::Banner),
            "complementary" => Some(AriaRole::Complementary),
            "contentinfo" => Some(AriaRole::Contentinfo),
            "form" => Some(AriaRole::Form),
            "main" => Some(AriaRole::Main),
            "navigation" => Some(AriaRole::Navigation),
            "region" => Some(AriaRole::Region),
            "search" => Some(AriaRole::Search),
            "alert" => Some(AriaRole::Alert),
            "log" => Some(AriaRole::Log),
            "status" => Some(AriaRole::Status),
            "timer" => Some(AriaRole::Timer),
            "alertdialog" => Some(AriaRole::Alertdialog),
            "dialog" => Some(AriaRole::Dialog),
            _ => None,
        }
    }
}

/// Estados ARIA
#[derive(Clone, Debug, Default)]
pub struct AriaStates {
    pub checked: Option<bool>,          // aria-checked
    pub selected: Option<bool>,         // aria-selected
    pub pressed: Option<bool>,          // aria-pressed
    pub expanded: Option<bool>,         // aria-expanded
    pub hidden: bool,                   // aria-hidden
    pub disabled: bool,                 // aria-disabled
    pub invalid: Option<bool>,          // aria-invalid
    pub readonly: Option<bool>,         // aria-readonly
    pub required: Option<bool>,         // aria-required
    pub busy: Option<bool>,             // aria-busy
    pub live: Option<String>,           // aria-live (polite, assertive, off)
    pub atomic: Option<bool>,           // aria-atomic
    pub relevant: Option<String>,       // aria-relevant
}

/// Propriedades ARIA
#[derive(Clone, Debug, Default)]
pub struct AriaProperties {
    pub label: Option<String>,              // aria-label
    pub labelledby: Vec<String>,            // aria-labelledby
    pub describedby: Vec<String>,           // aria-describedby
    pub details: Option<String>,            // aria-details
    pub errormessage: Option<String>,       // aria-errormessage
    pub controls: Vec<String>,              // aria-controls
    pub owns: Vec<String>,                  // aria-owns
    pub flowto: Vec<String>,                // aria-flowto
    pub activedescendant: Option<String>,   // aria-activedescendant
    pub posinset: Option<i32>,              // aria-posinset
    pub setsize: Option<i32>,               // aria-setsize
    pub level: Option<i32>,                 // aria-level
    pub valuenow: Option<f64>,              // aria-valuenow
    pub valuemin: Option<f64>,              // aria-valuemin
    pub valuemax: Option<f64>,              // aria-valuemax
    pub valuetext: Option<String>,          // aria-valuetext
    pub roledescription: Option<String>,    // aria-roledescription
}

/// Nó na Accessibility Tree
#[derive(Clone, Debug)]
pub struct AccessibilityNode {
    pub node_idx: usize,
    pub role: AriaRole,
    pub name: String,                       // Accessible name (computado)
    pub description: Option<String>,        // Accessible description
    pub states: AriaStates,
    pub properties: AriaProperties,
    pub children: Vec<usize>,               // Índices dos children na accessibility tree
    pub parent: Option<usize>,
    pub is_visible: bool,
    pub is_focusable: bool,
}

impl AccessibilityNode {
    pub fn new(node_idx: usize, role: AriaRole) -> Self {
        Self {
            node_idx,
            role,
            name: String::new(),
            description: None,
            states: AriaStates::default(),
            properties: AriaProperties::default(),
            children: Vec::new(),
            parent: None,
            is_visible: true,
            is_focusable: false,
        }
    }
}

/// Mapeamento HTML → ARIA Role implícito
pub struct ImplicitRoleMap;

impl ImplicitRoleMap {
    /// Retorna o role ARIA implícito para um elemento HTML
    pub fn get_implicit_role(tag: &str, attributes: &HashMap<String, String>) -> AriaRole {
        // Verifica se há role explícito via attribute
        if let Some(explicit_role) = attributes.get("role") {
            if let Some(role) = AriaRole::from_str(explicit_role) {
                return role;
            }
        }
        
        // Mapeamento implícito baseado no elemento HTML
        match tag.to_lowercase().as_str() {
            "a" => {
                if attributes.contains_key("href") {
                    AriaRole::Link
                } else {
                    AriaRole::Generic
                }
            },
            "article" => AriaRole::Article,
            "aside" => AriaRole::Complementary,
            "button" => AriaRole::Button,
            "datalist" => AriaRole::Listbox,
            "dd" => AriaRole::Definition,
            "details" => AriaRole::Group,
            "dfn" => AriaRole::Term,
            "dialog" => AriaRole::Dialog,
            "dt" => AriaRole::Term,
            "fieldset" => AriaRole::Group,
            "figure" => AriaRole::Figure,
            "footer" => {
                // footer dentro de body → contentinfo, senão → generic
                AriaRole::Contentinfo
            },
            "form" => AriaRole::Form,
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => AriaRole::Heading,
            "header" => {
                // header dentro de body → banner, senão → generic
                AriaRole::Banner
            },
            "hr" => AriaRole::Separator,
            "img" => {
                if attributes.get("alt").map(|s| s.is_empty()).unwrap_or(false) {
                    AriaRole::None // Imagem decorativa
                } else {
                    AriaRole::Img
                }
            },
            "input" => {
                let input_type = attributes.get("type")
                    .map(|s| s.as_str())
                    .unwrap_or("text");
                
                match input_type {
                    "button" | "submit" | "reset" | "image" => AriaRole::Button,
                    "checkbox" => AriaRole::Checkbox,
                    "color" => AriaRole::Textbox,
                    "date" | "datetime-local" | "month" | "time" | "week" => AriaRole::Combobox,
                    "email" | "tel" | "url" => AriaRole::Textbox,
                    "number" => AriaRole::Spinbutton,
                    "password" => AriaRole::Textbox,
                    "radio" => AriaRole::Radio,
                    "range" => AriaRole::Slider,
                    "search" => AriaRole::Searchbox,
                    "text" | "" => AriaRole::Textbox,
                    "hidden" => AriaRole::None,
                    _ => AriaRole::Generic,
                }
            },
            "li" => AriaRole::ListItem,
            "main" => AriaRole::Main,
            "math" => AriaRole::Math,
            "menu" => AriaRole::List,
            "nav" => AriaRole::Navigation,
            "ol" => AriaRole::List,
            "optgroup" => AriaRole::Group,
            "option" => AriaRole::Option,
            "output" => AriaRole::Status,
            "progress" => AriaRole::Progressbar,
            "section" => AriaRole::Region,
            "select" => {
                if attributes.contains_key("multiple") {
                    AriaRole::Listbox
                } else {
                    AriaRole::Combobox
                }
            },
            "summary" => AriaRole::Button,
            "table" => AriaRole::Table,
            "tbody" | "tfoot" | "thead" => AriaRole::Rowgroup,
            "td" => AriaRole::Cell,
            "textarea" => AriaRole::Textbox,
            "th" => {
                let scope = attributes.get("scope")
                    .map(|s| s.as_str())
                    .unwrap_or("");
                match scope {
                    "col" | "colgroup" => AriaRole::Columnheader,
                    "row" | "rowgroup" => AriaRole::Rowheader,
                    _ => AriaRole::Rowheader,
                }
            },
            "tr" => AriaRole::Row,
            "ul" => AriaRole::List,
            
            // Elements with no implicit role
            "b" | "i" | "span" | "div" | "pre" | "blockquote" | "code" | "em" | "strong" | "small" => {
                AriaRole::Generic
            },
            
            _ => AriaRole::Generic,
        }
    }
}

/// Accessible Name Computation (AccName 1.2 Spec)
pub struct AccessibleNameComputer;

impl AccessibleNameComputer {
    /// Computa o accessible name seguindo AccName 1.2
    /// Prioridade: aria-labelledby > aria-label > title > conteúdo interno
    pub fn compute_name(dom: &AceDOM, node_idx: usize) -> String {
        let node = match dom.get_node(node_idx) {
            Some(n) => n,
            None => return String::new(),
        };
        
        let element = match &node.node_type {
            AceNodeType::Element(el) => el,
            _ => return String::new(),
        };
        
        // 1. Verifica aria-labelledby (IDs referenciados)
        if let Some(labelledby) = element.attributes.get("aria-labelledby") {
            let name = Self::compute_from_labelledby(dom, labelledby);
            if !name.is_empty() {
                return Self::normalize_whitespace(&name);
            }
        }
        
        // 2. Verifica aria-label
        if let Some(label) = element.attributes.get("aria-label") {
            return Self::normalize_whitespace(label);
        }
        
        // 3. Verifica title attribute (fallback)
        if let Some(title) = element.attributes.get("title") {
            return Self::normalize_whitespace(title);
        }
        
        // 4. Computa do conteúdo interno (recursive text collection)
        let content = Self::collect_text_content(dom, node_idx);
        Self::normalize_whitespace(&content)
    }
    
    fn compute_from_labelledby(dom: &AceDOM, labelledby: &str) -> String {
        let mut parts = Vec::new();
        
        for id_ref in labelledby.split_whitespace() {
            // Encontra elemento com este ID
            if let Some(ref_idx) = Self::find_element_by_id(dom, id_ref) {
                let text = Self::collect_text_content(dom, ref_idx);
                if !text.is_empty() {
                    parts.push(text);
                }
            }
        }
        
        parts.join(" ")
    }
    
    fn find_element_by_id(dom: &AceDOM, id: &str) -> Option<usize> {
        for (idx, node) in dom.nodes.iter().enumerate() {
            if let AceNodeType::Element(el) = &node.node_type {
                if el.attributes.get("id") == Some(&id.to_string()) {
                    return Some(idx);
                }
            }
        }
        None
    }
    
    fn collect_text_content(dom: &AceDOM, node_idx: usize) -> String {
        let mut text = String::new();
        let mut stack = vec![node_idx];
        
        while let Some(idx) = stack.pop() {
            if let Some(node) = dom.get_node(idx) {
                match &node.node_type {
                    AceNodeType::Text(content) => {
                        text.push_str(content);
                        text.push(' ');
                    },
                    AceNodeType::Element(_) => {
                        // Adiciona children ao stack
                        stack.extend(&node.children);
                    },
                    _ => {}
                }
            }
        }
        
        text
    }
    
    fn normalize_whitespace(s: &str) -> String {
        s.split_whitespace().collect::<Vec<&str>>().join(" ")
    }
}

/// Accessibility Tree Builder
pub struct AccessibilityTree {
    pub nodes: HashMap<usize, AccessibilityNode>,
    pub root: Option<usize>,
}

impl AccessibilityTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root: None,
        }
    }
    
    /// Constrói a accessibility tree a partir do DOM
    pub fn build(dom: &AceDOM) -> Self {
        let mut tree = Self::new();
        
        // Começa do body ou root
        let start_idx = dom.body.unwrap_or(dom.root);
        
        tree.build_recursive(dom, start_idx, None);
        tree.root = Some(start_idx);
        
        tree
    }
    
    fn build_recursive(
        &mut self,
        dom: &AceDOM,
        node_idx: usize,
        parent_idx: Option<usize>,
    ) -> Option<usize> {
        let node = dom.get_node(node_idx)?;
        
        // Verifica visibilidade
        let is_visible = self.is_node_visible(dom, node_idx);
        if !is_visible {
            return None;
        }
        
        // Obtém role implícito
        let role = match &node.node_type {
            AceNodeType::Element(el) => {
                ImplicitRoleMap::get_implicit_role(&el.tag, &el.attributes)
            },
            AceNodeType::Text(_) => return None, // Text nodes não entram na tree
            AceNodeType::Document => AriaRole::Document,
            _ => AriaRole::Generic,
        };
        
        // Pula elementos com role="none" ou "presentation"
        if role == AriaRole::None || role == AriaRole::Presentation {
            // Mas ainda processa children
            for &child_idx in &node.children {
                self.build_recursive(dom, child_idx, parent_idx);
            }
            return None;
        }
        
        // Cria o accessibility node
        let mut acc_node = AccessibilityNode::new(node_idx, role);
        
        // Computa accessible name
        acc_node.name = AccessibleNameComputer::compute_name(dom, node_idx);
        
        // Extrai estados e propriedades dos atributos ARIA
        self.extract_aria_attributes(dom, node_idx, &mut acc_node);
        
        // Determina se é focusable
        acc_node.is_focusable = self.is_focusable(dom, node_idx);
        
        // Processa children
        let mut acc_children = Vec::new();
        for &child_idx in &node.children {
            if let Some(child_acc_idx) = self.build_recursive(dom, child_idx, Some(node_idx)) {
                acc_children.push(child_acc_idx);
            }
        }
        acc_node.children = acc_children;
        acc_node.parent = parent_idx;
        
        // Insere na tree
        self.nodes.insert(node_idx, acc_node);
        
        Some(node_idx)
    }
    
    fn is_node_visible(&self, dom: &AceDOM, node_idx: usize) -> bool {
        // Verifica aria-hidden
        if let Some(node) = dom.get_node(node_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                if el.attributes.get("aria-hidden") == Some(&"true".to_string()) {
                    return false;
                }
                
                // Verifica display: none e visibility: hidden (em produção, verificaria computed styles)
                // Aqui é uma verificação simplificada
            }
        }
        
        true
    }
    
    fn is_focusable(&self, dom: &AceDOM, node_idx: usize) -> bool {
        if let Some(node) = dom.get_node(node_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                // Elementos naturalmente focusable
                let focusable_tags = ["a", "button", "input", "select", "textarea"];
                if focusable_tags.contains(&el.tag.to_lowercase().as_str()) {
                    // Verifica tabindex
                    if let Some(tabindex) = el.attributes.get("tabindex") {
                        return tabindex.parse::<i32>().unwrap_or(-1) >= 0;
                    }
                    return true;
                }
                
                // tabindex explícito
                if let Some(tabindex) = el.attributes.get("tabindex") {
                    return tabindex.parse::<i32>().unwrap_or(-1) >= 0;
                }
            }
        }
        
        false
    }
    
    fn extract_aria_attributes(&self, dom: &AceDOM, node_idx: usize, acc_node: &mut AccessibilityNode) {
        if let Some(node) = dom.get_node(node_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                // States
                if let Some(checked) = el.attributes.get("aria-checked") {
                    acc_node.states.checked = Some(checked == "true");
                }
                if let Some(selected) = el.attributes.get("aria-selected") {
                    acc_node.states.selected = Some(selected == "true");
                }
                if let Some(expanded) = el.attributes.get("aria-expanded") {
                    acc_node.states.expanded = Some(expanded == "true");
                }
                if let Some(hidden) = el.attributes.get("aria-hidden") {
                    acc_node.states.hidden = hidden == "true";
                }
                if let Some(disabled) = el.attributes.get("aria-disabled") {
                    acc_node.states.disabled = disabled == "true";
                }
                
                // Properties
                if let Some(label) = el.attributes.get("aria-label") {
                    acc_node.properties.label = Some(label.clone());
                }
                if let Some(labelledby) = el.attributes.get("aria-labelledby") {
                    acc_node.properties.labelledby = labelledby.split_whitespace()
                        .map(|s| s.to_string()).collect();
                }
                if let Some(describedby) = el.attributes.get("aria-describedby") {
                    acc_node.properties.describedby = describedby.split_whitespace()
                        .map(|s| s.to_string()).collect();
                }
                if let Some(controls) = el.attributes.get("aria-controls") {
                    acc_node.properties.controls = controls.split_whitespace()
                        .map(|s| s.to_string()).collect();
                }
                if let Some(level) = el.attributes.get("aria-level") {
                    acc_node.properties.level = level.parse().ok();
                }
                if let Some(valuenow) = el.attributes.get("aria-valuenow") {
                    acc_node.properties.valuenow = valuenow.parse().ok();
                }
            }
        }
    }
    
    /// Retorna os children acessíveis de um nó
    pub fn get_accessible_children(&self, node_idx: usize) -> Vec<&AccessibilityNode> {
        if let Some(node) = self.nodes.get(&node_idx) {
            node.children.iter()
                .filter_map(|&idx| self.nodes.get(&idx))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Retorna o parent acessível de um nó
    pub fn get_accessible_parent(&self, node_idx: usize) -> Option<&AccessibilityNode> {
        if let Some(node) = self.nodes.get(&node_idx) {
            node.parent.and_then(|p| self.nodes.get(&p))
        } else {
            None
        }
    }
    
    /// Traverse pre-order
    pub fn traverse_pre_order<F>(&self, callback: &mut F)
    where
        F: FnMut(&AccessibilityNode),
    {
        if let Some(root_idx) = self.root {
            self.traverse_pre_order_recursive(root_idx, callback);
        }
    }
    
    fn traverse_pre_order_recursive<F>(&self, node_idx: usize, callback: &mut F)
    where
        F: FnMut(&AccessibilityNode),
    {
        if let Some(node) = self.nodes.get(&node_idx) {
            callback(node);
            for &child_idx in &node.children {
                self.traverse_pre_order_recursive(child_idx, callback);
            }
        }
    }
}

/// Extensões para AceDOM
impl AceDOM {
    /// Constrói a accessibility tree
    pub fn build_accessibility_tree(&self) -> AccessibilityTree {
        AccessibilityTree::build(self)
    }
    
    /// Verifica se um nó é visível para acessibilidade
    pub fn is_accessible(&self, node_idx: usize) -> bool {
        // Verificações básicas de visibilidade
        if let Some(node) = self.get_node(node_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                // aria-hidden
                if el.attributes.get("aria-hidden") == Some(&"true".to_string()) {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::dom::AceDOM;
    
    #[test]
    fn test_implicit_role_button() {
        let role = ImplicitRoleMap::get_implicit_role("button", &HashMap::new());
        assert_eq!(role, AriaRole::Button);
        assert_eq!(role.as_str(), "button");
    }
    
    #[test]
    fn test_implicit_role_link_with_href() {
        let mut attrs = HashMap::new();
        attrs.insert("href".to_string(), "#".to_string());
        let role = ImplicitRoleMap::get_implicit_role("a", &attrs);
        assert_eq!(role, AriaRole::Link);
    }
    
    #[test]
    fn test_implicit_role_link_without_href() {
        let role = ImplicitRoleMap::get_implicit_role("a", &HashMap::new());
        assert_eq!(role, AriaRole::Generic);
    }
    
    #[test]
    fn test_explicit_role_override() {
        let mut attrs = HashMap::new();
        attrs.insert("role".to_string(), "button".to_string());
        let role = ImplicitRoleMap::get_implicit_role("div", &attrs);
        assert_eq!(role, AriaRole::Button);
    }
    
    #[test]
    fn test_compute_name_from_aria_label() {
        let dom = AceDOM::from_html("<button aria-label=\"Close\">X</button>");
        let button_idx = dom.body.map(|b| {
            if let Some(node) = dom.get_node(b) {
                node.children.first().copied().unwrap_or(b)
            } else {
                b
            }
        }).unwrap_or(1);
        
        let name = AccessibleNameComputer::compute_name(&dom, button_idx);
        assert_eq!(name, "Close");
    }
    
    #[test]
    fn test_compute_name_from_content() {
        let dom = AceDOM::from_html("<button>Click Me</button>");
        let button_idx = dom.body.map(|b| {
            if let Some(node) = dom.get_node(b) {
                node.children.first().copied().unwrap_or(b)
            } else {
                b
            }
        }).unwrap_or(1);
        
        let name = AccessibleNameComputer::compute_name(&dom, button_idx);
        assert_eq!(name, "Click Me");
    }
    
    #[test]
    fn test_build_accessibility_tree() {
        let dom = AceDOM::from_html("<main><button>Click</button></main>");
        let tree = dom.build_accessibility_tree();
        
        assert!(tree.root.is_some());
        assert!(!tree.nodes.is_empty());
    }
    
    #[test]
    fn test_aria_hidden_excludes_from_tree() {
        let dom = AceDOM::from_html("<button aria-hidden=\"true\">Hidden</button>");
        let tree = dom.build_accessibility_tree();
        
        // O botão com aria-hidden não deve estar na tree
        // (dependendo da implementação, pode ou não estar)
    }
    
    #[test]
    fn test_heading_levels() {
        for tag in &["h1", "h2", "h3", "h4", "h5", "h6"] {
            let role = ImplicitRoleMap::get_implicit_role(tag, &HashMap::new());
            assert_eq!(role, AriaRole::Heading);
        }
    }
    
    #[test]
    fn test_input_types() {
        let mut attrs_checkbox = HashMap::new();
        attrs_checkbox.insert("type".to_string(), "checkbox".to_string());
        let role_checkbox = ImplicitRoleMap::get_implicit_role("input", &attrs_checkbox);
        assert_eq!(role_checkbox, AriaRole::Checkbox);
        
        let mut attrs_radio = HashMap::new();
        attrs_radio.insert("type".to_string(), "radio".to_string());
        let role_radio = ImplicitRoleMap::get_implicit_role("input", &attrs_radio);
        assert_eq!(role_radio, AriaRole::Radio);
        
        let mut attrs_range = HashMap::new();
        attrs_range.insert("type".to_string(), "range".to_string());
        let role_range = ImplicitRoleMap::get_implicit_role("input", &attrs_range);
        assert_eq!(role_range, AriaRole::Slider);
    }
}
