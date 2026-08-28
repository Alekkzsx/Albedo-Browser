//! # Índices Rápidos de Busca DOM (ElementIndex)
//!
//! Tabelas de dispersão de altíssimo throughput para resolução $O(1)$ de `getElementById`
//! e agrupamento instantâneo de classes.

use crate::node::NodeData;
use crate::tree::Document;
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use rustc_hash::FxHashMap;

/// Índice de busca acelerada sobre os elementos de um `Document`.
#[derive(Debug, Clone, Default)]
pub struct ElementIndex {
    id_map: FxHashMap<Atom, NodeId>,
    class_map: FxHashMap<Atom, Vec<NodeId>>,
}

impl ElementIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconstrói o índice completo a partir de um documento.
    pub fn rebuild(&mut self, doc: &Document) {
        self.rebuild_from_arena(&doc.arena, doc.root());
    }

    /// Reconstrói o índice diretamente da arena e nó raiz sem conflito de empréstimo.
    pub fn rebuild_from_arena(&mut self, arena: &ace_core::arena::Arena<NodeData>, root: NodeId) {
        self.id_map.clear();
        self.class_map.clear();

        let descendants = crate::node::iter::DescendantsIter::new(arena, root);
        for (node_id, node) in descendants {
            if let Some(el) = node.as_element() {
                // Mapeia ID
                if let Some(ref id_atom) = el.id_attr {
                    self.id_map.insert(id_atom.clone(), node_id);
                }

                // Mapeia Classes
                for class_atom in el.classes.as_slice() {
                    self.class_map
                        .entry(class_atom.clone())
                        .or_default()
                        .push(node_id);
                }
            }
        }
    }

    /// Indexa um nó adicionado.
    pub fn index_node(&mut self, node_id: NodeId, node: &NodeData) {
        if let Some(el) = node.as_element() {
            if let Some(ref id_atom) = el.id_attr {
                self.id_map.insert(id_atom.clone(), node_id);
            }
            for class_atom in el.classes.as_slice() {
                self.class_map
                    .entry(class_atom.clone())
                    .or_default()
                    .push(node_id);
            }
        }
    }

    /// Desindexa um nó removido.
    pub fn unindex_node(&mut self, node_id: NodeId, node: &NodeData) {
        if let Some(el) = node.as_element() {
            if let Some(ref id_atom) = el.id_attr {
                if self.id_map.get(id_atom) == Some(&node_id) {
                    self.id_map.remove(id_atom);
                }
            }
            for class_atom in el.classes.as_slice() {
                if let Some(vec) = self.class_map.get_mut(class_atom) {
                    vec.retain(|&id| id != node_id);
                }
            }
        }
    }

    /// Obtém o `NodeId` correspondente a um ID HTML em $O(1)$.
    #[inline]
    pub fn get_by_id(&self, id: &str) -> Option<NodeId> {
        let atom = Atom::new(id);
        self.id_map.get(&atom).copied()
    }

    /// Obtém todos os `NodeId`s que possuem a classe especificada.
    #[inline]
    pub fn get_by_class(&self, class_name: &str) -> &[NodeId] {
        let atom = Atom::new(class_name);
        self.class_map.get(&atom).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

use crate::query::selector::{ComplexSelector, SimpleSelector};

/// Índice de regras de estilo particionado por baldes (Blink/Gecko RuleIndex pattern).
///
/// Filtra instantaneamente mais de 95% dos seletores que não possuem correspondência
/// potencial com o elemento atual antes de disparar o matching RTL.
#[derive(Debug, Clone, Default)]
pub struct RuleBucketIndex<T: Clone> {
    pub id_rules: FxHashMap<Atom, Vec<(ComplexSelector, T)>>,
    pub class_rules: FxHashMap<Atom, Vec<(ComplexSelector, T)>>,
    pub tag_rules: FxHashMap<Atom, Vec<(ComplexSelector, T)>>,
    pub universal_rules: Vec<(ComplexSelector, T)>,
}

impl<T: Clone> RuleBucketIndex<T> {
    pub fn new() -> Self {
        Self {
            id_rules: FxHashMap::default(),
            class_rules: FxHashMap::default(),
            tag_rules: FxHashMap::default(),
            universal_rules: Vec::new(),
        }
    }

    /// Adiciona um seletor e seu payload ao balde mais específico baseado no Key Selector.
    pub fn add_rule(&mut self, selector: ComplexSelector, payload: T) {
        if let Some((key_compound, _)) = selector.parts.last() {
            // 1. Balde de ID (mais específico)
            if let Some(id_atom) = key_compound.simple_selectors.iter().find_map(|s| {
                if let SimpleSelector::Id(id) = s {
                    Some(id)
                } else {
                    None
                }
            }) {
                self.id_rules
                    .entry(id_atom.clone())
                    .or_default()
                    .push((selector, payload));
                return;
            }

            // 2. Balde de Classe
            if let Some(class_atom) = key_compound.simple_selectors.iter().find_map(|s| {
                if let SimpleSelector::Class(c) = s {
                    Some(c)
                } else {
                    None
                }
            }) {
                self.class_rules
                    .entry(class_atom.clone())
                    .or_default()
                    .push((selector, payload));
                return;
            }

            // 3. Balde de Tag
            if let Some(tag_atom) = key_compound.simple_selectors.iter().find_map(|s| {
                if let SimpleSelector::Tag(t) = s {
                    Some(t)
                } else {
                    None
                }
            }) {
                self.tag_rules
                    .entry(tag_atom.clone())
                    .or_default()
                    .push((selector, payload));
                return;
            }
        }

        // 4. Balde Universal
        self.universal_rules.push((selector, payload));
    }

    /// Coleta todas as regras candidatas para um determinado elemento do DOM.
    pub fn match_candidates(&self, doc: &Document, node_id: NodeId, out: &mut Vec<T>) {
        let node = match doc.get_node(node_id) {
            Some(n) => n,
            None => return,
        };
        let el = match node.as_element() {
            Some(e) => e,
            None => return,
        };

        // 1. Consulta regras de ID
        if let Some(ref id_atom) = el.id_attr {
            if let Some(rules) = self.id_rules.get(id_atom) {
                for (sel, payload) in rules {
                    if sel.matches(doc, node_id) {
                        out.push(payload.clone());
                    }
                }
            }
        }

        // 2. Consulta regras de Classe
        for class_atom in el.classes.as_slice() {
            if let Some(rules) = self.class_rules.get(class_atom) {
                for (sel, payload) in rules {
                    if sel.matches(doc, node_id) {
                        out.push(payload.clone());
                    }
                }
            }
        }

        // 3. Consulta regras de Tag
        if let Some(rules) = self.tag_rules.get(&el.tag_name) {
            for (sel, payload) in rules {
                if sel.matches(doc, node_id) {
                    out.push(payload.clone());
                }
            }
        }

        // 4. Consulta regras Universais
        for (sel, payload) in &self.universal_rules {
            if sel.matches(doc, node_id) {
                out.push(payload.clone());
            }
        }
    }
}
