//! # Camadas de Cascata CSS (`@layer`) conforme CSS Cascade Level 5
//!
//! Representação de regras `@layer` e registro global de precedência de camadas.

use crate::model::stylesheet::CSSRule;
use ace_core::intern::Atom;
use rustc_hash::FxHashMap;

/// Regra de bloco de camada `@layer <name> { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerBlockRule {
    pub name: Atom,
    pub rules: Vec<CSSRule>,
}

/// Regra de declaração de precedência de camadas `@layer name1, name2, name3;`.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerStatementRule {
    pub names: Vec<Atom>,
}

/// Registro e resolvedor de ordem de camadas de cascata.
///
/// No fluxo normal do CSS Cascade 5:
/// Camadas declaradas mais tarde vencem camadas declaradas mais cedo (maior ordem vence).
/// Regras fora de qualquer camada (unlayered) vencem todas as regras em camadas.
///
/// No fluxo `!important`:
/// A ordem é invertida! Camadas declaradas mais cedo vencem camadas declaradas mais tarde!
/// E regras dentro de camadas vencem regras unlayered!
#[derive(Debug, Clone, Default)]
pub struct LayerRegistry {
    layer_orders: FxHashMap<Atom, u32>,
    next_order: u32,
}

impl LayerRegistry {
    pub fn new() -> Self {
        Self {
            layer_orders: FxHashMap::default(),
            next_order: 1, // 0 reservado para unlayered
        }
    }

    /// Registra uma lista de camadas declaradas em uma instrução `@layer a, b, c;`.
    pub fn register_statement(&mut self, names: &[Atom]) {
        for name in names {
            self.register_layer(name);
        }
    }

    /// Registra uma camada se ela ainda não tiver ordem definida. Retorna o número de ordem.
    pub fn register_layer(&mut self, name: &Atom) -> u32 {
        if let Some(&order) = self.layer_orders.get(name) {
            order
        } else {
            let order = self.next_order;
            self.next_order += 1;
            self.layer_orders.insert(name.clone(), order);
            order
        }
    }

    /// Cria uma nova camada anônima com ordem exclusiva.
    pub fn register_anonymous(&mut self) -> (Atom, u32) {
        let order = self.next_order;
        self.next_order += 1;
        let name = Atom::new(&format!("__anon_layer_{}", order));
        self.layer_orders.insert(name.clone(), order);
        (name, order)
    }

    /// Retorna a ordem da camada se ela existir.
    pub fn get_layer_order(&self, name: &Atom) -> Option<u32> {
        self.layer_orders.get(name).copied()
    }
}
