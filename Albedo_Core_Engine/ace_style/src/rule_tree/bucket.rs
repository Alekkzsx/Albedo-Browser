//! # Indexação de Regras em Baldes de Busca Rápida (Rule Buckets)
//!
//! Particionamento de regras CSS por ID, Classe, Tag e Seletor Universal para busca em O(1).

use crate::model::stylesheet::{CSSRule, StyleSheet};
use ace_core::intern::Atom;
use ace_dom::query::selector::{ComplexSelector, SimpleSelector};
use rustc_hash::FxHashMap;

/// Referência compacta a uma regra indexada em um balde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuleRef {
    pub sheet_index: usize,
    pub rule_index: usize,
    pub selector_index: usize,
}

/// Mapa de baldes de regras CSS particionadas pelo Key Selector (mais à direita).
#[derive(Debug, Clone, Default)]
pub struct RuleBucketMap {
    pub id_buckets: FxHashMap<Atom, Vec<RuleRef>>,
    pub class_buckets: FxHashMap<Atom, Vec<RuleRef>>,
    pub tag_buckets: FxHashMap<Atom, Vec<RuleRef>>,
    pub universal_bucket: Vec<RuleRef>,
}

impl RuleBucketMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Limpa todos os baldes indexados.
    pub fn clear(&mut self) {
        self.id_buckets.clear();
        self.class_buckets.clear();
        self.tag_buckets.clear();
        self.universal_bucket.clear();
    }

    /// Indexa todas as regras de uma folha de estilo nos baldes correspondentes.
    pub fn add_stylesheet(&mut self, sheet_index: usize, sheet: &StyleSheet) {
        self.index_rules(sheet_index, &sheet.rules);
    }

    fn index_rules(&mut self, sheet_index: usize, rules: &[CSSRule]) {
        for (rule_index, rule) in rules.iter().enumerate() {
            match rule {
                CSSRule::Style(style_rule) => {
                    for (selector_index, selector) in style_rule.selectors.iter().enumerate() {
                        let rule_ref = RuleRef {
                            sheet_index,
                            rule_index,
                            selector_index,
                        };
                        self.index_selector(selector, rule_ref);
                    }
                }
                CSSRule::Media(media_rule) => {
                    self.index_rules(sheet_index, &media_rule.rules);
                }
                CSSRule::LayerBlock(layer_block) => {
                    self.index_rules(sheet_index, &layer_block.rules);
                }
                _ => {}
            }
        }
    }

    fn index_selector(&mut self, selector: &ComplexSelector, rule_ref: RuleRef) {
        // Encontra o Key Selector (seletor composto mais à direita)
        let Some((key_compound, _)) = selector.parts.last() else {
            self.universal_bucket.push(rule_ref);
            return;
        };

        // Prioridade de balde: ID > Class > Tag > Universal
        for simple in &key_compound.simple_selectors {
            if let SimpleSelector::Id(id) = simple {
                self.id_buckets.entry(id.clone()).or_default().push(rule_ref);
                return;
            }
        }

        for simple in &key_compound.simple_selectors {
            if let SimpleSelector::Class(class) = simple {
                self.class_buckets.entry(class.clone()).or_default().push(rule_ref);
                return;
            }
        }

        for simple in &key_compound.simple_selectors {
            if let SimpleSelector::Tag(tag) = simple {
                let lower = Atom::new(&tag.as_str().to_ascii_lowercase());
                self.tag_buckets.entry(lower).or_default().push(rule_ref);
                return;
            }
        }

        self.universal_bucket.push(rule_ref);
    }
}
