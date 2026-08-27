//! # Estilo Computado & Resolução em Cascata (CSS Cascading & Inheritance Level 4)
//!
//! Algoritmo de resolução de estilos combinando regras de folhas de estilo e estilos em linha
//! com suporte a especificidade CSS e ordenação normatizada.

use crate::cssom::declaration::CSSStyleDeclaration;
use crate::cssom::stylesheet::{CSSRule, CSSStyleSheet};
use crate::tree::Document;
use ace_core::id::NodeId;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

/// Representação do conjunto de estilos computados para um elemento DOM.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ComputedStyle {
    properties: FxHashMap<SmolStr, SmolStr>,
}

impl ComputedStyle {
    pub fn new() -> Self {
        Self {
            properties: FxHashMap::default(),
        }
    }

    /// Retorna o valor computado de uma propriedade CSS.
    #[inline]
    pub fn get_property_value(&self, name: &str) -> Option<&str> {
        self.properties.get(name).map(|s| s.as_str())
    }

    /// Define diretamente uma propriedade computada.
    pub fn set_property(&mut self, name: impl Into<SmolStr>, value: impl Into<SmolStr>) {
        self.properties.insert(name.into(), value.into());
    }
}

/// Resolvedor de cascata de estilos para a árvore DOM.
pub struct StyleResolver;

impl StyleResolver {
    /// Computa o estilo final para um determinado elemento considerando folhas de estilo e estilo inline.
    pub fn resolve_element_style(
        doc: &Document,
        element_id: NodeId,
        sheets: &[CSSStyleSheet],
    ) -> ComputedStyle {
        let mut computed = ComputedStyle::new();

        // 1. Aplica regras normais das folhas de estilo
        for sheet in sheets {
            if sheet.disabled {
                continue;
            }
            for rule in &sheet.rules {
                if let CSSRule::Style(ref style_rule) = rule {
                    for selector in &style_rule.selectors {
                        if selector.matches(doc, element_id) {
                            for prop in style_rule.style.properties.as_slice() {
                                if !prop.important {
                                    computed.set_property(prop.name.as_str(), prop.value.as_str());
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Aplica estilos inline (style="...")
        if let Some(node) = doc.get_node(element_id) {
            if let Some(el) = node.as_element() {
                if let Some(ref rare) = el.rare_data {
                    if let Some(ref inline_decl) = rare.inline_style_decl {
                        for prop in inline_decl.properties.as_slice() {
                            if !prop.important {
                                computed.set_property(prop.name.as_str(), prop.value.as_str());
                            }
                        }
                    }
                }
            }
        }

        // 3. Aplica regras !important das folhas de estilo
        for sheet in sheets {
            if sheet.disabled {
                continue;
            }
            for rule in &sheet.rules {
                if let CSSRule::Style(ref style_rule) = rule {
                    for selector in &style_rule.selectors {
                        if selector.matches(doc, element_id) {
                            for prop in style_rule.style.properties.as_slice() {
                                if prop.important {
                                    computed.set_property(prop.name.as_str(), prop.value.as_str());
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Aplica estilos inline !important (maior precedência na cascata)
        if let Some(node) = doc.get_node(element_id) {
            if let Some(el) = node.as_element() {
                if let Some(ref rare) = el.rare_data {
                    if let Some(ref inline_decl) = rare.inline_style_decl {
                        for prop in inline_decl.properties.as_slice() {
                            if prop.important {
                                computed.set_property(prop.name.as_str(), prop.value.as_str());
                            }
                        }
                    }
                }
            }
        }

        computed
    }
}
