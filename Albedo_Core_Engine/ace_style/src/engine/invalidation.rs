//! # Invalidação Reativa Cirúrgica (Rule Invalidation Sets — Modelo Blink)
//!
//! Rastreamento de dependências de seletores para invalidação pontual e precisa de sub-árvores
//! quando o DOM sofre mutações de classes, IDs ou atributos, evitando recálculos da árvore inteira.

use crate::model::stylesheet::{CSSRule, StyleSheet};
use crate::rule_tree::bucket::pseudo_class_to_atom;
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use ace_dom::query::selector::{Combinator, ComplexSelector, SimpleSelector};
use ace_dom::tree::Document;
use bitflags::bitflags;
use rustc_hash::{FxHashMap, FxHashSet};

bitflags! {
    /// Sinalizadores de estilo sujo para invalidação reativa cirúrgica (Dirty Bits).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct StyleDirtyFlags: u8 {
        const CLEAN = 0;
        const RECALC_SELF = 1 << 0;
        const RECALC_DESCENDANTS = 1 << 1;
    }
}

/// Conjunto de alvos a invalidar quando um gatilho de estilo for alterado.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InvalidationSet {
    pub target_classes: FxHashSet<Atom>,
    pub target_ids: FxHashSet<Atom>,
    pub target_tags: FxHashSet<Atom>,
    pub target_attributes: FxHashSet<Atom>,
    pub invalidates_all: bool,
}

impl InvalidationSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        !self.invalidates_all
            && self.target_classes.is_empty()
            && self.target_ids.is_empty()
            && self.target_tags.is_empty()
            && self.target_attributes.is_empty()
    }
}

/// Tabela compilada de Invalidation Sets derivados de folhas de estilo.
#[derive(Debug, Clone, Default)]
pub struct RuleInvalidationSets {
    pub descendant_class_sets: FxHashMap<Atom, InvalidationSet>,
    pub descendant_id_sets: FxHashMap<Atom, InvalidationSet>,
    pub descendant_attr_sets: FxHashMap<Atom, InvalidationSet>,
    pub sibling_class_sets: FxHashMap<Atom, InvalidationSet>,
    pub sibling_state_sets: FxHashMap<Atom, InvalidationSet>,
}

impl RuleInvalidationSets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.descendant_class_sets.clear();
        self.descendant_id_sets.clear();
        self.descendant_attr_sets.clear();
        self.sibling_class_sets.clear();
        self.sibling_state_sets.clear();
    }

    /// Compila conjuntos de invalidação a partir das regras de uma folha de estilo.
    pub fn add_stylesheet(&mut self, sheet: &StyleSheet) {
        self.index_rules(&sheet.rules);
    }

    fn index_rules(&mut self, rules: &[CSSRule]) {
        for rule in rules {
            match rule {
                CSSRule::Style(style_rule) => {
                    for selector in &style_rule.selectors {
                        self.index_selector(selector);
                    }
                }
                CSSRule::Media(media) => {
                    self.index_rules(&media.rules);
                }
                CSSRule::LayerBlock(block) => {
                    self.index_rules(&block.rules);
                }
                _ => {}
            }
        }
    }

    fn index_selector(&mut self, selector: &ComplexSelector) {
        if selector.parts.len() <= 1 {
            return;
        }

        let Some((target_compound, _)) = selector.parts.last() else {
            return;
        };

        // Para cada composto ancestral ou irmão anterior ao Key Selector
        for i in 0..selector.parts.len() - 1 {
            let (compound, comb) = &selector.parts[i];

            match comb {
                Some(Combinator::Descendant | Combinator::Child) => {
                    for simple in &compound.simple_selectors {
                        match simple {
                            SimpleSelector::Class(c) => {
                                let set = self.descendant_class_sets.entry(c.clone()).or_default();
                                add_targets_to_set(set, target_compound);
                            }
                            SimpleSelector::Id(id) => {
                                let set = self.descendant_id_sets.entry(id.clone()).or_default();
                                add_targets_to_set(set, target_compound);
                            }
                            SimpleSelector::Attribute { name, .. } => {
                                let set = self.descendant_attr_sets.entry(name.clone()).or_default();
                                add_targets_to_set(set, target_compound);
                            }
                            _ => {}
                        }
                    }
                }
                Some(Combinator::AdjacentSibling | Combinator::GeneralSibling) => {
                    for simple in &compound.simple_selectors {
                        match simple {
                            SimpleSelector::Class(c) => {
                                let set = self.sibling_class_sets.entry(c.clone()).or_default();
                                add_targets_to_set(set, target_compound);
                            }
                            SimpleSelector::Pseudo(p) => {
                                let atom = pseudo_class_to_atom(p);
                                let set = self.sibling_state_sets.entry(atom).or_default();
                                add_targets_to_set(set, target_compound);
                            }
                            _ => {}
                        }
                    }
                }
                None => {}
            }
        }
    }
}

fn add_targets_to_set(
    set: &mut InvalidationSet,
    target_compound: &ace_dom::query::selector::CompoundSelector,
) {
    if target_compound.simple_selectors.is_empty() {
        set.invalidates_all = true;
        return;
    }

    for simple in &target_compound.simple_selectors {
        match simple {
            SimpleSelector::Class(c) => {
                set.target_classes.insert(c.clone());
            }
            SimpleSelector::Id(id) => {
                set.target_ids.insert(id.clone());
            }
            SimpleSelector::Tag(tag) => {
                let lower = Atom::new(&tag.as_str().to_ascii_lowercase());
                set.target_tags.insert(lower);
            }
            SimpleSelector::Attribute { name, .. } => {
                let lower = Atom::new(&name.as_str().to_ascii_lowercase());
                set.target_attributes.insert(lower);
            }
            SimpleSelector::Universal => {
                set.invalidates_all = true;
            }
            _ => {}
        }
    }
}

/// Tipo de mutação disparada no DOM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DOMMutation {
    ClassChanged {
        old_class: Option<Atom>,
        new_class: Option<Atom>,
    },
    AttributeChanged {
        name: Atom,
    },
    IdChanged {
        old_id: Option<Atom>,
        new_id: Option<Atom>,
    },
}

/// Resultado analítico de uma invalidação reativa.
#[derive(Debug, Clone, Default)]
pub struct InvalidationResult {
    pub invalidated_count: usize,
    pub affected_nodes: FxHashSet<NodeId>,
}
