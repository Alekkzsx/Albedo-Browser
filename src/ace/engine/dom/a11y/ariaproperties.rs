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


/// Propriedades ARIA
#[derive(Clone, Debug, Default)]
pub struct AriaProperties {
    pub label: Option<String>,              // aria-label
    pub labelledby: Vec<String>,            // aria-labelledby
    pub describedby: Vec<String>,           // aria-describedby
    pub details: Option<String>,            // aria-details
    pub errormessage: Option<String>,       // aria-errormessage
    pub controls: Vec<String>,              // aria-controls
    pub owns: Vec<String>,                  // aria-owns
    pub flowto: Vec<String>,                // aria-flowto
    pub activedescendant: Option<String>,   // aria-activedescendant
    pub posinset: Option<i32>,              // aria-posinset
    pub setsize: Option<i32>,               // aria-setsize
    pub level: Option<i32>,                 // aria-level
    pub valuenow: Option<f64>,              // aria-valuenow
    pub valuemin: Option<f64>,              // aria-valuemin
    pub valuemax: Option<f64>,              // aria-valuemax
    pub valuetext: Option<String>,          // aria-valuetext
    pub roledescription: Option<String>,    // aria-roledescription
}
