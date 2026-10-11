//! # Indexação de Regras em Baldes de Busca Rápida Estendidos (Rule Buckets)
//!
//! Particionamento de regras CSS por ID, Classe, Tag, Atributo ([attr]), Estado (:checked, :disabled, etc.)
//! e Pseudo-Elementos (::before, ::after) para busca em O(1) com Universal Bucket verdadeiramente enxuto.

use crate::model::stylesheet::{CSSRule, StyleSheet};
use ace_core::intern::Atom;
use ace_dom::query::selector::{ComplexSelector, PseudoClass, PseudoElement, SimpleSelector};
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
    pub attribute_buckets: FxHashMap<Atom, Vec<RuleRef>>,
    pub state_buckets: FxHashMap<Atom, Vec<RuleRef>>,
    pub pseudo_element_buckets: FxHashMap<PseudoElement, Vec<RuleRef>>,
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
        self.attribute_buckets.clear();
        self.state_buckets.clear();
        self.pseudo_element_buckets.clear();
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

        // 1. Prioridade máxima: Pseudo-elementos (::before, ::after)
        for simple in &key_compound.simple_selectors {
            if let SimpleSelector::PseudoElement(pe) = simple {
                self.pseudo_element_buckets
                    .entry(*pe)
                    .or_default()
                    .push(rule_ref);
                return;
            }
        }

        // 2. Balde de ID (#id)
        for simple in &key_compound.simple_selectors {
            if let SimpleSelector::Id(id) = simple {
                self.id_buckets
                    .entry(id.clone())
                    .or_default()
                    .push(rule_ref);
                return;
            }
        }

        // 3. Balde de Classe (.class)
        for simple in &key_compound.simple_selectors {
            if let SimpleSelector::Class(class) = simple {
                self.class_buckets
                    .entry(class.clone())
                    .or_default()
                    .push(rule_ref);
                return;
            }
        }

        // 4. Balde de Tag (div, p, span, etc.)
        for simple in &key_compound.simple_selectors {
            if let SimpleSelector::Tag(tag) = simple {
                let lower = Atom::new(&tag.as_str().to_ascii_lowercase());
                self.tag_buckets.entry(lower).or_default().push(rule_ref);
                return;
            }
        }

        // 5. Balde de Atributo ([attr])
        for simple in &key_compound.simple_selectors {
            if let SimpleSelector::Attribute { name, .. } = simple {
                let lower = Atom::new(&name.as_str().to_ascii_lowercase());
                self.attribute_buckets
                    .entry(lower)
                    .or_default()
                    .push(rule_ref);
                return;
            }
        }

        // 6. Balde de Estado / Pseudo-classe (:checked, :disabled, etc.)
        for simple in &key_compound.simple_selectors {
            if let SimpleSelector::Pseudo(p) = simple {
                let state_atom = pseudo_class_to_atom(p);
                self.state_buckets
                    .entry(state_atom)
                    .or_default()
                    .push(rule_ref);
                return;
            }
        }

        // 7. Universal Bucket (apenas curingas verdadeiros '*')
        self.universal_bucket.push(rule_ref);
    }
}

pub fn pseudo_class_to_atom(p: &PseudoClass) -> Atom {
    match p {
        PseudoClass::FirstChild => Atom::new("first-child"),
        PseudoClass::LastChild => Atom::new("last-child"),
        PseudoClass::OnlyChild => Atom::new("only-child"),
        PseudoClass::Empty => Atom::new("empty"),
        PseudoClass::Root => Atom::new("root"),
        PseudoClass::Checked => Atom::new("checked"),
        PseudoClass::Disabled => Atom::new("disabled"),
        PseudoClass::Enabled => Atom::new("enabled"),
        PseudoClass::Required => Atom::new("required"),
        PseudoClass::Optional => Atom::new("optional"),
        PseudoClass::Hover => Atom::new("hover"),
        PseudoClass::Active => Atom::new("active"),
        PseudoClass::Focus => Atom::new("focus"),
        PseudoClass::FocusVisible => Atom::new("focus-visible"),
        PseudoClass::Target => Atom::new("target"),
        _ => Atom::new("other"),
    }
}
