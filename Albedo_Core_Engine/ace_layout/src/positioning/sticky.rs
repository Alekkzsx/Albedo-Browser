//! # Posicionamento Fixo e Elástico (`position: sticky`)
//!
//! Implementação da restrição e cálculo de deslocamento 2D conforme
//! a especificação W3C CSS Positioned Layout Module Level 3 §6.

use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{LogicalPoint, LogicalRect, LogicalSides, LogicalSize};
use ace_style::computed::ComputedStyle;
use ace_style::values::enums::Overflow;
use euclid::default::Vector2D;

use crate::tree::layout_box::LayoutBox;

/// Insets especificados para posicionamento sticky (`top`, `right`, `bottom`, `left`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StickyInsets {
    pub top: Option<LayoutUnit>,
    pub right: Option<LayoutUnit>,
    pub bottom: Option<LayoutUnit>,
    pub left: Option<LayoutUnit>,
}

impl StickyInsets {
    /// Extrai os insets sticky diretamente do estilo computado.
    #[inline]
    pub fn from_style(style: &ComputedStyle) -> Self {
        Self {
            top: style.top(),
            right: style.right(),
            bottom: style.bottom(),
            left: style.left(),
        }
    }

    /// Retorna `true` se nenhum inset foi especificado (nenhum deslocamento a calcular).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.top.is_none() && self.right.is_none() && self.bottom.is_none() && self.left.is_none()
    }
}

/// Representa a janela visível de um contêiner de rolagem (Scrollport).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scrollport {
    /// Retângulo visível do contêiner de scroll no espaço do seu conteúdo.
    pub rect: LogicalRect<LayoutUnit>,
}

impl Scrollport {
    /// Cria um scrollport a partir de um retângulo lógico.
    #[inline]
    pub const fn new(rect: LogicalRect<LayoutUnit>) -> Self {
        Self { rect }
    }

    /// Cria um scrollport a partir da dimensão da janela e do offset de rolagem.
    #[inline]
    pub fn from_viewport(
        size: LogicalSize<LayoutUnit>,
        scroll_offset: LogicalPoint<LayoutUnit>,
    ) -> Self {
        Self {
            rect: LogicalRect::new(
                scroll_offset.inline,
                scroll_offset.block,
                size.inline_size,
                size.block_size,
            ),
        }
    }

    #[inline]
    pub fn inline_start(&self) -> LayoutUnit {
        self.rect.inline_start
    }

    #[inline]
    pub fn inline_end(&self) -> LayoutUnit {
        self.rect.inline_start.saturating_add(self.rect.inline_size)
    }

    #[inline]
    pub fn block_start(&self) -> LayoutUnit {
        self.rect.block_start
    }

    #[inline]
    pub fn block_end(&self) -> LayoutUnit {
        self.rect.block_start.saturating_add(self.rect.block_size)
    }
}

/// Posicionador determinístico de elementos com `position: sticky`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickyConstraintPositioner {
    /// Retângulo de borda natural do elemento no fluxo normal.
    pub natural_rect: LogicalRect<LayoutUnit>,
    /// Retângulo do scrollport ativo no mesmo espaço de coordenadas.
    pub scrollport_rect: LogicalRect<LayoutUnit>,
    /// Retângulo do bloco contêiner (containing block) no mesmo espaço de coordenadas.
    pub containing_block_rect: LogicalRect<LayoutUnit>,
    /// Margens do elemento sticky.
    pub margins: LogicalSides<LayoutUnit>,
    /// Insets declarados.
    pub insets: StickyInsets,
}

impl StickyConstraintPositioner {
    /// Cria uma nova instância do posicionador sticky com margens explícitas.
    pub fn new(
        natural_rect: LogicalRect<LayoutUnit>,
        scrollport: &Scrollport,
        containing_block: &LogicalRect<LayoutUnit>,
        margins: LogicalSides<LayoutUnit>,
        insets: StickyInsets,
    ) -> Self {
        Self {
            natural_rect,
            scrollport_rect: scrollport.rect,
            containing_block_rect: *containing_block,
            margins,
            insets,
        }
    }

    /// Calcula o vetor 2D de deslocamento sticky.
    pub fn compute_offset(&self) -> Vector2D<LayoutUnit> {
        let (delta_x, delta_y) = Self::compute_sticky_displacement(
            self.natural_rect,
            &self.insets,
            self.scrollport_rect,
            self.containing_block_rect,
            self.margins,
        );
        Vector2D::new(delta_x, delta_y)
    }

    /// Método de interface canônico conforme PROJECT.md.
    #[inline]
    pub fn compute_sticky_offset(
        natural_rect: LogicalRect<LayoutUnit>,
        insets: &StickyInsets,
        scrollport: &Scrollport,
        containing_block: &LogicalRect<LayoutUnit>,
    ) -> Vector2D<LayoutUnit> {
        let (delta_x, delta_y) = Self::compute_sticky_displacement(
            natural_rect,
            insets,
            scrollport.rect,
            *containing_block,
            LogicalSides::default(),
        );
        Vector2D::new(delta_x, delta_y)
    }

    /// Cálculo central com clamping aritmético exato em ponto fixo base-60 (`LayoutUnit`).
    fn compute_sticky_displacement(
        nat: LogicalRect<LayoutUnit>,
        insets: &StickyInsets,
        s_rect: LogicalRect<LayoutUnit>,
        c_rect: LogicalRect<LayoutUnit>,
        margins: LogicalSides<LayoutUnit>,
    ) -> (LayoutUnit, LayoutUnit) {
        let w = nat.inline_size;
        let h = nat.block_size;

        let s_top = s_rect.block_start;
        let s_bottom = s_rect.block_start.saturating_add(s_rect.block_size);
        let s_left = s_rect.inline_start;
        let s_right = s_rect.inline_start.saturating_add(s_rect.inline_size);

        let c_top = c_rect.block_start;
        let c_bottom = c_rect.block_start.saturating_add(c_rect.block_size);
        let c_left = c_rect.inline_start;
        let c_right = c_rect.inline_start.saturating_add(c_rect.inline_size);

        // --- Eixo Vertical (Block) ---
        let delta_y = if let Some(top_inset) = insets.top {
            let y_nat = nat.block_start;
            let y_target = s_top.saturating_add(top_inset);
            let y_max = c_bottom.saturating_sub(h).saturating_sub(margins.block_end);

            let y_final = if y_max < y_nat {
                y_nat
            } else {
                y_nat.max(y_target.min(y_max))
            };
            y_final.saturating_sub(y_nat)
        } else if let Some(bottom_inset) = insets.bottom {
            let y_nat = nat.block_start;
            let y_target = s_bottom.saturating_sub(bottom_inset).saturating_sub(h);
            let y_min = c_top.saturating_add(margins.block_start);

            let y_final = y_nat.min(y_target.max(y_min));
            y_final.saturating_sub(y_nat)
        } else {
            LayoutUnit::ZERO
        };

        // --- Eixo Horizontal (Inline) ---
        let delta_x = if let Some(left_inset) = insets.left {
            let x_nat = nat.inline_start;
            let x_target = s_left.saturating_add(left_inset);
            let x_max = c_right.saturating_sub(w).saturating_sub(margins.inline_end);

            let x_final = if x_max < x_nat {
                x_nat
            } else {
                x_nat.max(x_target.min(x_max))
            };
            x_final.saturating_sub(x_nat)
        } else if let Some(right_inset) = insets.right {
            let x_nat = nat.inline_start;
            let x_target = s_right.saturating_sub(right_inset).saturating_sub(w);
            let x_min = c_left.saturating_add(margins.inline_start);

            let x_final = x_nat.min(x_target.max(x_min));
            x_final.saturating_sub(x_nat)
        } else {
            LayoutUnit::ZERO
        };

        (delta_x, delta_y)
    }
}

/// Determina se uma caixa de layout estabelece um scrollport de rolagem.
#[inline]
pub fn establishes_scrollport(node: &LayoutBox) -> bool {
    let ox = node.style.overflow_x();
    let oy = node.style.overflow_y();
    ox == Overflow::Scroll || ox == Overflow::Auto || oy == Overflow::Scroll || oy == Overflow::Auto
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sticky_three_phase_lifecycle() {
        let natural_rect = LogicalRect::new(
            LayoutUnit::ZERO,
            LayoutUnit::from_px(200),
            LayoutUnit::from_px(100),
            LayoutUnit::from_px(50),
        );
        let containing_block = LogicalRect::new(
            LayoutUnit::ZERO,
            LayoutUnit::ZERO,
            LayoutUnit::from_px(800),
            LayoutUnit::from_px(1000),
        );
        let insets = StickyInsets {
            top: Some(LayoutUnit::from_px(20)),
            right: None,
            bottom: None,
            left: None,
        };

        // Fase 1: Pré-gatilho (scroll = 0). Alvo = 0 + 20 = 20 < 200 (Y_nat).
        // Deve permanecer em fluxo normal (deslocamento = 0).
        let scrollport1 = Scrollport::from_viewport(
            LogicalSize::new(LayoutUnit::from_px(800), LayoutUnit::from_px(600)),
            LogicalPoint::new(LayoutUnit::ZERO, LayoutUnit::ZERO),
        );
        let pos1 = StickyConstraintPositioner::new(
            natural_rect,
            &scrollport1,
            &containing_block,
            LogicalSides::default(),
            insets,
        );
        assert_eq!(
            pos1.compute_offset(),
            Vector2D::new(LayoutUnit::ZERO, LayoutUnit::ZERO)
        );

        // Fase 2: Rastreamento ativo (scroll = 300). Alvo = 300 + 20 = 320.
        // Deslocamento = 320 - 200 = 120.
        let scrollport2 = Scrollport::from_viewport(
            LogicalSize::new(LayoutUnit::from_px(800), LayoutUnit::from_px(600)),
            LogicalPoint::new(LayoutUnit::ZERO, LayoutUnit::from_px(300)),
        );
        let pos2 = StickyConstraintPositioner::new(
            natural_rect,
            &scrollport2,
            &containing_block,
            LogicalSides::default(),
            insets,
        );
        assert_eq!(
            pos2.compute_offset(),
            Vector2D::new(LayoutUnit::ZERO, LayoutUnit::from_px(120))
        );

        // Fase 3: Bloqueio no fundo do containing block (scroll = 2000).
        // Containing block bottom = 1000. Altura = 50. Max allowed = 950.
        // Deslocamento = 950 - 200 = 750.
        let scrollport3 = Scrollport::from_viewport(
            LogicalSize::new(LayoutUnit::from_px(800), LayoutUnit::from_px(600)),
            LogicalPoint::new(LayoutUnit::ZERO, LayoutUnit::from_px(2000)),
        );
        let pos3 = StickyConstraintPositioner::new(
            natural_rect,
            &scrollport3,
            &containing_block,
            LogicalSides::default(),
            insets,
        );
        assert_eq!(
            pos3.compute_offset(),
            Vector2D::new(LayoutUnit::ZERO, LayoutUnit::from_px(750))
        );
    }

    #[test]
    fn test_sticky_bottom_positioning() {
        let natural_rect = LogicalRect::new(
            LayoutUnit::ZERO,
            LayoutUnit::from_px(600),
            LayoutUnit::from_px(100),
            LayoutUnit::from_px(50),
        );
        let containing_block = LogicalRect::new(
            LayoutUnit::ZERO,
            LayoutUnit::ZERO,
            LayoutUnit::from_px(800),
            LayoutUnit::from_px(1000),
        );
        let insets = StickyInsets {
            top: None,
            right: None,
            bottom: Some(LayoutUnit::from_px(10)),
            left: None,
        };

        // Scrollport de altura 400 em scroll_y = 100 -> bottom = 500.
        // Alvo = 500 - 10 - 50 = 440.
        // Y_final = min(600, max(440, 0)) = 440.
        // Delta = 440 - 600 = -160.
        let scrollport = Scrollport::from_viewport(
            LogicalSize::new(LayoutUnit::from_px(800), LayoutUnit::from_px(400)),
            LogicalPoint::new(LayoutUnit::ZERO, LayoutUnit::from_px(100)),
        );
        let pos = StickyConstraintPositioner::new(
            natural_rect,
            &scrollport,
            &containing_block,
            LogicalSides::default(),
            insets,
        );
        assert_eq!(
            pos.compute_offset(),
            Vector2D::new(LayoutUnit::ZERO, LayoutUnit::from_px(-160))
        );
    }
}
