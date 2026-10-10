//! # Tabela Formal de Herança de Propriedades CSS (Inheritance Table)
//!
//! Separação normativa entre propriedades hereditárias e não-hereditárias.

use crate::computed::style::ComputedStyle;

/// Retorna `true` se a propriedade CSS for herdada por padrão pelo elemento filho.
pub fn is_inherited_property(property: &str) -> bool {
    if property.starts_with("--") {
        return true;
    }

    matches!(
        property,
        "color"
            | "font-size"
            | "font-weight"
            | "font-style"
            | "font-family"
            | "line-height"
            | "text-align"
            | "visibility"
            | "white-space"
            | "word-break"
            | "overflow-wrap"
    )
}

/// Aplica a herança de estilo do elemento pai para um novo estilo computado.
pub fn inherit_from_parent(style: &mut ComputedStyle, parent: &ComputedStyle) {
    // 1. Tipografia e Cores
    style.color = parent.color;
    style.font_size = parent.font_size;
    style.font_weight = parent.font_weight;
    style.font_style = parent.font_style;
    style.font_family = parent.font_family.clone();
    style.line_height = parent.line_height;
    style.text_align = parent.text_align;
    style.visibility = parent.visibility;
    style.white_space = parent.white_space;
    style.word_break = parent.word_break;
    style.overflow_wrap = parent.overflow_wrap;

    // 2. Custom Properties (todas as variáveis CSS herdam por padrão)
    style.custom_properties = parent.custom_properties.clone();
}
