//! # Casamento Rápido de Seletores com AncestorFilter (Bloom Filter)
//!
//! Rejeição em O(1) de seletores descendentes e casamento contra nós da árvore DOM.

use crate::cascade::origin::StyleSheetOrigin;
use crate::model::stylesheet::{CSSRule, StyleRule, StyleSheet};
use crate::parser::declaration_parser::ParsedDeclaration;
use crate::rule_tree::bucket::{RuleBucketMap, RuleRef};
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use ace_dom::query::bloom::AncestorFilter;
use ace_dom::query::selector::{Combinator, ComplexSelector, Specificity};
use ace_dom::tree::Document;
use rustc_hash::FxHashSet;

/// Uma regra CSS casada com sucesso contra um determinado nó DOM.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchedRule {
    pub sheet_origin: StyleSheetOrigin,
    pub sheet_index: usize,
    pub rule_index: usize,
    pub selector_index: usize,
    pub specificity: Specificity,
    pub source_order: u32,
    pub layer: Option<Atom>,
    pub declarations: Vec<ParsedDeclaration>,
}

impl RuleBucketMap {
    /// Coleta todas as regras que casam contra o nó informado no documento.
    pub fn collect_matching_rules(
        &self,
        doc: &Document,
        node_id: NodeId,
        sheets: &[StyleSheet],
        ancestor_filter: Option<&AncestorFilter>,
    ) -> Vec<MatchedRule> {
        let Some(node) = doc.get_node(node_id) else {
            return Vec::new();
        };

        let Some(el) = node.as_element() else {
            return Vec::new();
        };

        let mut candidate_refs: Vec<RuleRef> = Vec::new();
        let mut seen = FxHashSet::default();

        // 1. Balde por ID
        if let Some(ref id) = el.id_attr {
            if let Some(rules) = self.id_buckets.get(id) {
                for r in rules {
                    if seen.insert(*r) {
                        candidate_refs.push(*r);
                    }
                }
            }
        }

        // 2. Baldes por Classes
        for class in el.classes.as_slice() {
            if let Some(rules) = self.class_buckets.get(class) {
                for r in rules {
                    if seen.insert(*r) {
                        candidate_refs.push(*r);
                    }
                }
            }
        }

        // 3. Balde por Tag
        let lower_tag = Atom::new(&el.tag_name.as_str().to_ascii_lowercase());
        if let Some(rules) = self.tag_buckets.get(&lower_tag) {
            for r in rules {
                if seen.insert(*r) {
                    candidate_refs.push(*r);
                }
            }
        }

        // 4. Balde Universal
        for r in &self.universal_bucket {
            if seen.insert(*r) {
                candidate_refs.push(*r);
            }
        }

        let mut matched = Vec::new();

        // 5. Avalia os candidatos com Fast-Reject via AncestorFilter
        for candidate in candidate_refs {
            let Some(sheet) = sheets.get(candidate.sheet_index) else {
                continue;
            };

            let Some(style_rule) = get_style_rule(&sheet.rules, candidate.rule_index) else {
                continue;
            };

            let Some(selector) = style_rule.selectors.get(candidate.selector_index) else {
                continue;
            };

            // Fast-Reject via Bloom Filter: se o seletor exigir ancestrais e o filtro puder rejeitar com 100% de certeza
            if let Some(filter) = ancestor_filter {
                if should_bloom_reject(selector, filter) {
                    continue;
                }
            }

            // Casamento formal RTL do seletor complexo
            if selector.matches(doc, node_id) {
                let spec = selector_specificity(selector);
                matched.push(MatchedRule {
                    sheet_origin: sheet.origin,
                    sheet_index: candidate.sheet_index,
                    rule_index: candidate.rule_index,
                    selector_index: candidate.selector_index,
                    specificity: spec,
                    source_order: style_rule.source_order,
                    layer: style_rule.layer.clone(),
                    declarations: style_rule.declarations.clone(),
                });
            }
        }

        matched
    }
}

/// Helper para obter uma `StyleRule` desdobrando regras aninhadas se necessário.
fn get_style_rule(rules: &[CSSRule], index: usize) -> Option<&StyleRule> {
    rules.get(index).and_then(|r| match r {
        CSSRule::Style(s) => Some(s),
        _ => None,
    })
}

/// Verifica se o filtro de Bloom de ancestrais pode rejeitar o seletor instantaneamente em O(1).
fn should_bloom_reject(selector: &ComplexSelector, filter: &AncestorFilter) -> bool {
    // Se o seletor só tiver 1 composto (apenas o próprio elemento), não há ancestrais para testar
    if selector.parts.len() <= 1 {
        return false;
    }

    // Para todos os compostos anteriores ao Key Selector
    for i in 0..selector.parts.len() - 1 {
        let (compound, comb) = &selector.parts[i];
        // Se for combinador descendente ou filho
        if matches!(comb, Some(Combinator::Descendant | Combinator::Child)) {
            if filter.fast_reject(compound) {
                return true;
            }
        }
    }

    false
}

fn selector_specificity(selector: &ComplexSelector) -> Specificity {
    let mut spec = Specificity::ZERO;
    for (compound, _) in &selector.parts {
        spec.add(compound.specificity());
    }
    spec
}
