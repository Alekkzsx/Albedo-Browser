//! # Estilos Computados e Resolução de Cascata (`ace_style::computed`)
//!
//! `ComputedStyle` denso, tabela de herança formal, resolução de variáveis e keywords universais.

pub mod inherit;
pub mod resolver;
pub mod style;
pub mod variables;

pub use inherit::{inherit_from_parent, is_inherited_property};
pub use resolver::resolve_computed_style;
pub use style::ComputedStyle;
pub use variables::{VariableResolver, MAX_EXPANDED_TOKEN_LEN, MAX_SUBSTITUTION_DEPTH};
