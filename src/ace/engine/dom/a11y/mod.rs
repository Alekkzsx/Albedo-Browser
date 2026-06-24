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

pub mod ariarole; pub use ariarole::*;
pub mod ariastates; pub use ariastates::*;
pub mod ariaproperties; pub use ariaproperties::*;
pub mod accessibilitynode; pub use accessibilitynode::*;
pub mod implicitrolemap_; pub use implicitrolemap_::*;
pub mod implicitrolemap; pub use implicitrolemap::*;
pub mod accessiblenamecomputer_; pub use accessiblenamecomputer_::*;
pub mod accessiblenamecomputer; pub use accessiblenamecomputer::*;
pub mod accessibilitytree; pub use accessibilitytree::*;
pub mod accessibilitytree_impl_1; pub use accessibilitytree_impl_1::*;
pub mod accessibilitytree_impl_2; pub use accessibilitytree_impl_2::*;
pub mod acedom; pub use acedom::*;
pub mod test_implicit_role_button; pub use test_implicit_role_button::*;
