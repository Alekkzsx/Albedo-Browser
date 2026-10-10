//! # Albedo Core Engine — ace_style
//!
//! Motor de Estilo, Cascata e Render Tree do Albedo Browser (Fase 6).
//!
//! ## Submódulos e Frentes
//! - [`values`]: Tipos primitivos, comprimentos (`Length`, `CalcExpr`), cores (`CssColor`) e enums.
//! - [`parser`]: Tokenizer SIMD L3, valores de componentes e parser de declarações.
//! - [`model`]: Folhas de estilo (`StyleSheet`), `@media` e `@layer`.
//! - [`user_agent`]: Folha de estilo padrão do navegador (WHATWG HTML §15).
//! - [`rule_tree`]: Indexação em baldes (`RuleBucketMap`) e casamento com `AncestorFilter`.
//! - [`cascade`]: Motor de cascata das 8 origens normativas, `@layer` e especificidade.
//! - [`computed`]: Estilo computado (`ComputedStyle`), herança e DAG de variáveis `var()`.
//! - [`engine`]: Orquestrador `StyleEngine`, `StyleSharingCache` e `RenderTree`.

pub mod cascade;
pub mod computed;
pub mod engine;
pub mod model;
pub mod parser;
pub mod rule_tree;
pub mod user_agent;
pub mod values;

// Re-exports estratégicos de uso frequente
pub use cascade::{CascadeOrigin, CascadeSorter, StyleSheetOrigin};
pub use computed::{inherit_from_parent, is_inherited_property, ComputedStyle, VariableResolver};
pub use engine::{RenderTree, RenderTreeNode, StyleEngine, StyleSharingCache};
pub use model::{CSSRule, LayerBlockRule, LayerRegistry, MediaContext, MediaQuery, MediaRule, StyleRule, StyleSheet};
pub use parser::{parse_declarations, ParsedDeclaration, Token, Tokenizer};
pub use user_agent::default_user_agent_stylesheet;
pub use values::{Color, CssColor, Display, Length, Position};
