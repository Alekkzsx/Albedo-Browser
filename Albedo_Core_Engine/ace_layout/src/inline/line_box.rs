//! # Caixas de Linha e Alinhamento Inline (`LineBoxFragment`)
//!
//! Montagem de fragmentos de linha física, alinhamento vertical por baseline (`vertical-align`),
//! alinhamento horizontal (`text-align`: left, center, right, justify) e inversão de coordenadas BiDi/RTL.

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{Direction, LogicalRect};
use ace_style::computed::structs::StyleFont;
use ace_style::values::enums::TextAlign;
use smol_str::SmolStr;

/// Alinhamento vertical do fragmento inline na caixa de linha (CSS 2.2 §10.8.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VerticalAlign {
    /// Alinha a linha de base do fragmento com a baseline da caixa de linha.
    #[default]
    Baseline,
    /// Alinha o topo do fragmento com o topo da caixa de linha.
    Top,
    /// Alinha o centro vertical do fragmento com a metade da altura da linha.
    Middle,
    /// Alinha o fundo do fragmento com a base inferior da caixa de linha.
    Bottom,
}

/// Fragmento inline participante de uma caixa de linha.
#[derive(Debug, Clone, PartialEq)]
pub enum InlineFragment {
    /// Segmento de texto formatado.
    Text {
        text: SmolStr,
        font: StyleFont,
        bounds: LogicalRect<LayoutUnit>,
        ascent: LayoutUnit,
        descent: LayoutUnit,
        vertical_align: VerticalAlign,
    },
    /// Caixa inline-block ou elemento inline atômico.
    Box {
        id: NodeId,
        bounds: LogicalRect<LayoutUnit>,
        baseline: LayoutUnit,
        ascent: LayoutUnit,
        descent: LayoutUnit,
        vertical_align: VerticalAlign,
    },
}

impl InlineFragment {
    /// Retorna os limites lógicos do fragmento.
    #[inline]
    pub fn bounds(&self) -> &LogicalRect<LayoutUnit> {
        match self {
            Self::Text { bounds, .. } => bounds,
            Self::Box { bounds, .. } => bounds,
        }
    }

    /// Retorna referência mutável aos limites lógicos do fragmento.
    #[inline]
    pub fn bounds_mut(&mut self) -> &mut LogicalRect<LayoutUnit> {
        match self {
            Self::Text { bounds, .. } => bounds,
            Self::Box { bounds, .. } => bounds,
        }
    }

    /// Retorna o alinhamento vertical configurado para este fragmento.
    #[inline]
    pub fn vertical_align(&self) -> VerticalAlign {
        match self {
            Self::Text { vertical_align, .. } => *vertical_align,
            Self::Box { vertical_align, .. } => *vertical_align,
        }
    }

    /// Retorna o ascent vertical do fragmento.
    #[inline]
    pub fn ascent(&self) -> LayoutUnit {
        match self {
            Self::Text { ascent, .. } => *ascent,
            Self::Box { ascent, .. } => *ascent,
        }
    }

    /// Retorna o descent vertical do fragmento.
    #[inline]
    pub fn descent(&self) -> LayoutUnit {
        match self {
            Self::Text { descent, .. } => *descent,
            Self::Box { descent, .. } => *descent,
        }
    }
}

/// Caixa de linha física contendo uma sequência horizontal de fragmentos inline formatados.
#[derive(Debug, Clone, PartialEq)]
pub struct LineBoxFragment {
    /// Limites geométricos da caixa de linha no contêiner de bloco.
    pub bounds: LogicalRect<LayoutUnit>,
    /// Posição da linha de base medida a partir do topo da caixa de linha.
    pub baseline: LayoutUnit,
    /// Fragmentos inline contidos nesta linha.
    pub fragments: Vec<InlineFragment>,
}

impl LineBoxFragment {
    /// Cria uma nova caixa de linha vazia na posição vertical especificada.
    pub fn new(block_start: LayoutUnit) -> Self {
        Self {
            bounds: LogicalRect::new(
                LayoutUnit::ZERO,
                block_start,
                LayoutUnit::ZERO,
                LayoutUnit::ZERO,
            ),
            baseline: LayoutUnit::ZERO,
            fragments: Vec::new(),
        }
    }

    /// Adiciona um fragmento inline à caixa de linha.
    pub fn add_fragment(&mut self, fragment: InlineFragment) {
        self.fragments.push(fragment);
    }

    /// Executa o cálculo normativo de alinhamento vertical por baseline (CSS 2.2 §10.8).
    ///
    /// Determina a baseline da linha como o maior `ascent` dos fragmentos com `vertical-align: baseline`,
    /// ajusta a altura total da linha e posiciona cada fragmento no eixo vertical (block).
    pub fn apply_baseline_alignment(&mut self) {
        if self.fragments.is_empty() {
            return;
        }

        // 1. Determina o maior ascent e descent entre os fragmentos alinhados por baseline
        let mut max_ascent = LayoutUnit::ZERO;
        let mut max_descent = LayoutUnit::ZERO;
        let mut max_fragment_height = LayoutUnit::ZERO;

        for frag in &self.fragments {
            let asc = frag.ascent();
            let desc = frag.descent();
            let h = frag.bounds().block_size;

            max_fragment_height = max_fragment_height.max(h);

            if frag.vertical_align() == VerticalAlign::Baseline {
                max_ascent = max_ascent.max(asc);
                max_descent = max_descent.max(desc);
            }
        }

        // Se nenhum fragmento for explicitamente Baseline, usa os maiores gerais
        if max_ascent == LayoutUnit::ZERO {
            for frag in &self.fragments {
                max_ascent = max_ascent.max(frag.ascent());
                max_descent = max_descent.max(frag.descent());
            }
        }

        let line_height = (max_ascent.saturating_add(max_descent)).max(max_fragment_height);
        self.baseline = max_ascent;
        self.bounds.block_size = line_height;

        // 2. Posiciona cada fragmento verticalmente
        for frag in &mut self.fragments {
            let val = frag.vertical_align();
            let h = frag.bounds().block_size;
            let asc = frag.ascent();

            let y = match val {
                VerticalAlign::Baseline => self.baseline.saturating_sub(asc),
                VerticalAlign::Top => LayoutUnit::ZERO,
                VerticalAlign::Bottom => line_height.saturating_sub(h),
                VerticalAlign::Middle => {
                    // Centro alinhado com a baseline menos metade do x-height estimado (ascent / 4)
                    let mid_line = self
                        .baseline
                        .saturating_sub(self.baseline.saturating_div(4));
                    mid_line
                        .saturating_sub(h.saturating_div(2))
                        .max(LayoutUnit::ZERO)
                }
            };

            frag.bounds_mut().block_start = y;
        }
    }

    /// Executa o alinhamento horizontal (`text-align`) e a distribuição de espaços (`justify`).
    pub fn apply_horizontal_alignment(
        &mut self,
        container_width: LayoutUnit,
        text_align: TextAlign,
        direction: Direction,
        is_last_line: bool,
    ) {
        self.apply_horizontal_alignment_with_inset(
            LayoutUnit::ZERO,
            container_width,
            text_align,
            direction,
            is_last_line,
        );
    }

    /// Executa o alinhamento horizontal com respeito a um inset lateral (`inline_start_inset`)
    /// proveniente de elementos flutuantes no `ExclusionSpace`.
    pub fn apply_horizontal_alignment_with_inset(
        &mut self,
        inline_start_inset: LayoutUnit,
        available_width: LayoutUnit,
        text_align: TextAlign,
        direction: Direction,
        is_last_line: bool,
    ) {
        if self.fragments.is_empty() {
            return;
        }

        // Calcula a largura total ocupada pelos fragmentos
        let total_content_width = self.fragments.iter().fold(LayoutUnit::ZERO, |acc, f| {
            acc.saturating_add(f.bounds().inline_size)
        });

        self.bounds.inline_start = inline_start_inset;
        self.bounds.inline_size = total_content_width;
        let free_space = available_width.saturating_sub(total_content_width);

        if free_space <= LayoutUnit::ZERO {
            // Sem espaço livre para redistribuição; posiciona sequencialmente
            let mut curr_x = inline_start_inset;
            for frag in &mut self.fragments {
                frag.bounds_mut().inline_start = curr_x;
                curr_x = curr_x.saturating_add(frag.bounds().inline_size);
            }
            if direction == Direction::Rtl {
                self.invert_rtl_offsets_with_inset(inline_start_inset, available_width);
            }
            return;
        }

        let base_offset = match text_align {
            TextAlign::Left | TextAlign::Start => LayoutUnit::ZERO,
            TextAlign::Right | TextAlign::End => free_space,
            TextAlign::Center => free_space.saturating_div(2),
            TextAlign::Justify => {
                if !is_last_line && self.fragments.len() > 1 {
                    let gaps_count = (self.fragments.len() - 1) as i32;
                    let gap_addition = LayoutUnit::from_raw(free_space.raw() / gaps_count);
                    let mut remainder_raw = free_space.raw() % gaps_count;

                    let mut curr_x = inline_start_inset;
                    for frag in &mut self.fragments {
                        frag.bounds_mut().inline_start = curr_x;
                        let extra = if remainder_raw > 0 {
                            remainder_raw -= 1;
                            LayoutUnit::from_raw(1)
                        } else {
                            LayoutUnit::ZERO
                        };
                        curr_x = curr_x
                            .saturating_add(frag.bounds().inline_size)
                            .saturating_add(gap_addition)
                            .saturating_add(extra);
                    }
                    self.bounds.inline_size = available_width;
                    if direction == Direction::Rtl {
                        self.invert_rtl_offsets_with_inset(inline_start_inset, available_width);
                    }
                    return;
                } else {
                    LayoutUnit::ZERO
                }
            }
        };

        let mut curr_x = inline_start_inset.saturating_add(base_offset);
        for frag in &mut self.fragments {
            frag.bounds_mut().inline_start = curr_x;
            curr_x = curr_x.saturating_add(frag.bounds().inline_size);
        }

        if direction == Direction::Rtl {
            self.invert_rtl_offsets_with_inset(inline_start_inset, available_width);
        }
    }

    /// Inverte os offsets horizontais para parágrafos com escrita da direita para a esquerda (`Rtl`).
    ///
    /// Fórmula de inversão de coordenadas:
    /// $$x_{\text{rtl}} = \text{container\_width} - x_{\text{ltr}} - \text{fragment\_width}$$
    pub fn invert_rtl_offsets(&mut self, container_width: LayoutUnit) {
        self.invert_rtl_offsets_with_inset(LayoutUnit::ZERO, container_width);
    }

    /// Inverte os offsets horizontais para parágrafos RTL respeitando o inset horizontal.
    pub fn invert_rtl_offsets_with_inset(
        &mut self,
        inline_start_inset: LayoutUnit,
        available_width: LayoutUnit,
    ) {
        for frag in &mut self.fragments {
            let orig_rel = frag.bounds().inline_start.saturating_sub(inline_start_inset);
            let w = frag.bounds().inline_size;
            let inverted_rel = available_width
                .saturating_sub(orig_rel)
                .saturating_sub(w)
                .max(LayoutUnit::ZERO);
            frag.bounds_mut().inline_start = inline_start_inset.saturating_add(inverted_rel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_text_fragment(
        text: &str,
        w: i32,
        h: i32,
        ascent: i32,
        descent: i32,
        align: VerticalAlign,
    ) -> InlineFragment {
        InlineFragment::Text {
            text: SmolStr::new(text),
            font: StyleFont::initial(),
            bounds: LogicalRect::new(
                LayoutUnit::ZERO,
                LayoutUnit::ZERO,
                LayoutUnit::from_px(w),
                LayoutUnit::from_px(h),
            ),
            ascent: LayoutUnit::from_px(ascent),
            descent: LayoutUnit::from_px(descent),
            vertical_align: align,
        }
    }

    #[test]
    fn test_line_box_baseline_alignment() {
        let mut line = LineBoxFragment::new(LayoutUnit::ZERO);
        // Fragment 1: 12px ascent, 4px descent (16px total)
        line.add_fragment(mock_text_fragment(
            "hello",
            40,
            16,
            12,
            4,
            VerticalAlign::Baseline,
        ));
        // Fragment 2: maior fonte: 18px ascent, 6px descent (24px total)
        line.add_fragment(mock_text_fragment(
            "WORLD",
            60,
            24,
            18,
            6,
            VerticalAlign::Baseline,
        ));

        line.apply_baseline_alignment();

        // Linha deve ter baseline = 18px e altura total = 18 + 6 = 24px
        assert_eq!(line.baseline, LayoutUnit::from_px(18));
        assert_eq!(line.bounds.block_size, LayoutUnit::from_px(24));

        // Fragment 1 deve ter block_start = 18 - 12 = 6px
        assert_eq!(
            line.fragments[0].bounds().block_start,
            LayoutUnit::from_px(6)
        );
        // Fragment 2 deve ter block_start = 18 - 18 = 0px
        assert_eq!(line.fragments[1].bounds().block_start, LayoutUnit::ZERO);
    }

    #[test]
    fn test_line_box_top_and_bottom_alignment() {
        let mut line = LineBoxFragment::new(LayoutUnit::ZERO);
        line.add_fragment(mock_text_fragment(
            "base",
            30,
            20,
            15,
            5,
            VerticalAlign::Baseline,
        ));
        line.add_fragment(mock_text_fragment("top", 20, 10, 8, 2, VerticalAlign::Top));
        line.add_fragment(mock_text_fragment(
            "bottom",
            20,
            8,
            6,
            2,
            VerticalAlign::Bottom,
        ));

        line.apply_baseline_alignment();

        // Top fragmento deve estar em y = 0
        assert_eq!(line.fragments[1].bounds().block_start, LayoutUnit::ZERO);
        // Bottom fragmento deve estar em y = line_height - 8 = 20 - 8 = 12px
        assert_eq!(
            line.fragments[2].bounds().block_start,
            LayoutUnit::from_px(12)
        );
    }

    #[test]
    fn test_horizontal_alignment_center_and_right() {
        let mut line = LineBoxFragment::new(LayoutUnit::ZERO);
        line.add_fragment(mock_text_fragment(
            "abc",
            40,
            16,
            12,
            4,
            VerticalAlign::Baseline,
        ));
        line.apply_baseline_alignment();

        let container_w = LayoutUnit::from_px(100);

        // Center: 100 - 40 = 60 free space -> 30px offset
        line.apply_horizontal_alignment(container_w, TextAlign::Center, Direction::Ltr, false);
        assert_eq!(
            line.fragments[0].bounds().inline_start,
            LayoutUnit::from_px(30)
        );

        // Right: 60px offset
        line.apply_horizontal_alignment(container_w, TextAlign::Right, Direction::Ltr, false);
        assert_eq!(
            line.fragments[0].bounds().inline_start,
            LayoutUnit::from_px(60)
        );
    }

    #[test]
    fn test_horizontal_alignment_justify() {
        let mut line = LineBoxFragment::new(LayoutUnit::ZERO);
        line.add_fragment(mock_text_fragment(
            "one",
            20,
            16,
            12,
            4,
            VerticalAlign::Baseline,
        ));
        line.add_fragment(mock_text_fragment(
            "two",
            20,
            16,
            12,
            4,
            VerticalAlign::Baseline,
        ));
        line.add_fragment(mock_text_fragment(
            "three",
            20,
            16,
            12,
            4,
            VerticalAlign::Baseline,
        ));
        line.apply_baseline_alignment();

        // Total content = 60px. Container = 100px. Free = 40px. 2 gaps -> 20px por gap
        let container_w = LayoutUnit::from_px(100);
        line.apply_horizontal_alignment(container_w, TextAlign::Justify, Direction::Ltr, false);

        assert_eq!(
            line.fragments[0].bounds().inline_start,
            LayoutUnit::from_px(0)
        );
        assert_eq!(
            line.fragments[1].bounds().inline_start,
            LayoutUnit::from_px(40)
        ); // 20 + 20
        assert_eq!(
            line.fragments[2].bounds().inline_start,
            LayoutUnit::from_px(80)
        ); // 40 + 20 + 20
    }

    #[test]
    fn test_rtl_offset_inversion() {
        let mut line = LineBoxFragment::new(LayoutUnit::ZERO);
        line.add_fragment(mock_text_fragment(
            "rtl",
            30,
            16,
            12,
            4,
            VerticalAlign::Baseline,
        ));
        line.apply_baseline_alignment();

        let container_w = LayoutUnit::from_px(100);
        // Em LTR Left: inline_start = 0. Ao inverter para RTL: 100 - 0 - 30 = 70px
        line.apply_horizontal_alignment(container_w, TextAlign::Left, Direction::Rtl, false);
        assert_eq!(
            line.fragments[0].bounds().inline_start,
            LayoutUnit::from_px(70)
        );
    }

    #[test]
    fn test_middle_alignment() {
        let mut line = LineBoxFragment::new(LayoutUnit::ZERO);
        line.add_fragment(mock_text_fragment(
            "base",
            40,
            24,
            16,
            8,
            VerticalAlign::Baseline,
        ));
        line.add_fragment(mock_text_fragment(
            "mid",
            30,
            10,
            8,
            2,
            VerticalAlign::Middle,
        ));
        line.apply_baseline_alignment();

        // Baseline = 16px, line_height = 24px.
        // mid_line = baseline - baseline / 4 = 16 - 4 = 12px
        // y = mid_line - 10 / 2 = 12 - 5 = 7px
        assert_eq!(
            line.fragments[1].bounds().block_start,
            LayoutUnit::from_px(7)
        );
    }

    #[test]
    fn test_text_align_start_end_rtl() {
        let mut line = LineBoxFragment::new(LayoutUnit::ZERO);
        line.add_fragment(mock_text_fragment(
            "text",
            40,
            16,
            12,
            4,
            VerticalAlign::Baseline,
        ));
        line.apply_baseline_alignment();

        let container_w = LayoutUnit::from_px(100);

        // Em RTL, TextAlign::Start alinha à direita: 100 - 40 = 60px
        line.apply_horizontal_alignment(container_w, TextAlign::Start, Direction::Rtl, false);
        assert_eq!(
            line.fragments[0].bounds().inline_start,
            LayoutUnit::from_px(60)
        );

        // Em RTL, TextAlign::End alinha à esquerda: 0px
        line.apply_horizontal_alignment(container_w, TextAlign::End, Direction::Rtl, false);
        assert_eq!(
            line.fragments[0].bounds().inline_start,
            LayoutUnit::from_px(0)
        );
    }
}
