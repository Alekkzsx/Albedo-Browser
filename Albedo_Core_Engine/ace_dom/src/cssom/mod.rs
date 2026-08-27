//! # CSSOM (CSS Object Model - WHATWG CSSOM Standard)
//!
//! Representação de folhas de estilo (`CSSStyleSheet`), coleções de declarações (`CSSStyleDeclaration`),
//! propriedades atômicas (`CSSProperty`) e resolução de cascata (`StyleResolver` / `ComputedStyle`).

pub mod computed;
pub mod declaration;
pub mod stylesheet;

pub use computed::{ComputedStyle, StyleResolver};
pub use declaration::{CSSProperty, CSSStyleDeclaration};
pub use stylesheet::{CSSRule, CSSStyleRule, CSSStyleSheet};
