//! # Motor de Seletores CSS e Combinadores (Selector Engine)
//!
//! Suporte a seletores atômicos (Tag, ID, Classe, Atributo, Pseudo-classes)
//! e combinadores hierárquicos (Filho direto `>`, Descendente ` `, Irmão Adjacente `+`, Irmão Geral `~`).

use crate::node::NodeData;
use crate::tree::Document;
use ace_core::id::NodeId;
use ace_core::intern::Atom;

/// Helper para obter o irmão anterior que seja um Elemento (ignorando nós de texto/comentário).
fn prev_element_sibling(doc: &Document, node_id: NodeId) -> Option<NodeId> {
    let mut curr = doc.get_node(node_id)?.prev_sibling;
    while let Some(id) = curr {
        let node = doc.get_node(id)?;
        if node.is_element() {
            return Some(id);
        }
        curr = node.prev_sibling;
    }
    None
}

/// Helper para obter o irmão posterior que seja um Elemento (ignorando nós de texto/comentário).
fn next_element_sibling(doc: &Document, node_id: NodeId) -> Option<NodeId> {
    let mut curr = doc.get_node(node_id)?.next_sibling;
    while let Some(id) = curr {
        let node = doc.get_node(id)?;
        if node.is_element() {
            return Some(id);
        }
        curr = node.next_sibling;
    }
    None
}

/// Operadores de casamento de atributos CSS (Selectors Level 4 §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeOp {
    /// Presença do atributo (`[attr]`)
    Exists,
    /// Igualdade exata (`[attr="val"]`)
    Exact(String),
    /// Prefixo (`[attr^="val"]`)
    Prefix(String),
    /// Sufixo (`[attr$="val"]`)
    Suffix(String),
    /// Substring (`[attr*="val"]`)
    Contains(String),
    /// Lista separada por espaços contém palavra (`[attr~="val"]`)
    Includes(String),
    /// Correspondência de prefixo com hífen (`[attr|="val"]`)
    DashMatch(String),
    /// Igualdade exata insensível a maiúsculas (`[attr="val" i]`)
    ExactCaseInsensitive(String),
    /// Prefixo insensível a maiúsculas (`[attr^="val" i]`)
    PrefixCaseInsensitive(String),
    /// Sufixo insensível a maiúsculas (`[attr$="val" i]`)
    SuffixCaseInsensitive(String),
    /// Substring insensível a maiúsculas (`[attr*="val" i]`)
    ContainsCaseInsensitive(String),
}

/// Helper para parsing da fórmula An+B de pseudo-classes `:nth-*`.
fn parse_an_plus_b(s: &str) -> Option<(i32, i32)> {
    let s = s.trim().to_ascii_lowercase().replace(' ', "");
    if s == "odd" {
        return Some((2, 1));
    }
    if s == "even" {
        return Some((2, 0));
    }
    if let Ok(num) = s.parse::<i32>() {
        return Some((0, num));
    }
    if s == "n" {
        return Some((1, 0));
    }
    if s == "-n" {
        return Some((-1, 0));
    }
    if let Some(b_str) = s.strip_prefix("n+") {
        let b = b_str.parse::<i32>().ok()?;
        return Some((1, b));
    }
    if let Some(b_str) = s.strip_prefix("n-") {
        let b = b_str.parse::<i32>().ok()?;
        return Some((1, -b));
    }
    if let Some(b_str) = s.strip_prefix("-n+") {
        let b = b_str.parse::<i32>().ok()?;
        return Some((-1, b));
    }
    if let Some(b_str) = s.strip_prefix("-n-") {
        let b = b_str.parse::<i32>().ok()?;
        return Some((-1, -b));
    }
    if let Some((a_str, b_str)) = s.split_once("n+") {
        let a = a_str.parse::<i32>().ok()?;
        let b = b_str.parse::<i32>().ok()?;
        return Some((a, b));
    }
    if let Some((a_str, b_str)) = s.split_once("n-") {
        let a = a_str.parse::<i32>().ok()?;
        let b = b_str.parse::<i32>().ok()?;
        return Some((a, -b));
    }
    if let Some(a_str) = s.strip_suffix('n') {
        let a = if a_str.is_empty() || a_str == "+" {
            1
        } else if a_str == "-" {
            -1
        } else {
            a_str.parse::<i32>().ok()?
        };
        return Some((a, 0));
    }
    None
}

fn matches_an_plus_b(idx: i32, a: i32, b: i32) -> bool {
    if a == 0 {
        idx == b
    } else {
        (idx - b) % a == 0 && (idx - b) / a >= 0
    }
}

/// Pseudo-classes estruturais e de estado suportadas no DOM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PseudoClass {
    FirstChild,
    LastChild,
    OnlyChild,
    Empty,
    Root,
    Checked,
    Disabled,
    Enabled,
    Required,
    Optional,
    Hover,
    Active,
    Focus,
    FocusVisible,
    Target,
    NthChild(i32, i32),
    NthLastChild(i32, i32),
    NthOfType(i32, i32),
    NthLastOfType(i32, i32),
    Not(Box<SimpleSelector>),
    Is(Vec<ComplexSelector>),
    Where(Vec<ComplexSelector>),
    Has(Box<ComplexSelector>),
}

/// Um seletor simples que pode ser casado contra um único nó.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleSelector {
    /// Seletor universal `*`
    Universal,
    /// Seletor de tag (ex: `div`, `p`, `span`)
    Tag(Atom),
    /// Seletor de ID (ex: `#main`)
    Id(Atom),
    /// Seletor de Classe (ex: `.active`)
    Class(Atom),
    /// Seletor de Atributo com operador (ex: `[target="_blank"]`, `[href^="https"]`)
    Attribute {
        name: Atom,
        op: AttributeOp,
    },
    /// Pseudo-classe estrutural ou de estado
    Pseudo(PseudoClass),
}

impl SimpleSelector {
    /// Faz o parse de um seletor atômico.
    pub fn parse_atomic(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed == "*" {
            return Some(Self::Universal);
        }

        if let Some(id_str) = trimmed.strip_prefix('#') {
            return Some(Self::Id(Atom::new(id_str)));
        }

        if let Some(class_str) = trimmed.strip_prefix('.') {
            return Some(Self::Class(Atom::new(class_str)));
        }

        if let Some(pseudo_str) = trimmed.strip_prefix(':') {
            let lower = pseudo_str.to_ascii_lowercase();
            if lower == "first-child" {
                return Some(Self::Pseudo(PseudoClass::FirstChild));
            } else if lower == "last-child" {
                return Some(Self::Pseudo(PseudoClass::LastChild));
            } else if lower == "only-child" {
                return Some(Self::Pseudo(PseudoClass::OnlyChild));
            } else if lower == "empty" {
                return Some(Self::Pseudo(PseudoClass::Empty));
            } else if lower == "root" {
                return Some(Self::Pseudo(PseudoClass::Root));
            } else if lower == "checked" {
                return Some(Self::Pseudo(PseudoClass::Checked));
            } else if lower == "disabled" {
                return Some(Self::Pseudo(PseudoClass::Disabled));
            } else if lower == "enabled" {
                return Some(Self::Pseudo(PseudoClass::Enabled));
            } else if lower == "required" {
                return Some(Self::Pseudo(PseudoClass::Required));
            } else if lower == "optional" {
                return Some(Self::Pseudo(PseudoClass::Optional));
            } else if lower == "hover" {
                return Some(Self::Pseudo(PseudoClass::Hover));
            } else if lower == "active" {
                return Some(Self::Pseudo(PseudoClass::Active));
            } else if lower == "focus" {
                return Some(Self::Pseudo(PseudoClass::Focus));
            } else if lower == "focus-visible" {
                return Some(Self::Pseudo(PseudoClass::FocusVisible));
            } else if lower == "target" {
                return Some(Self::Pseudo(PseudoClass::Target));
            } else if let Some(arg) = lower.strip_prefix("nth-child(").and_then(|s| s.strip_suffix(')')) {
                if let Some((a, b)) = parse_an_plus_b(arg) {
                    return Some(Self::Pseudo(PseudoClass::NthChild(a, b)));
                }
            } else if let Some(arg) = lower.strip_prefix("nth-last-child(").and_then(|s| s.strip_suffix(')')) {
                if let Some((a, b)) = parse_an_plus_b(arg) {
                    return Some(Self::Pseudo(PseudoClass::NthLastChild(a, b)));
                }
            } else if let Some(arg) = lower.strip_prefix("nth-of-type(").and_then(|s| s.strip_suffix(')')) {
                if let Some((a, b)) = parse_an_plus_b(arg) {
                    return Some(Self::Pseudo(PseudoClass::NthOfType(a, b)));
                }
            } else if let Some(arg) = lower.strip_prefix("nth-last-of-type(").and_then(|s| s.strip_suffix(')')) {
                if let Some((a, b)) = parse_an_plus_b(arg) {
                    return Some(Self::Pseudo(PseudoClass::NthLastOfType(a, b)));
                }
            } else if let Some(arg) = lower.strip_prefix("not(").and_then(|s| s.strip_suffix(')')) {
                if let Some(inner_sel) = Self::parse_atomic(arg) {
                    return Some(Self::Pseudo(PseudoClass::Not(Box::new(inner_sel))));
                }
            } else if let Some(arg) = lower.strip_prefix("is(").and_then(|s| s.strip_suffix(')')) {
                let selectors: Vec<ComplexSelector> = arg
                    .split(',')
                    .filter_map(|s| ComplexSelector::parse(s.trim()))
                    .collect();
                if !selectors.is_empty() {
                    return Some(Self::Pseudo(PseudoClass::Is(selectors)));
                }
            } else if let Some(arg) = lower.strip_prefix("where(").and_then(|s| s.strip_suffix(')')) {
                let selectors: Vec<ComplexSelector> = arg
                    .split(',')
                    .filter_map(|s| ComplexSelector::parse(s.trim()))
                    .collect();
                if !selectors.is_empty() {
                    return Some(Self::Pseudo(PseudoClass::Where(selectors)));
                }
            } else if let Some(arg) = lower.strip_prefix("has(").and_then(|s| s.strip_suffix(')')) {
                if let Some(inner_sel) = ComplexSelector::parse(arg) {
                    return Some(Self::Pseudo(PseudoClass::Has(Box::new(inner_sel))));
                }
            }
            return None;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = &trimmed[1..trimmed.len() - 1];

            fn parse_val_and_case(v: &str) -> (String, bool) {
                let trimmed_v = v.trim();
                if let Some(rest) = trimmed_v.strip_suffix(" i").or_else(|| trimmed_v.strip_suffix(" I")) {
                    (rest.trim().trim_matches('"').trim_matches('\'').to_string(), true)
                } else if let Some(rest) = trimmed_v.strip_suffix(" s").or_else(|| trimmed_v.strip_suffix(" S")) {
                    (rest.trim().trim_matches('"').trim_matches('\'').to_string(), false)
                } else {
                    (trimmed_v.trim_matches('"').trim_matches('\'').to_string(), false)
                }
            }

            if let Some((attr_name, attr_val)) = inner.split_once("^=") {
                let (val, case_i) = parse_val_and_case(attr_val);
                let op = if case_i {
                    AttributeOp::PrefixCaseInsensitive(val)
                } else {
                    AttributeOp::Prefix(val)
                };
                return Some(Self::Attribute {
                    name: Atom::new(attr_name.trim()),
                    op,
                });
            } else if let Some((attr_name, attr_val)) = inner.split_once("$=") {
                let (val, case_i) = parse_val_and_case(attr_val);
                let op = if case_i {
                    AttributeOp::SuffixCaseInsensitive(val)
                } else {
                    AttributeOp::Suffix(val)
                };
                return Some(Self::Attribute {
                    name: Atom::new(attr_name.trim()),
                    op,
                });
            } else if let Some((attr_name, attr_val)) = inner.split_once("*=") {
                let (val, case_i) = parse_val_and_case(attr_val);
                let op = if case_i {
                    AttributeOp::ContainsCaseInsensitive(val)
                } else {
                    AttributeOp::Contains(val)
                };
                return Some(Self::Attribute {
                    name: Atom::new(attr_name.trim()),
                    op,
                });
            } else if let Some((attr_name, attr_val)) = inner.split_once("~=") {
                let (val, _) = parse_val_and_case(attr_val);
                return Some(Self::Attribute {
                    name: Atom::new(attr_name.trim()),
                    op: AttributeOp::Includes(val),
                });
            } else if let Some((attr_name, attr_val)) = inner.split_once("|=") {
                let (val, _) = parse_val_and_case(attr_val);
                return Some(Self::Attribute {
                    name: Atom::new(attr_name.trim()),
                    op: AttributeOp::DashMatch(val),
                });
            } else if let Some((attr_name, attr_val)) = inner.split_once('=') {
                let (val, case_i) = parse_val_and_case(attr_val);
                let op = if case_i {
                    AttributeOp::ExactCaseInsensitive(val)
                } else {
                    AttributeOp::Exact(val)
                };
                return Some(Self::Attribute {
                    name: Atom::new(attr_name.trim()),
                    op,
                });
            } else {
                return Some(Self::Attribute {
                    name: Atom::new(inner.trim()),
                    op: AttributeOp::Exists,
                });
            }
        }

        // Tag name canônica (apenas se for um identificador puro sem combinadores ou modificadores)
        if !trimmed.contains('.')
            && !trimmed.contains('#')
            && !trimmed.contains(':')
            && !trimmed.contains('[')
        {
            Some(Self::Tag(Atom::new(&trimmed.to_ascii_lowercase())))
        } else {
            None
        }
    }

    /// Avalia se um determinado nó satisfaz este seletor atômico.
    pub fn matches(&self, doc: &Document, node_id: NodeId, node: &NodeData) -> bool {
        let el = match node.as_element() {
            Some(e) => e,
            None => return false,
        };

        match self {
            Self::Universal => true,
            Self::Tag(tag) => el.tag_name.eq_ignore_ascii_case(tag.as_str()),
            Self::Id(id) => el.id_attr.as_ref() == Some(id),
            Self::Class(class) => el.has_class(class.as_str()),
            Self::Attribute { name, op } => {
                let attr_val = el.get_attribute(name.as_str());
                match op {
                    AttributeOp::Exists => attr_val.is_some(),
                    AttributeOp::Exact(expected) => attr_val == Some(expected.as_str()),
                    AttributeOp::Prefix(prefix) => attr_val.is_some_and(|v| v.starts_with(prefix)),
                    AttributeOp::Suffix(suffix) => attr_val.is_some_and(|v| v.ends_with(suffix)),
                    AttributeOp::Contains(sub) => attr_val.is_some_and(|v| v.contains(sub)),
                    AttributeOp::Includes(word) => {
                        attr_val.is_some_and(|v| v.split_ascii_whitespace().any(|w| w == word))
                    }
                    AttributeOp::DashMatch(prefix) => attr_val.is_some_and(|v| {
                        v == prefix || v.starts_with(&format!("{}-", prefix))
                    }),
                    AttributeOp::ExactCaseInsensitive(expected) => {
                        attr_val.is_some_and(|v| v.eq_ignore_ascii_case(expected))
                    }
                    AttributeOp::PrefixCaseInsensitive(prefix) => {
                        attr_val.is_some_and(|v| v.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()))
                    }
                    AttributeOp::SuffixCaseInsensitive(suffix) => {
                        attr_val.is_some_and(|v| v.to_ascii_lowercase().ends_with(&suffix.to_ascii_lowercase()))
                    }
                    AttributeOp::ContainsCaseInsensitive(sub) => {
                        attr_val.is_some_and(|v| v.to_ascii_lowercase().contains(&sub.to_ascii_lowercase()))
                    }
                }
            }
            Self::Pseudo(pseudo) => match pseudo {
                PseudoClass::FirstChild => prev_element_sibling(doc, node_id).is_none(),
                PseudoClass::LastChild => next_element_sibling(doc, node_id).is_none(),
                PseudoClass::OnlyChild => {
                    prev_element_sibling(doc, node_id).is_none()
                        && next_element_sibling(doc, node_id).is_none()
                }
                PseudoClass::Empty => node.first_child.is_none(),
                PseudoClass::Root => node.parent == Some(doc.root()),
                PseudoClass::Checked => el.has_attribute("checked") || el.has_attribute("selected"),
                PseudoClass::Disabled => el.has_attribute("disabled"),
                PseudoClass::Enabled => !el.has_attribute("disabled"),
                PseudoClass::Required => el.has_attribute("required"),
                PseudoClass::Optional => !el.has_attribute("required"),
                PseudoClass::NthChild(a, b) => {
                    // Calcula índice 1-based entre os irmãos elementos
                    let mut idx = 1;
                    let mut curr = prev_element_sibling(doc, node_id);
                    while let Some(prev) = curr {
                        idx += 1;
                        curr = prev_element_sibling(doc, prev);
                    }
                    matches_an_plus_b(idx, *a, *b)
                }
                PseudoClass::NthLastChild(a, b) => {
                    let mut idx = 1;
                    let mut curr = next_element_sibling(doc, node_id);
                    while let Some(next) = curr {
                        idx += 1;
                        curr = next_element_sibling(doc, next);
                    }
                    matches_an_plus_b(idx, *a, *b)
                }
                PseudoClass::NthOfType(a, b) => {
                    let my_tag = &el.tag_name;
                    let mut idx = 1;
                    let mut curr = prev_element_sibling(doc, node_id);
                    while let Some(prev) = curr {
                        if let Some(prev_node) = doc.get_node(prev) {
                            if let Some(prev_el) = prev_node.as_element() {
                                if prev_el.tag_name == *my_tag {
                                    idx += 1;
                                }
                            }
                        }
                        curr = prev_element_sibling(doc, prev);
                    }
                    matches_an_plus_b(idx, *a, *b)
                }
                PseudoClass::NthLastOfType(a, b) => {
                    let my_tag = &el.tag_name;
                    let mut idx = 1;
                    let mut curr = next_element_sibling(doc, node_id);
                    while let Some(next) = curr {
                        if let Some(next_node) = doc.get_node(next) {
                            if let Some(next_el) = next_node.as_element() {
                                if next_el.tag_name == *my_tag {
                                    idx += 1;
                                }
                            }
                        }
                        curr = next_element_sibling(doc, next);
                    }
                    matches_an_plus_b(idx, *a, *b)
                }
                PseudoClass::Hover => el.has_attribute("data-hover"),
                PseudoClass::Active => el.has_attribute("data-active"),
                PseudoClass::Focus => el.has_attribute("data-focus") || el.has_attribute("autofocus"),
                PseudoClass::FocusVisible => {
                    el.has_attribute("data-focus-visible") || el.has_attribute("autofocus")
                }
                PseudoClass::Target => {
                    if let Some(ref id) = el.id_attr {
                        if let Some(doc_node) = doc.get_node(doc.root()) {
                            if let crate::node::NodeKind::Document(ref d_data) = doc_node.kind {
                                if let Some(ref url) = d_data.url {
                                    if let Some((_, frag)) = url.split_once('#') {
                                        return id.as_str() == frag;
                                    }
                                }
                            }
                        }
                    }
                    false
                }
                PseudoClass::Not(inner) => !inner.matches(doc, node_id, node),
                PseudoClass::Is(selectors) | PseudoClass::Where(selectors) => {
                    selectors.iter().any(|sel| sel.matches(doc, node_id))
                }
                PseudoClass::Has(inner) => {
                    for (desc_id, _) in doc.descendants(node_id) {
                        if desc_id != node_id && inner.matches(doc, desc_id) {
                            return true;
                        }
                    }
                    false
                }
            },
        }
    }
}

/// Combinador hierárquico entre seletores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// Filho direto (`>`)
    Child,
    /// Descendente qualquer (espaço)
    Descendant,
    /// Irmão adjacente (`+`)
    AdjacentSibling,
    /// Irmão geral (`~`)
    GeneralSibling,
}

/// Um seletor composto (ex: `div.container#main`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompoundSelector {
    pub simple_selectors: Vec<SimpleSelector>,
}

impl CompoundSelector {
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut simple_selectors = Vec::new();
        // Se for um seletor atômico único
        if let Some(atomic) = SimpleSelector::parse_atomic(trimmed) {
            simple_selectors.push(atomic);
            return Some(Self { simple_selectors });
        }

        // Divide seletores múltiplos concatenados (ex: `div.active#main[data-v="1.0"]:first-child`)
        let mut curr = String::new();
        let mut in_bracket = false;
        let mut in_quote: Option<char> = None;
        let mut in_paren = 0;

        for ch in trimmed.chars() {
            match ch {
                '"' | '\'' => {
                    if in_quote == Some(ch) {
                        in_quote = None;
                    } else if in_quote.is_none() {
                        in_quote = Some(ch);
                    }
                    curr.push(ch);
                }
                '(' if in_quote.is_none() => {
                    in_paren += 1;
                    curr.push(ch);
                }
                ')' if in_quote.is_none() => {
                    if in_paren > 0 {
                        in_paren -= 1;
                    }
                    curr.push(ch);
                    if in_paren == 0 && !in_bracket {
                        if let Some(s) = SimpleSelector::parse_atomic(&curr) {
                            simple_selectors.push(s);
                        }
                        curr.clear();
                    }
                }
                '[' if in_quote.is_none() => {
                    if !in_bracket && in_paren == 0 && !curr.is_empty() {
                        if let Some(s) = SimpleSelector::parse_atomic(&curr) {
                            simple_selectors.push(s);
                        }
                        curr.clear();
                    }
                    in_bracket = true;
                    curr.push(ch);
                }
                ']' if in_quote.is_none() => {
                    in_bracket = false;
                    curr.push(ch);
                    if in_paren == 0 {
                        if let Some(s) = SimpleSelector::parse_atomic(&curr) {
                            simple_selectors.push(s);
                        }
                        curr.clear();
                    }
                }
                '.' | '#' | ':' if !in_bracket && in_quote.is_none() && in_paren == 0 => {
                    if !curr.is_empty() {
                        if let Some(s) = SimpleSelector::parse_atomic(&curr) {
                            simple_selectors.push(s);
                        }
                        curr.clear();
                    }
                    curr.push(ch);
                }
                _ => {
                    curr.push(ch);
                }
            }
        }

        if !curr.is_empty() {
            if let Some(s) = SimpleSelector::parse_atomic(&curr) {
                simple_selectors.push(s);
            }
        }

        if simple_selectors.is_empty() {
            None
        } else {
            Some(Self { simple_selectors })
        }
    }

    pub fn matches(&self, doc: &Document, node_id: NodeId, node: &NodeData) -> bool {
        self.simple_selectors
            .iter()
            .all(|s| s.matches(doc, node_id, node))
    }
}

enum SelectorToken {
    Compound(String),
    Comb(Combinator),
}

/// Um seletor CSS completo com cadeia de combinadores (ex: `div.content > p + span`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplexSelector {
    pub parts: Vec<(CompoundSelector, Option<Combinator>)>,
}

impl ComplexSelector {
    /// Faz o parse de uma cadeia de seletores com combinadores de qualquer comprimento.
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut tokens = Vec::new();
        let mut curr = String::new();
        let mut in_bracket = false;
        let mut in_quote: Option<char> = None;
        let mut in_paren = 0;

        let chars: Vec<char> = trimmed.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            match c {
                '"' | '\'' => {
                    if in_quote == Some(c) {
                        in_quote = None;
                    } else if in_quote.is_none() {
                        in_quote = Some(c);
                    }
                    curr.push(c);
                    i += 1;
                }
                '(' if in_quote.is_none() => {
                    in_paren += 1;
                    curr.push(c);
                    i += 1;
                }
                ')' if in_quote.is_none() => {
                    if in_paren > 0 {
                        in_paren -= 1;
                    }
                    curr.push(c);
                    i += 1;
                }
                '[' if in_quote.is_none() => {
                    in_bracket = true;
                    curr.push(c);
                    i += 1;
                }
                ']' if in_quote.is_none() => {
                    in_bracket = false;
                    curr.push(c);
                    i += 1;
                }
                '>' | '+' | '~' if !in_bracket && in_quote.is_none() && in_paren == 0 => {
                    let prev_chunk = curr.trim();
                    if !prev_chunk.is_empty() {
                        tokens.push(SelectorToken::Compound(prev_chunk.to_string()));
                        curr.clear();
                    } else if let Some(SelectorToken::Comb(Combinator::Descendant)) = tokens.last() {
                        tokens.pop();
                    }
                    let comb = match c {
                        '>' => Combinator::Child,
                        '+' => Combinator::AdjacentSibling,
                        '~' => Combinator::GeneralSibling,
                        _ => unreachable!(),
                    };
                    tokens.push(SelectorToken::Comb(comb));
                    i += 1;
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                }
                ' ' if !in_bracket && in_quote.is_none() && in_paren == 0 => {
                    let prev_chunk = curr.trim();
                    if !prev_chunk.is_empty() {
                        tokens.push(SelectorToken::Compound(prev_chunk.to_string()));
                        curr.clear();
                    }
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    if i < chars.len() && (chars[i] == '>' || chars[i] == '+' || chars[i] == '~') {
                        continue;
                    }
                    if !matches!(tokens.last(), Some(SelectorToken::Comb(_))) {
                        tokens.push(SelectorToken::Comb(Combinator::Descendant));
                    }
                }
                _ => {
                    curr.push(c);
                    i += 1;
                }
            }
        }

        let remaining = curr.trim();
        if !remaining.is_empty() {
            tokens.push(SelectorToken::Compound(remaining.to_string()));
        }

        let mut parts = Vec::new();
        let mut idx = 0;
        while idx < tokens.len() {
            match &tokens[idx] {
                SelectorToken::Compound(s) => {
                    let compound = CompoundSelector::parse(s)?;
                    let comb = if idx + 1 < tokens.len() {
                        if let SelectorToken::Comb(c) = &tokens[idx + 1] {
                            idx += 1;
                            Some(*c)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    parts.push((compound, comb));
                }
                SelectorToken::Comb(_) => {
                    return None;
                }
            }
            idx += 1;
        }

        if parts.is_empty() {
            None
        } else {
            Some(Self { parts })
        }
    }

    /// Avalia se um nó específico do documento casa com este seletor complexo usando matching Right-to-Left (RTL).
    pub fn matches(&self, doc: &Document, node_id: NodeId) -> bool {
        let node = match doc.get_node(node_id) {
            Some(n) => n,
            None => return false,
        };

        if self.parts.is_empty() {
            return false;
        }

        // 1. Testa o Key Selector (mais à direita) em O(1)
        let last_idx = self.parts.len() - 1;
        let (key_selector, _) = &self.parts[last_idx];
        if !key_selector.matches(doc, node_id, node) {
            return false;
        }

        if self.parts.len() == 1 {
            return true;
        }

        // 2. Caminha da direita para a esquerda na cadeia de combinadores
        self.match_chain_rtl(doc, node_id, last_idx)
    }

    fn match_chain_rtl(&self, doc: &Document, curr_node_id: NodeId, curr_idx: usize) -> bool {
        if curr_idx == 0 {
            return true;
        }

        let prev_idx = curr_idx - 1;
        let (prev_selector, combinator) = &self.parts[prev_idx];

        match combinator {
            Some(Combinator::Child) => {
                let parent_id = match doc.get_node(curr_node_id).and_then(|n| n.parent) {
                    Some(p) => p,
                    None => return false,
                };
                let parent_node = match doc.get_node(parent_id) {
                    Some(n) if n.is_element() => n,
                    _ => return false,
                };
                if prev_selector.matches(doc, parent_id, parent_node) {
                    self.match_chain_rtl(doc, parent_id, prev_idx)
                } else {
                    false
                }
            }
            Some(Combinator::Descendant) => {
                for (ancestor_id, ancestor_node) in doc.ancestors(curr_node_id) {
                    if ancestor_node.is_element() && prev_selector.matches(doc, ancestor_id, ancestor_node) {
                        if self.match_chain_rtl(doc, ancestor_id, prev_idx) {
                            return true;
                        }
                    }
                }
                false
            }
            Some(Combinator::AdjacentSibling) => {
                let prev_id = match prev_element_sibling(doc, curr_node_id) {
                    Some(id) => id,
                    None => return false,
                };
                let prev_node = match doc.get_node(prev_id) {
                    Some(n) => n,
                    None => return false,
                };
                if prev_selector.matches(doc, prev_id, prev_node) {
                    self.match_chain_rtl(doc, prev_id, prev_idx)
                } else {
                    false
                }
            }
            Some(Combinator::GeneralSibling) => {
                let mut curr = prev_element_sibling(doc, curr_node_id);
                while let Some(prev_id) = curr {
                    if let Some(prev_node) = doc.get_node(prev_id) {
                        if prev_selector.matches(doc, prev_id, prev_node) {
                            if self.match_chain_rtl(doc, prev_id, prev_idx) {
                                return true;
                            }
                        }
                        curr = prev_element_sibling(doc, prev_id);
                    } else {
                        break;
                    }
                }
                false
            }
            None => true,
        }
    }
}
