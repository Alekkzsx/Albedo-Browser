//! # Layout Constraint Space (`ConstraintSpace`)
//!
//! Encapsulates sizing constraints, available sizes, percentage resolution bases,
//! and pass modes passed from parent layout boxes to child layout boxes.

use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{Direction, LogicalSize, WritingMode};

/// Representação do espaço disponível em um determinado eixo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailableSize {
    /// Tamanho fixo e determinado pelo contêiner pai.
    Definite(LayoutUnit),
    /// Tamanho indefinido (por exemplo, altura automática em fluxo ou contêiner irrestrito).
    Indefinite,
    /// Restrição para determinação do tamanho intrínseco mínimo (`min-content`).
    MinContent,
    /// Restrição para determinação do tamanho intrínseco máximo (`max-content`).
    MaxContent,
}

impl AvailableSize {
    /// Converte para um valor definido opcional em `LayoutUnit`.
    #[inline]
    pub fn to_definite(self) -> Option<LayoutUnit> {
        match self {
            Self::Definite(u) => Some(u),
            _ => None,
        }
    }

    /// Retorna `true` se o tamanho disponível for definitivo.
    #[inline]
    pub fn is_definite(self) -> bool {
        matches!(self, Self::Definite(_))
    }

    /// Retorna `true` se o tamanho disponível for indefinido.
    #[inline]
    pub fn is_indefinite(self) -> bool {
        matches!(self, Self::Indefinite)
    }

    /// Retorna `true` se a restrição for `min-content`.
    #[inline]
    pub fn is_min_content(self) -> bool {
        matches!(self, Self::MinContent)
    }

    /// Retorna `true` se a restrição for `max-content`.
    #[inline]
    pub fn is_max_content(self) -> bool {
        matches!(self, Self::MaxContent)
    }

    /// Constrói um `AvailableSize` a partir de um `Option<LayoutUnit>`.
    #[inline]
    pub fn from_option(opt: Option<LayoutUnit>) -> Self {
        match opt {
            Some(u) => Self::Definite(u),
            None => Self::Indefinite,
        }
    }

    /// Converte em `Option<LayoutUnit>`.
    #[inline]
    pub fn into_option(self) -> Option<LayoutUnit> {
        self.to_definite()
    }
}

/// Restrições espaciais e contextuais imutáveis passadas do contêiner pai para a caixa filha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintSpace {
    /// Espaço disponível no eixo inline.
    pub available_inline_size: AvailableSize,
    /// Espaço disponível no eixo block.
    pub available_block_size: AvailableSize,
    /// Tamanho base para resolução de percentagens CSS (largura e altura do contêiner pai).
    pub percentage_resolution_size: LogicalSize<LayoutUnit>,
    /// Indica se a passada corrente é de medição hipotética (`Measure`) ou definitiva (`Definitive`).
    pub is_measure_pass: bool,
    /// Modo de escrita do fluxo corrente.
    pub writing_mode: WritingMode,
    /// Direção base de leitura (LTR ou RTL).
    pub direction: Direction,
    /// Indica se o nó estabelece um novo contexto de formatação independente.
    pub is_new_fc: bool,
}

impl ConstraintSpace {
    /// Cria um novo `ConstraintSpace` padrão (passada definitiva, LTR, HorizontalTb).
    #[inline]
    pub fn new(
        available_inline_size: AvailableSize,
        available_block_size: AvailableSize,
        percentage_resolution_size: LogicalSize<LayoutUnit>,
    ) -> Self {
        Self {
            available_inline_size,
            available_block_size,
            percentage_resolution_size,
            is_measure_pass: false,
            writing_mode: WritingMode::HorizontalTb,
            direction: Direction::Ltr,
            is_new_fc: false,
        }
    }

    /// Cria um novo `ConstraintSpace` especificando explicitamente o modo de passada.
    #[inline]
    pub fn new_with_pass(
        available_inline_size: AvailableSize,
        available_block_size: AvailableSize,
        percentage_resolution_size: LogicalSize<LayoutUnit>,
        is_measure_pass: bool,
    ) -> Self {
        Self {
            available_inline_size,
            available_block_size,
            percentage_resolution_size,
            is_measure_pass,
            writing_mode: WritingMode::HorizontalTb,
            direction: Direction::Ltr,
            is_new_fc: false,
        }
    }

    /// Cria um `ConstraintSpace` a partir de dimensões físicas/lógicas de contêiner.
    #[inline]
    pub fn for_container(size: LogicalSize<LayoutUnit>) -> Self {
        Self {
            available_inline_size: AvailableSize::Definite(size.inline_size),
            available_block_size: AvailableSize::Definite(size.block_size),
            percentage_resolution_size: size,
            is_measure_pass: false,
            writing_mode: WritingMode::HorizontalTb,
            direction: Direction::Ltr,
            is_new_fc: false,
        }
    }

    /// Modificador para modo de escrita.
    #[inline]
    pub fn with_writing_mode(mut self, wm: WritingMode) -> Self {
        self.writing_mode = wm;
        self
    }

    /// Modificador para direção de texto.
    #[inline]
    pub fn with_direction(mut self, dir: Direction) -> Self {
        self.direction = dir;
        self
    }

    /// Modificador para flag de novo contexto de formatação.
    #[inline]
    pub fn with_is_new_fc(mut self, is_new: bool) -> Self {
        self.is_new_fc = is_new;
        self
    }

    /// Modificador para flag de passada de medição.
    #[inline]
    pub fn with_measure_pass(mut self, is_measure: bool) -> Self {
        self.is_measure_pass = is_measure;
        self
    }

    /// Retorna se a passada é puramente de medição hipotética.
    #[inline]
    pub fn is_measure_pass(&self) -> bool {
        self.is_measure_pass
    }

    /// Retorna se a passada é definitiva (produzirá fragmentos finais).
    #[inline]
    pub fn is_definitive_pass(&self) -> bool {
        !self.is_measure_pass
    }

    /// Retorna o espaço disponível no eixo inline.
    #[inline]
    pub fn available_inline_size(&self) -> AvailableSize {
        self.available_inline_size
    }

    /// Retorna o espaço disponível no eixo block.
    #[inline]
    pub fn available_block_size(&self) -> AvailableSize {
        self.available_block_size
    }

    /// Retorna as dimensões base para resolução de percentagens.
    #[inline]
    pub fn percentage_resolution_size(&self) -> LogicalSize<LayoutUnit> {
        self.percentage_resolution_size
    }

    /// Retorna o modo de escrita.
    #[inline]
    pub fn writing_mode(&self) -> WritingMode {
        self.writing_mode
    }

    /// Retorna a direção de texto.
    #[inline]
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Retorna uma nova instância com os tamanhos disponíveis reduzidos por margens/bordas/padding.
    pub fn shrink_by_insets(
        &self,
        inline_insets: LayoutUnit,
        block_insets: LayoutUnit,
    ) -> Self {
        let new_inline = match self.available_inline_size {
            AvailableSize::Definite(size) => {
                let remaining = size.saturating_sub(inline_insets);
                AvailableSize::Definite(remaining.max(LayoutUnit::ZERO))
            }
            other => other,
        };

        let new_block = match self.available_block_size {
            AvailableSize::Definite(size) => {
                let remaining = size.saturating_sub(block_insets);
                AvailableSize::Definite(remaining.max(LayoutUnit::ZERO))
            }
            other => other,
        };

        Self {
            available_inline_size: new_inline,
            available_block_size: new_block,
            percentage_resolution_size: self.percentage_resolution_size,
            is_measure_pass: self.is_measure_pass,
            writing_mode: self.writing_mode,
            direction: self.direction,
            is_new_fc: self.is_new_fc,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_size_variants() {
        let def = AvailableSize::Definite(LayoutUnit::from_px(400));
        assert!(def.is_definite());
        assert_eq!(def.to_definite(), Some(LayoutUnit::from_px(400)));

        let indef = AvailableSize::Indefinite;
        assert!(indef.is_indefinite());
        assert_eq!(indef.to_definite(), None);

        let min_c = AvailableSize::MinContent;
        assert!(min_c.is_min_content());

        let max_c = AvailableSize::MaxContent;
        assert!(max_c.is_max_content());
    }

    #[test]
    fn test_constraint_space_creation_and_shrinking() {
        let container_size = LogicalSize {
            inline_size: LayoutUnit::from_px(800),
            block_size: LayoutUnit::from_px(600),
        };

        let cs = ConstraintSpace::for_container(container_size)
            .with_direction(Direction::Rtl)
            .with_is_new_fc(true);

        assert_eq!(
            cs.available_inline_size,
            AvailableSize::Definite(LayoutUnit::from_px(800))
        );
        assert_eq!(cs.direction, Direction::Rtl);
        assert!(cs.is_new_fc);
        assert!(!cs.is_measure_pass());
        assert!(cs.is_definitive_pass());

        let shrunken = cs.shrink_by_insets(LayoutUnit::from_px(40), LayoutUnit::from_px(20));
        assert_eq!(
            shrunken.available_inline_size,
            AvailableSize::Definite(LayoutUnit::from_px(760))
        );
        assert_eq!(
            shrunken.available_block_size,
            AvailableSize::Definite(LayoutUnit::from_px(580))
        );
    }

    #[test]
    fn test_constraint_space_measure_pass_and_writing_mode() {
        let size = LogicalSize {
            inline_size: LayoutUnit::from_px(500),
            block_size: LayoutUnit::from_px(300),
        };
        let cs = ConstraintSpace::new_with_pass(
            AvailableSize::Definite(size.inline_size),
            AvailableSize::Indefinite,
            size,
            true,
        )
        .with_writing_mode(WritingMode::VerticalRl);

        assert!(cs.is_measure_pass());
        assert!(!cs.is_definitive_pass());
        assert_eq!(cs.writing_mode(), WritingMode::VerticalRl);
        assert_eq!(cs.available_block_size(), AvailableSize::Indefinite);
    }

    #[test]
    fn test_available_size_conversions_and_saturation() {
        let opt_some = AvailableSize::from_option(Some(LayoutUnit::from_px(150)));
        assert_eq!(opt_some, AvailableSize::Definite(LayoutUnit::from_px(150)));
        assert_eq!(opt_some.into_option(), Some(LayoutUnit::from_px(150)));

        let opt_none = AvailableSize::from_option(None);
        assert_eq!(opt_none, AvailableSize::Indefinite);
        assert_eq!(opt_none.into_option(), None);

        let size = LogicalSize {
            inline_size: LayoutUnit::from_px(50),
            block_size: LayoutUnit::from_px(50),
        };
        let cs = ConstraintSpace::for_container(size);
        let overshrunk = cs.shrink_by_insets(LayoutUnit::from_px(100), LayoutUnit::from_px(100));
        assert_eq!(
            overshrunk.available_inline_size,
            AvailableSize::Definite(LayoutUnit::ZERO)
        );
        assert_eq!(
            overshrunk.available_block_size,
            AvailableSize::Definite(LayoutUnit::ZERO)
        );
    }
}
