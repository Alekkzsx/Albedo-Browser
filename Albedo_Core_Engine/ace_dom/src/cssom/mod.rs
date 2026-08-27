//! # CSSOM (CSS Object Model - WHATWG CSSOM Standard)
//!
//! Representação de folhas de estilo (`CSSStyleSheet`), coleções de declarações (`CSSStyleDeclaration`),
//! propriedades atômicas (`CSSProperty`), contexto de mídia (`MediaContext`) e resolução de cascata (`StyleResolver` / `ComputedStyle`).

pub mod computed;
pub mod declaration;
pub mod media;
pub mod stylesheet;

pub use computed::{ComputedStyle, StyleCache, StyleResolver};
pub use declaration::{CSSProperty, CSSStyleDeclaration};
pub use media::{ColorScheme, MediaContext, MediaType, Orientation};
pub use stylesheet::{CSSRule, CSSStyleRule, CSSStyleSheet};
