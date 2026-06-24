use super::*;
//! Accessibility Tree (ARIA 1.2) Implementation - W3C WAI-ARIA Spec
//! 
//! Este módulo implementa:
//! - Mapeamento implícito de roles HTML → ARIA
//! - Accessible Name Computation (AccName 1.2)
//! - States & Properties ARIA
//! - Relations (aria-controls, aria-owns, etc.)
//! - Tree traversal para screen readers

use std::collections::HashMap;
use crate::ace::engine::dom::{AceDOM, AceNodeType};

/// Role ARIA de um elemento


/// Estados ARIA
#[derive(Clone, Debug, Default)]
pub struct AriaStates {
    pub checked: Option<bool>,          // aria-checked
    pub selected: Option<bool>,         // aria-selected
    pub pressed: Option<bool>,          // aria-pressed
    pub expanded: Option<bool>,         // aria-expanded
    pub hidden: bool,                   // aria-hidden
    pub disabled: bool,                 // aria-disabled
    pub invalid: Option<bool>,          // aria-invalid
    pub readonly: Option<bool>,         // aria-readonly
    pub required: Option<bool>,         // aria-required
    pub busy: Option<bool>,             // aria-busy
    pub live: Option<String>,           // aria-live (polite, assertive, off)
    pub atomic: Option<bool>,           // aria-atomic
    pub relevant: Option<String>,       // aria-relevant
}
