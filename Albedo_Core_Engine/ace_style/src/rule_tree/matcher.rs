//! # Casamento Rápido de Seletores com AncestorFilter e Baldes Estendidos
//!
//! Rejeição em O(1) de seletores descendentes, particionamento por atributos e estados,
//! e casamento especializado de pseudo-elementos (`::before`, `::after`).

use crate::cascade::origin::StyleSheetOrigin;
use crate::model::stylesheet::{CSSRule, StyleRule, StyleSheet};
use crate::parser::declaration_parser::ParsedDeclaration;
use crate::rule_tree::bucket::{RuleBucketMap, RuleRef};
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use ace_dom::query::bloom::AncestorFilter;
use ace_dom::query::selector::{
    Combinator, ComplexSelector, PseudoElement, SimpleSelector, Specificity,
};
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

        // 1. Balde por ID (#id)
        if let Some(ref id) = el.id_attr {
            if let Some(rules) = self.id_buckets.get(id) {
                for r in rules {
                    if seen.insert(*r) {
                        candidate_refs.push(*r);
                    }
                }
            }
        }

        // 2. Baldes por Classes (.class)
        for class in el.classes.as_slice() {
            if let Some(rules) = self.class_buckets.get(class) {
                for r in rules {
                    if seen.insert(*r) {
                        candidate_refs.push(*r);
                    }
                }
            }
        }

        // 3. Balde por Tag (div, p, h1, etc.)
        let lower_tag = Atom::new(&el.tag_name.as_str().to_ascii_lowercase());
        if let Some(rules) = self.tag_buckets.get(&lower_tag) {
            for r in rules {
                if seen.insert(*r) {
                    candidate_refs.push(*r);
                }
            }
        }

        // 4. Baldes por Atributos ([attr])
        for attr in el.attributes.as_slice() {
            let lower_attr = Atom::new(&attr.name.as_str().to_ascii_lowercase());
            if let Some(rules) = self.attribute_buckets.get(&lower_attr) {
                for r in rules {
                    if seen.insert(*r) {
                        candidate_refs.push(*r);
                    }
                }
            }
        }

        // 5. Baldes por Estado / Pseudo-classes (:checked, :disabled, etc.)
        let mut check_state = |state: &str| {
            let atom = Atom::new(state);
            if let Some(rules) = self.state_buckets.get(&atom) {
                for r in rules {
                    if seen.insert(*r) {
                        candidate_refs.push(*r);
                    }
                }
            }
        };

        if el.has_attribute("checked") {
            check_state("checked");
        }
        if el.has_attribute("disabled") {
            check_state("disabled");
        }
        if el.has_attribute("data-hover") {
            check_state("hover");
        }
        if el.has_attribute("data-active") {
            check_state("active");
        }
        if el.has_attribute("data-focus") || el.has_attribute("autofocus") {
            check_state("focus");
            check_state("focus-visible");
        }
        if node.first_child.is_none() {
            check_state("empty");
        }
        if node.prev_sibling.is_none() {
            check_state("first-child");
        }
        if node.next_sibling.is_none() {
            check_state("last-child");
        }
        if node.prev_sibling.is_none() && node.next_sibling.is_none() {
            check_state("only-child");
        }
        if node_id == doc.root() {
            check_state("root");
        }

        // 6. Balde Universal (regras verdadeiras '*')
        for r in &self.universal_bucket {
            if seen.insert(*r) {
                candidate_refs.push(*r);
            }
        }

        let mut matched = Vec::new();

        // 7. Avalia os candidatos com Fast-Reject via AncestorFilter
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

            // Fast-Reject via Bloom Filter
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

    /// Coleta todas as regras que casam para um pseudo-elemento específico (`::before` ou `::after`).
    pub fn collect_matching_pseudo_rules(
        &self,
        doc: &Document,
        node_id: NodeId,
        pe: PseudoElement,
        sheets: &[StyleSheet],
    ) -> Vec<MatchedRule> {
        let Some(candidate_refs) = self.pseudo_element_buckets.get(&pe) else {
            return Vec::new();
        };

        let mut matched = Vec::new();

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

            if matches_pseudo_selector(doc, node_id, selector, pe) {
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
    if selector.parts.len() <= 1 {
        return false;
    }

    for i in 0..selector.parts.len() - 1 {
        let (compound, comb) = &selector.parts[i];
        if matches!(comb, Some(Combinator::Descendant | Combinator::Child))
            && filter.fast_reject(compound)
        {
            return true;
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

/// Avalia se um seletor que termina em um pseudo-elemento casa o elemento hospedeiro.
fn matches_pseudo_selector(
    doc: &Document,
    node_id: NodeId,
    selector: &ComplexSelector,
    pe: PseudoElement,
) -> bool {
    let Some((key_compound, _)) = selector.parts.last() else {
        return false;
    };

    // O Key Selector deve conter o pseudo-elemento alvo
    if !key_compound
        .simple_selectors
        .iter()
        .any(|s| matches!(s, SimpleSelector::PseudoElement(target) if *target == pe))
    {
        return false;
    }

    // Constrói seletor sem o pseudo-elemento para testar contra o elemento hospedeiro
    let mut new_parts = selector.parts.clone();
    let last_idx = new_parts.len() - 1;
    let (ref mut last_compound, _) = new_parts[last_idx];

    last_compound
        .simple_selectors
        .retain(|s| !matches!(s, SimpleSelector::PseudoElement(target) if *target == pe));

    if last_compound.simple_selectors.is_empty() {
        last_compound
            .simple_selectors
            .push(SimpleSelector::Universal);
    }

    let modified_selector = ComplexSelector { parts: new_parts };
    modified_selector.matches(doc, node_id)
}
