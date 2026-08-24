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

/// Pseudo-classes estruturais suportadas no DOM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoClass {
    FirstChild,
    LastChild,
    OnlyChild,
    Empty,
    Root,
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
    /// Seletor de Atributo (ex: `[target="_blank"]` ou `[disabled]`)
    Attribute {
        name: Atom,
        value: Option<String>,
    },
    /// Pseudo-classe estrutural
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
            return match pseudo_str.to_ascii_lowercase().as_str() {
                "first-child" => Some(Self::Pseudo(PseudoClass::FirstChild)),
                "last-child" => Some(Self::Pseudo(PseudoClass::LastChild)),
                "only-child" => Some(Self::Pseudo(PseudoClass::OnlyChild)),
                "empty" => Some(Self::Pseudo(PseudoClass::Empty)),
                "root" => Some(Self::Pseudo(PseudoClass::Root)),
                _ => None,
            };
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = &trimmed[1..trimmed.len() - 1];
            if let Some((attr_name, attr_val)) = inner.split_once('=') {
                let clean_val = attr_val.trim().trim_matches('"').trim_matches('\'');
                return Some(Self::Attribute {
                    name: Atom::new(attr_name.trim()),
                    value: Some(clean_val.to_string()),
                });
            } else {
                return Some(Self::Attribute {
                    name: Atom::new(inner.trim()),
                    value: None,
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
            Self::Tag(tag) => el.tag_name == *tag,
            Self::Id(id) => el.id_attr.as_ref() == Some(id),
            Self::Class(class) => el.has_class(class.as_str()),
            Self::Attribute { name, value } => {
                if let Some(val_expected) = value {
                    el.get_attribute(name.as_str()) == Some(val_expected.as_str())
                } else {
                    el.get_attribute(name.as_str()).is_some()
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

        // Divide seletores múltiplos concatenados (ex: `div.active#main`)
        let mut curr = String::new();
        for ch in trimmed.chars() {
            if (ch == '.' || ch == '#' || ch == ':' || ch == '[') && !curr.is_empty() {
                if let Some(s) = SimpleSelector::parse_atomic(&curr) {
                    simple_selectors.push(s);
                }
                curr.clear();
            }
            curr.push(ch);
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

/// Um seletor CSS completo com cadeia de combinadores (ex: `div.content > p + span`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplexSelector {
    pub parts: Vec<(CompoundSelector, Option<Combinator>)>,
}

impl ComplexSelector {
    /// Faz o parse de uma cadeia de seletores com combinadores.
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Tenta combinador de filho direto `>`
        if let Some((left, right)) = trimmed.split_once('>') {
            let left_comp = CompoundSelector::parse(left)?;
            let right_comp = CompoundSelector::parse(right)?;
            return Some(Self {
                parts: vec![
                    (left_comp, Some(Combinator::Child)),
                    (right_comp, None),
                ],
            });
        }

        // Tenta combinador de irmão adjacente `+`
        if let Some((left, right)) = trimmed.split_once('+') {
            let left_comp = CompoundSelector::parse(left)?;
            let right_comp = CompoundSelector::parse(right)?;
            return Some(Self {
                parts: vec![
                    (left_comp, Some(Combinator::AdjacentSibling)),
                    (right_comp, None),
                ],
            });
        }

        // Tenta combinador de irmão geral `~`
        if let Some((left, right)) = trimmed.split_once('~') {
            let left_comp = CompoundSelector::parse(left)?;
            let right_comp = CompoundSelector::parse(right)?;
            return Some(Self {
                parts: vec![
                    (left_comp, Some(Combinator::GeneralSibling)),
                    (right_comp, None),
                ],
            });
        }

        // Tenta combinador de descendente (espaço)
        if trimmed.contains(' ') {
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            if words.len() == 2 {
                let left_comp = CompoundSelector::parse(words[0])?;
                let right_comp = CompoundSelector::parse(words[1])?;
                return Some(Self {
                    parts: vec![
                        (left_comp, Some(Combinator::Descendant)),
                        (right_comp, None),
                    ],
                });
            }
        }

        // Seletor composto único
        let compound = CompoundSelector::parse(trimmed)?;
        Some(Self {
            parts: vec![(compound, None)],
        })
    }

    /// Avalia se um nó específico do documento casa com este seletor complexo.
    pub fn matches(&self, doc: &Document, node_id: NodeId) -> bool {
        let node = match doc.get_node(node_id) {
            Some(n) => n,
            None => return false,
        };

        if self.parts.len() == 1 {
            return self.parts[0].0.matches(doc, node_id, node);
        }

        if self.parts.len() == 2 {
            let (target_comp, _) = &self.parts[1];
            if !target_comp.matches(doc, node_id, node) {
                return false;
            }

            let (antecedent_comp, combinator) = &self.parts[0];
            match combinator {
                Some(Combinator::Child) => {
                    if let Some(parent_id) = node.parent {
                        if let Some(parent_node) = doc.get_node(parent_id) {
                            return antecedent_comp.matches(doc, parent_id, parent_node);
                        }
                    }
                    false
                }
                Some(Combinator::Descendant) => {
                    for (ancestor_id, ancestor_node) in doc.ancestors(node_id) {
                        if antecedent_comp.matches(doc, ancestor_id, ancestor_node) {
                            return true;
                        }
                    }
                    false
                }
                Some(Combinator::AdjacentSibling) => {
                    if let Some(prev_id) = prev_element_sibling(doc, node_id) {
                        if let Some(prev_node) = doc.get_node(prev_id) {
                            return antecedent_comp.matches(doc, prev_id, prev_node);
                        }
                    }
                    false
                }
                Some(Combinator::GeneralSibling) => {
                    let mut curr = prev_element_sibling(doc, node_id);
                    while let Some(prev_id) = curr {
                        if let Some(prev_node) = doc.get_node(prev_id) {
                            if antecedent_comp.matches(doc, prev_id, prev_node) {
                                return true;
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
        } else {
            false
        }
    }
}
