//! # Estilos Computados e Resolução de Cascata (`ace_style::computed`)
//!
//! `ComputedStyle` modular com style structs, tabela de herança formal, resolução de variáveis e keywords universais.

pub mod inherit;
pub mod resolver;
pub mod structs;
pub mod style;
pub mod variables;

pub use inherit::{inherit_from_parent, is_inherited_property};
pub use resolver::resolve_computed_style;
pub use structs::{
    StyleBox, StyleColor, StyleFlex, StyleFont, StyleGrid, StyleSurrounding, StyleText,
    INITIAL_BOX, INITIAL_COLOR, INITIAL_FLEX, INITIAL_FONT, INITIAL_GRID, INITIAL_SURROUNDING,
    INITIAL_TEXT,
};
pub use style::ComputedStyle;
pub use variables::{VariableResolver, MAX_EXPANDED_TOKEN_LEN, MAX_SUBSTITUTION_DEPTH};
