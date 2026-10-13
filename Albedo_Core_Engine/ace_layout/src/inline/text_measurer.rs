//! # Desacoplamento Tipográfico (`TextMeasurer`)
//!
//! Define a abstração `TextMeasurer` para medição de texto e métricas de baseline,
//! permitindo layout 100% determinístico e independente de backends de SO ou GPU.

use ace_core::math::layout_unit::LayoutUnit;
use ace_style::computed::structs::StyleFont;

/// Métricas espaciais e verticais resultantes da medição de texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextMetrics {
    /// Largura total ocupada pelo texto no eixo inline.
    pub width: LayoutUnit,
    /// Altura total ocupada pelo texto (altura da linha).
    pub height: LayoutUnit,
    /// Distância da linha de base até o topo da caixa de glifos.
    pub ascent: LayoutUnit,
    /// Distância da linha de base até o fundo da caixa de glifos.
    pub descent: LayoutUnit,
    /// Espaçamento entre linhas sugerido pela fonte.
    pub line_gap: LayoutUnit,
}

impl TextMetrics {
    /// Cria uma nova descrição de métricas textuais.
    #[inline]
    pub const fn new(
        width: LayoutUnit,
        height: LayoutUnit,
        ascent: LayoutUnit,
        descent: LayoutUnit,
        line_gap: LayoutUnit,
    ) -> Self {
        Self {
            width,
            height,
            ascent,
            descent,
            line_gap,
        }
    }

    /// Retorna a altura natural da linha baseada nas métricas verticais (`ascent + descent + line_gap`).
    #[inline]
    pub fn natural_line_height(&self) -> LayoutUnit {
        self.ascent
            .saturating_add(self.descent)
            .saturating_add(self.line_gap)
    }
}

/// Contrato para medição tipográfica de texto desacoplada de fontes físicas do sistema operacional.
pub trait TextMeasurer: Send + Sync {
    /// Mede as dimensões espaciais de um segmento de texto sob determinada fonte.
    fn measure_text(&self, text: &str, font: &StyleFont) -> TextMetrics;

    /// Retorna a distância da linha de base a partir do topo do fragmento de linha.
    fn baseline(&self, font: &StyleFont) -> LayoutUnit;
}

/// Medidor de texto em ponto fixo baseado em grade de caracteres (*Character-Cell Typography*).
///
/// Padrão: 8px de avanço por caractere, 12px de ascent, 4px de descent, 2px de line gap.
/// Linha total: 18px de altura (ou `font.line_height` se especificado).
#[derive(Debug, Clone)]
pub struct FixedTextMeasurer {
    /// Avanço horizontal de cada caractere.
    pub glyph_advance: LayoutUnit,
    /// Distância vertical da baseline até o topo.
    pub ascent: LayoutUnit,
    /// Distância vertical da baseline até a base.
    pub descent: LayoutUnit,
    /// Folga entre linhas recomendada.
    pub line_gap: LayoutUnit,
}

impl Default for FixedTextMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

impl FixedTextMeasurer {
    /// Cria um medidor monospaçado padrão (8px avanço, 12px ascent, 4px descent, 2px line gap).
    pub fn new() -> Self {
        Self {
            glyph_advance: LayoutUnit::from_px(8),
            ascent: LayoutUnit::from_px(12),
            descent: LayoutUnit::from_px(4),
            line_gap: LayoutUnit::from_px(2),
        }
    }

    /// Cria um medidor com avanço de glifo customizado.
    pub fn with_glyph_advance(glyph_advance: LayoutUnit) -> Self {
        Self {
            glyph_advance,
            ..Self::new()
        }
    }

    /// Cria um medidor com todas as métricas explicitadas.
    pub fn with_metrics(
        glyph_advance: LayoutUnit,
        ascent: LayoutUnit,
        descent: LayoutUnit,
        line_gap: LayoutUnit,
    ) -> Self {
        Self {
            glyph_advance,
            ascent,
            descent,
            line_gap,
        }
    }
}

impl TextMeasurer for FixedTextMeasurer {
    fn measure_text(&self, text: &str, font: &StyleFont) -> TextMetrics {
        let count = text.chars().count() as i32;
        let width = LayoutUnit::from_raw(self.glyph_advance.raw().saturating_mul(count));
        let natural_height = self
            .ascent
            .saturating_add(self.descent)
            .saturating_add(self.line_gap);
        let height = font.line_height.unwrap_or(natural_height);

        TextMetrics {
            width,
            height,
            ascent: self.ascent,
            descent: self.descent,
            line_gap: self.line_gap,
        }
    }

    fn baseline(&self, _font: &StyleFont) -> LayoutUnit {
        self.ascent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_text_measurer_defaults() {
        let measurer = FixedTextMeasurer::new();
        let font = StyleFont::initial();

        let metrics = measurer.measure_text("hello", &font);
        assert_eq!(metrics.width, LayoutUnit::from_px(40)); // 5 * 8px = 40px
        assert_eq!(metrics.height, LayoutUnit::from_px(18)); // 12 + 4 + 2 = 18px
        assert_eq!(metrics.ascent, LayoutUnit::from_px(12));
        assert_eq!(metrics.descent, LayoutUnit::from_px(4));
        assert_eq!(measurer.baseline(&font), LayoutUnit::from_px(12));
    }

    #[test]
    fn test_fixed_text_measurer_explicit_line_height() {
        let measurer = FixedTextMeasurer::new();
        let mut font = StyleFont::initial();
        font.line_height = Some(LayoutUnit::from_px(24));

        let metrics = measurer.measure_text("world", &font);
        assert_eq!(metrics.width, LayoutUnit::from_px(40));
        assert_eq!(metrics.height, LayoutUnit::from_px(24));
        assert_eq!(metrics.ascent, LayoutUnit::from_px(12));
    }
}
