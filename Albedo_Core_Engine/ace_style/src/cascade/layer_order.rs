//! # Resolução de Precedência de Camadas de Cascata (`@layer`)
//!
//! Algoritmo normativo de ordenação de camadas conforme CSS Cascading and Inheritance Level 5 §5.3.

use std::cmp::Ordering;

/// Compara a precedência entre duas camadas para uma declaração CSS.
///
/// Parâmetros:
/// - `layer_a`: Ordem da primeira camada (`None` se for unlayered / fora de camada)
/// - `layer_b`: Ordem da segunda camada (`None` se for unlayered)
/// - `is_important`: Se as declarações em disputa possuem o modificador `!important`
///
/// Regras normativas da W3C:
/// - Em declarações normais:
///   1. Regras unlayered (`None`) vencem regras em camadas (`Some`).
///   2. Entre camadas, a camada declarada depois vence (maior ordem numérica vence).
///
/// - Em declarações `!important`:
///   1. Regras em camadas (`Some`) vencem regras unlayered (`None`).
///   2. Entre camadas, a camada declarada antes vence (menor ordem numérica vence).
pub fn compare_layer_precedence(
    layer_a: Option<u32>,
    layer_b: Option<u32>,
    is_important: bool,
) -> Ordering {
    if !is_important {
        // Fluxo normal
        match (layer_a, layer_b) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater, // Unlayered vence layered
            (Some(_), None) => Ordering::Less,    // Layered perde para unlayered
            (Some(a), Some(b)) => a.cmp(&b),      // Maior ordem vence
        }
    } else {
        // Fluxo !important (ordem invertida)
        match (layer_a, layer_b) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,    // Unlayered perde para layered em !important
            (Some(_), None) => Ordering::Greater, // Layered vence unlayered em !important
            (Some(a), Some(b)) => b.cmp(&a),      // Menor ordem vence (invertido!)
        }
    }
}
