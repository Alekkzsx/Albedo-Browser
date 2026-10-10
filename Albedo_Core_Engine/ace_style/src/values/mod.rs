//! # Valores Tipados e Primitivos do CSS (`ace_style::values`)
//!
//! Submódulo contendo tipos primitivos, comprimentos, cores, enums e custom properties.

pub mod color;
pub mod custom_prop;
pub mod enums;
pub mod length;

pub use color::{Color, CssColor};
pub use custom_prop::{CustomPropertyName, VarRef};
pub use enums::{
    AlignContent, AlignItems, AlignSelf, BorderStyle, BoxSizing, Display, FlexDirection, FlexWrap,
    FontStyle, FontWeight, GridAutoFlow, JustifyContent, Overflow, OverflowWrap, Position,
    RectSides, TextAlign, TextDecoration, Visibility, WhiteSpace, WordBreak, ZIndex,
};
pub use length::{CalcExpr, Length, LengthPercentage};
