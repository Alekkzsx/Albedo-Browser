//! # Casamento de Seletores CSS Simples (Selector Engine)
//!
//! Avaliação rápida de seletores de tag, classe, ID e combinadores básicos.

use crate::node::NodeData;
use ace_core::intern::Atom;

/// Um seletor simples que pode ser casado contra um nó da árvore.
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
}

impl SimpleSelector {
    /// Faz o parse de uma string de seletor simples.
    pub fn parse(input: &str) -> Option<Self> {
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

        // Tag name
        Some(Self::Tag(Atom::new(&trimmed.to_ascii_lowercase())))
    }

    /// Avalia se um determinado nó satisfaz este seletor.
    pub fn matches(&self, node: &NodeData) -> bool {
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
        }
    }
}
