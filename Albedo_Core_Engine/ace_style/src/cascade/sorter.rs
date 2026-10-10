//! # Desempate e Resolução Final da Cascata CSS (Cascade Sorter)
//!
//! Algoritmo de desempate final: Origem -> @layer -> Especificidade (a, b, c) -> Ordem no documento.

use crate::cascade::layer_order::compare_layer_precedence;
use crate::cascade::origin::CascadeOrigin;
use crate::model::layer::LayerRegistry;
use crate::rule_tree::matcher::MatchedRule;
use ace_core::intern::Atom;
use ace_dom::query::selector::Specificity;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::cmp::Ordering;

/// Representação de uma declaração individual na cascata com todos os metadados de precedência.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CascadeDeclaration {
    pub property: Atom,
    pub value: SmolStr,
    pub origin: CascadeOrigin,
    pub layer_order: Option<u32>,
    pub specificity: Specificity,
    pub sheet_index: usize,
    pub source_order: u32,
    pub is_inline: bool,
}

impl CascadeDeclaration {
    /// Compara a precedência de duas declarações concorrentes para a mesma propriedade.
    /// Retorna `Ordering::Greater` se `self` vencer `other`.
    pub fn compare(&self, other: &Self) -> Ordering {
        // 1. Origem e Importância (UserAgent normal .. Transition)
        let origin_cmp = self.origin.cmp(&other.origin);
        if origin_cmp != Ordering::Equal {
            return origin_cmp;
        }

        // 2. Inline Style (estilos inline normais vencem estilos de autor de folhas normais)
        if self.is_inline != other.is_inline {
            if self.origin.is_important() {
                // Em !important, inline vence regras de autor de folhas
                return if self.is_inline { Ordering::Greater } else { Ordering::Less };
            } else {
                return if self.is_inline { Ordering::Greater } else { Ordering::Less };
            }
        }

        // 3. Ordem de Camadas (@layer)
        let is_important = self.origin.is_important();
        let layer_cmp = compare_layer_precedence(self.layer_order, other.layer_order, is_important);
        if layer_cmp != Ordering::Equal {
            return layer_cmp;
        }

        // 4. Especificidade (a, b, c)
        let spec_cmp = self.specificity.cmp(&other.specificity);
        if spec_cmp != Ordering::Equal {
            return spec_cmp;
        }

        // 5. Ordem do documento (sheet_index e source_order dentro da folha)
        self.sheet_index
            .cmp(&other.sheet_index)
            .then_with(|| self.source_order.cmp(&other.source_order))
    }
}

/// Coletor e resolvedor de cascata para um elemento.
#[derive(Debug, Default)]
pub struct CascadeSorter {
    /// Mapa de propriedades vencendo a cascata: Propriedade -> Declaração Vencedora
    winning_declarations: FxHashMap<Atom, CascadeDeclaration>,
}

impl CascadeSorter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Limpa o estado para o próximo elemento.
    pub fn clear(&mut self) {
        self.winning_declarations.clear();
    }

    /// Processa todas as regras casadas e declarações inline para resolver os vencedores da cascata.
    pub fn resolve_cascade(
        &mut self,
        matched_rules: &[MatchedRule],
        inline_declarations: &[crate::parser::declaration_parser::ParsedDeclaration],
        layer_registry: &LayerRegistry,
    ) {
        self.clear();

        // 1. Adiciona declarações das regras casadas
        for rule in matched_rules {
            let layer_order = rule.layer.as_ref().and_then(|name| layer_registry.get_layer_order(name));

            for decl in &rule.declarations {
                let origin = CascadeOrigin::from_sheet_origin(rule.sheet_origin, decl.important);
                let candidate = CascadeDeclaration {
                    property: decl.property.clone(),
                    value: decl.value.clone(),
                    origin,
                    layer_order,
                    specificity: rule.specificity,
                    sheet_index: rule.sheet_index,
                    source_order: rule.source_order,
                    is_inline: false,
                };
                self.add_candidate(candidate);
            }
        }

        // 2. Adiciona declarações inline (style="...")
        for decl in inline_declarations {
            let origin = CascadeOrigin::from_sheet_origin(crate::cascade::origin::StyleSheetOrigin::Author, decl.important);
            // Inline style tem especificidade de (1, 0, 0, 0) conceitual — representamos com flag is_inline = true
            let candidate = CascadeDeclaration {
                property: decl.property.clone(),
                value: decl.value.clone(),
                origin,
                layer_order: None,
                specificity: Specificity::ZERO,
                sheet_index: usize::MAX,
                source_order: u32::MAX,
                is_inline: true,
            };
            self.add_candidate(candidate);
        }
    }

    fn add_candidate(&mut self, candidate: CascadeDeclaration) {
        match self.winning_declarations.get_mut(&candidate.property) {
            Some(existing) => {
                if candidate.compare(existing) == Ordering::Greater {
                    *existing = candidate;
                }
            }
            None => {
                self.winning_declarations.insert(candidate.property.clone(), candidate);
            }
        }
    }

    /// Retorna as declarações vencedoras.
    pub fn winning_declarations(&self) -> &FxHashMap<Atom, CascadeDeclaration> {
        &self.winning_declarations
    }
}
