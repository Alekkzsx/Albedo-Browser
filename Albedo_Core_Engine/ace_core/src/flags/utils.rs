//! # Utilitários para Manipulação de Flags
//!
//! Funções auxiliares para propagação de sujeira (dirty flags) e mapeamento de impacto de estilos.

use super::{NodeFlags, StyleChangeHint};

/// Retorna `true` se o nó necessitar de qualquer tipo de reparo/reprocessamento (estilo, layout ou pintura).
#[inline]
pub fn is_node_dirty(flags: NodeFlags) -> bool {
    flags.intersects(
        NodeFlags::DIRTY_STYLE
            | NodeFlags::DIRTY_LAYOUT
            | NodeFlags::DIRTY_PAINT
            | NodeFlags::SUBTREE_DIRTY,
    )
}

/// Mapeia uma alteração de estilo em flags de nó correspondentes.
pub fn style_hint_to_node_flags(hint: StyleChangeHint) -> NodeFlags {
    let mut flags = NodeFlags::empty();
    if hint.contains(StyleChangeHint::REPAINT) {
        flags |= NodeFlags::DIRTY_PAINT;
    }
    if hint.contains(StyleChangeHint::REFLOW_LAYOUT) {
        flags |= NodeFlags::DIRTY_LAYOUT | NodeFlags::DIRTY_PAINT;
    }
    if hint.contains(StyleChangeHint::RECALC_STYLE)
        || hint.contains(StyleChangeHint::RECONSTRUCT_FRAME)
    {
        flags |= NodeFlags::DIRTY_STYLE | NodeFlags::DIRTY_LAYOUT | NodeFlags::DIRTY_PAINT;
    }
    if hint.contains(StyleChangeHint::SUBTREE_RECALC) {
        flags |= NodeFlags::SUBTREE_DIRTY | NodeFlags::DIRTY_STYLE;
    }
    flags
}
