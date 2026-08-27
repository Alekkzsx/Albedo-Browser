//! # Estilo Computado & Resolução em Cascata (CSS Cascading & Inheritance Level 4)
//!
//! Algoritmo de resolução de estilos combinando regras de folhas de estilo e estilos em linha
//! com suporte a especificidade CSS e ordenação normatizada.

use crate::cssom::declaration::CSSProperty;
use crate::cssom::stylesheet::{CSSRule, CSSStyleSheet};
use crate::query::Specificity;
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

#[derive(Clone)]
struct MatchedProperty<'a> {
    property: &'a CSSProperty,
    specificity: Specificity,
    order: usize,
}

/// Resolvedor de cascata de estilos para a árvore DOM conforme W3C CSS Cascading Level 4.
pub struct StyleResolver;

impl StyleResolver {
    /// Computa o estilo final para um determinado elemento considerando folhas de estilo (com especificidade) e estilo inline.
    pub fn resolve_element_style(
        doc: &Document,
        element_id: NodeId,
        sheets: &[CSSStyleSheet],
    ) -> ComputedStyle {
        let mut computed = ComputedStyle::new();
        let mut normal_decls: Vec<MatchedProperty> = Vec::new();
        let mut important_decls: Vec<MatchedProperty> = Vec::new();
        let mut order_counter = 0;

        // 1. Coleta todas as regras das folhas de estilo que casam com o elemento
        for sheet in sheets {
            if sheet.disabled {
                continue;
            }
            for rule in &sheet.rules {
                if let CSSRule::Style(ref style_rule) = rule {
                    for selector in &style_rule.selectors {
                        if selector.matches(doc, element_id) {
                            let spec = selector.specificity();
                            for prop in style_rule.style.properties.as_slice() {
                                order_counter += 1;
                                let matched = MatchedProperty {
                                    property: prop,
                                    specificity: spec,
                                    order: order_counter,
                                };
                                if prop.important {
                                    important_decls.push(matched);
                                } else {
                                    normal_decls.push(matched);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Ordena declarações normais por especificidade e ordem de aparição
        normal_decls.sort_by(|a, b| {
            a.specificity
                .cmp(&b.specificity)
                .then_with(|| a.order.cmp(&b.order))
        });

        // 1. Aplica regras normais das folhas de estilo ordenadas
        for decl in normal_decls {
            computed.set_property(decl.property.name.as_str(), decl.property.value.as_str());
        }

        // 2. Aplica estilos inline normais (style="...")
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

        // Ordena declarações !important por especificidade e ordem
        important_decls.sort_by(|a, b| {
            a.specificity
                .cmp(&b.specificity)
                .then_with(|| a.order.cmp(&b.order))
        });

        // 3. Aplica regras !important das folhas de estilo
        for decl in important_decls {
            computed.set_property(decl.property.name.as_str(), decl.property.value.as_str());
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
