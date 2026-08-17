//! # Conjuntos Tipados de Flags para Invalidação e Renderização
//!
//! Flags binárias de alta performance para rastreamento de nós DOM, dicas de invalidação de estilo e compositor.

use bitflags::bitflags;

bitflags! {
    /// Flags de estado e sujeira (dirty tracking) de nós da árvore DOM.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct NodeFlags: u32 {
        /// O nó é um Elemento HTML/SVG.
        const IS_ELEMENT       = 1 << 0;
        /// O nó é um nó de Texto.
        const IS_TEXT          = 1 << 1;
        /// O nó é um Comentário.
        const IS_COMMENT       = 1 << 2;
        /// O nó é a Raiz do Documento.
        const IS_DOCUMENT      = 1 << 3;
        /// O nó está conectado à árvore ativa do documento.
        const IS_CONNECTED     = 1 << 4;
        /// O nó precisa de recalculo de estilos CSS (`recalc_style`).
        const DIRTY_STYLE      = 1 << 5;
        /// O nó precisa de novo cálculo geométrico de layout (`reflow`).
        const DIRTY_LAYOUT     = 1 << 6;
        /// O nó precisa de repintura na tela (`repaint`).
        const DIRTY_PAINT      = 1 << 7;
        /// Pelo menos um descendente deste nó está marcado como sujo.
        const SUBTREE_DIRTY    = 1 << 8;
    }
}

bitflags! {
    /// Dica granular sobre a extensão da invalidação causada por uma alteração de estilo CSS.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct StyleChangeHint: u16 {
        /// Nenhuma alteração necessária.
        const NONE               = 0;
        /// Requer apenas repintura dos pixels (ex: mudança de `color`, `background-color`).
        const REPAINT            = 1 << 0;
        /// Requer recalcular estilos computados.
        const RECALC_STYLE       = 1 << 1;
        /// Requer novo passe completo de layout/geometria (ex: mudança de `width`, `display`, `margin`).
        const REFLOW_LAYOUT      = 1 << 2;
        /// Requer reconstruir a caixa de renderização na Render Tree.
        const RECONSTRUCT_FRAME  = 1 << 3;
        /// A alteração afeta as propriedades herdadas de toda a subárvore.
        const SUBTREE_RECALC     = 1 << 4;
    }
}

bitflags! {
    /// Flags de propriedades gráficas e camadas do compositor GPU (`wgpu`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct RenderFlags: u16 {
        /// A camada ou caixa está visível na tela.
        const VISIBLE             = 1 << 0;
        /// Possui matriz de transformação 2D/3D ativa (`transform`).
        const HAS_TRANSFORM       = 1 << 1;
        /// Possui transparência parcial (`opacity < 1.0`).
        const HAS_OPACITY         = 1 << 2;
        /// Possui recorte retangular ou máscara (`overflow: hidden`, `clip-path`).
        const HAS_CLIP            = 1 << 3;
        /// É um container de rolagem (`overflow: scroll` / `auto`).
        const IS_SCROLL_CONTAINER = 1 << 4;
        /// Promovido a camada compositada independente na GPU para aceleração por hardware.
        const COMPOSITED_LAYER    = 1 << 5;
    }
}
