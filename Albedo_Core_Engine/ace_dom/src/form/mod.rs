//! # Motor de Formulários e Validação de Restrições (WHATWG HTML §4.10)
//!
//! Fornece estruturas de controle, estados de validade e serialização de dados de formulário.

pub mod form_data;
pub mod validity;

pub use form_data::{FormData, FormDataEntry, FormDataValue};
pub use validity::ValidityState;

use crate::tree::Document;
use ace_core::id::NodeId;

/// Representa a associação de controles a um formulário `<form>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormAssociation {
    /// O ID do elemento `<form>`.
    pub form_id: NodeId,
    /// Os IDs dos controles pertencentes a este formulário (`<input>`, `<select>`, etc.).
    pub elements: Vec<NodeId>,
}

/// Avalia as restrições de validação de um elemento de controle no documento (WHATWG HTML §4.10.21).
pub fn check_control_validity(doc: &Document, node_id: NodeId) -> ValidityState {
    let mut state = ValidityState::default();
    let node = match doc.get_node(node_id) {
        Some(n) => n,
        None => return state,
    };

    let el = match node.as_element() {
        Some(e) => e,
        None => return state,
    };

    if let Some(custom) = el.get_attribute("data-custom-validity") {
        if !custom.is_empty() {
            state.set_custom_validity(custom);
        }
    }

    let is_required = el.has_attribute("required");
    let val = el.get_attribute("value").unwrap_or_default();

    // 1. Checagem de obrigatoriedade
    if is_required && val.trim().is_empty() {
        state.value_missing = true;
    }

    // 2. Checagem de tamanho máximo/mínimo
    if let Some(max_str) = el.get_attribute("maxlength") {
        if let Ok(max_len) = max_str.parse::<usize>() {
            if val.chars().count() > max_len {
                state.too_long = true;
            }
        }
    }

    if let Some(min_str) = el.get_attribute("minlength") {
        if let Ok(min_len) = min_str.parse::<usize>() {
            if !val.is_empty() && val.chars().count() < min_len {
                state.too_short = true;
            }
        }
    }

    // 3. Checagem de tipo (e-mail, url, number)
    let type_attr = el.get_attribute("type").unwrap_or("text").to_ascii_lowercase();
    if type_attr == "email" && !val.is_empty() {
        if !val.contains('@') || !val.contains('.') {
            state.type_mismatch = true;
        }
    } else if type_attr == "url" && !val.is_empty() {
        if !val.starts_with("http://") && !val.starts_with("https://") {
            state.type_mismatch = true;
        }
    } else if type_attr == "number" && !val.is_empty() {
        if let Ok(num) = val.parse::<f64>() {
            if let Some(min_val) = el.get_attribute("min").and_then(|m| m.parse::<f64>().ok()) {
                if num < min_val {
                    state.range_underflow = true;
                }
            }
            if let Some(max_val) = el.get_attribute("max").and_then(|m| m.parse::<f64>().ok()) {
                if num > max_val {
                    state.range_overflow = true;
                }
            }
            if let Some(step_val) = el.get_attribute("step").and_then(|s| s.parse::<f64>().ok()) {
                if step_val > 0.0 {
                    let min_base = el.get_attribute("min").and_then(|m| m.parse::<f64>().ok()).unwrap_or(0.0);
                    let diff = (num - min_base).abs();
                    let remainder = diff % step_val;
                    if remainder > 1e-5 && (step_val - remainder) > 1e-5 {
                        state.step_mismatch = true;
                    }
                }
            }
        } else {
            state.bad_input = true;
        }
    }

    // 4. Checagem de Pattern
    if let Some(pattern_str) = el.get_attribute("pattern") {
        if !val.is_empty() && !matches_simple_pattern(pattern_str, val) {
            state.pattern_mismatch = true;
        }
    }

    state
}

fn matches_simple_pattern(pattern: &str, text: &str) -> bool {
    if pattern == "[0-9]+" || pattern == "\\d+" {
        return text.chars().all(|c| c.is_ascii_digit());
    }
    if pattern == "[a-zA-Z]+" {
        return text.chars().all(|c| c.is_ascii_alphabetic());
    }
    if let Some(inner) = pattern.strip_prefix('^').and_then(|p| p.strip_suffix('$')) {
        return text == inner;
    }
    text.contains(pattern)
}
