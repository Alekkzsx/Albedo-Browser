//! # Tabela Formal de Herança de Propriedades CSS (Inheritance Table)
//!
//! Separação normativa entre propriedades hereditárias e não-hereditárias,
//! com clonagem de custo zero para ponteiros `Arc` de sub-estruturas herdadas.

use crate::computed::style::ComputedStyle;
use std::sync::Arc;

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
            | "direction"
            | "writing-mode"
    )
}

/// Aplica a herança de estilo do elemento pai para um novo estilo computado.
pub fn inherit_from_parent(style: &mut ComputedStyle, parent: &ComputedStyle) {
    // 1. Structs Herdadas: compartilhamento direto de instâncias via Arc
    style.font = Arc::clone(&parent.font);
    style.color = Arc::clone(&parent.color);
    style.text = Arc::clone(&parent.text);

    // 2. Custom Properties (todas as variáveis CSS herdam por padrão)
    style.custom_properties = parent.custom_properties.clone();
}
