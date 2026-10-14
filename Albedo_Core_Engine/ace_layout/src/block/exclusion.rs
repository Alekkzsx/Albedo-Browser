//! # Espaço de Exclusão e Geometria de Flutuações (`ExclusionSpace`)
//!
//! Implementação da geometria bidimensional de exclusões e do algoritmo de prateleiras (shelf model)
//! em tempo $O(F)$, conforme W3C CSS 2.2 §9.5 e Chromium Blink LayoutNG (`NGExclusionSpace`).

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::LogicalRect;
use ace_style::values::enums::{Clear, Float};

/// Categoria direcional de um elemento flutuante (CSS 2.2 §9.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FloatType {
    /// O elemento flutua à esquerda da linha.
    #[default]
    Left,
    /// O elemento flutua à direita da linha.
    Right,
}

impl FloatType {
    /// Converte de representação textual CSS.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }

    /// Converte do enum `Float` do `ace_style`.
    pub fn from_style(style_float: Float) -> Option<Self> {
        match style_float {
            Float::Left => Some(Self::Left),
            Float::Right => Some(Self::Right),
            Float::None => None,
        }
    }
}

/// Modo de desobstrução de flutuações (CSS 2.2 §9.5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ClearType {
    #[default]
    None,
    Left,
    Right,
    Both,
}

impl ClearType {
    /// Converte de representação textual CSS.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    /// Converte do enum `Clear` do `ace_style`.
    pub fn from_style(style_clear: Clear) -> Self {
        match style_clear {
            Clear::None => Self::None,
            Clear::Left => Self::Left,
            Clear::Right => Self::Right,
            Clear::Both => Self::Both,
        }
    }

    /// Retorna `true` se desobstrui floats à esquerda.
    #[inline]
    pub fn clears_left(&self) -> bool {
        matches!(self, Self::Left | Self::Both)
    }

    /// Retorna `true` se desobstrui floats à direita.
    #[inline]
    pub fn clears_right(&self) -> bool {
        matches!(self, Self::Right | Self::Both)
    }
}

/// Representa um obstáculo geométrico ativo (margin box de um float colocado no BFC).
#[derive(Debug, Clone, PartialEq)]
pub struct Exclusion {
    /// Retângulo da margem do float em coordenadas lógicas relativas à origem do BFC:
    /// `inline_start` (x), `block_start` (y), `inline_size` (largura total da margem), `block_size` (altura).
    pub rect: LogicalRect<LayoutUnit>,
    /// Direção do float (Left ou Right).
    pub kind: FloatType,
    /// Identificador do nó DOM que gerou o float.
    pub node_id: NodeId,
}

impl Exclusion {
    /// Cria uma nova exclusão com retângulo de margem e tipo.
    #[inline]
    pub const fn new(rect: LogicalRect<LayoutUnit>, kind: FloatType, node_id: NodeId) -> Self {
        Self { rect, kind, node_id }
    }

    /// Retorna a coordenada inferior do retângulo de margem (block_start + block_size).
    #[inline]
    pub fn block_end(&self) -> LayoutUnit {
        self.rect.block_start.saturating_add(self.rect.block_size)
    }

    /// Retorna a coordenada direita do retângulo de margem (inline_start + inline_size).
    #[inline]
    pub fn inline_end(&self) -> LayoutUnit {
        self.rect.inline_start.saturating_add(self.rect.inline_size)
    }
}

/// Oportunidade horizontal livre para colocação de caixas de linha ou novos floats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutOpportunity {
    /// Posição vertical inicial onde a prateleira horizontal livre se encontra.
    pub block_start: LayoutUnit,
    /// Altura vertical de validade desta oportunidade.
    pub block_size: LayoutUnit,
    /// Posição horizontal onde o espaço livre começa (após os floats à esquerda).
    pub inline_start: LayoutUnit,
    /// Largura horizontal disponível entre floats esquerdos e direitos nesta fatia vertical.
    pub inline_size: LayoutUnit,
}

impl LayoutOpportunity {
    /// Cria uma oportunidade geométrica livre.
    #[inline]
    pub const fn new(
        block_start: LayoutUnit,
        block_size: LayoutUnit,
        inline_start: LayoutUnit,
        inline_size: LayoutUnit,
    ) -> Self {
        Self {
            block_start,
            block_size,
            inline_start,
            inline_size,
        }
    }

    /// Verifica se a largura disponível atende a um requisito de largura mínima.
    #[inline]
    pub fn fits(&self, required_inline_size: LayoutUnit) -> bool {
        self.inline_size >= required_inline_size
    }

    /// Retorna a borda horizontal direita da oportunidade livre.
    #[inline]
    pub fn inline_end(&self) -> LayoutUnit {
        self.inline_start.saturating_add(self.inline_size)
    }
}

/// Coleção imutável/derivável de exclusões ativas em um BFC (`ExclusionSpace`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExclusionSpace {
    /// Lista de exclusões registradas no BFC.
    exclusions: Vec<Exclusion>,
    /// Menor cota inferior que ultrapassa todos os floats esquerdos ($O(1)$ lookup).
    max_left_bottom: LayoutUnit,
    /// Menor cota inferior que ultrapassa todos os floats direitos ($O(1)$ lookup).
    max_right_bottom: LayoutUnit,
}

impl ExclusionSpace {
    /// Cria um novo espaço de exclusão vazio.
    #[inline]
    pub const fn new() -> Self {
        Self {
            exclusions: Vec::new(),
            max_left_bottom: LayoutUnit::ZERO,
            max_right_bottom: LayoutUnit::ZERO,
        }
    }

    /// Retorna `true` se não houver exclusões ativas.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.exclusions.is_empty()
    }

    /// Retorna a lista de exclusões ativas.
    #[inline]
    pub fn exclusions(&self) -> &[Exclusion] {
        &self.exclusions
    }

    /// Adiciona uma nova exclusão ao espaço e atualiza as cotas de clearance em O(1).
    pub fn add_exclusion(&mut self, exclusion: Exclusion) {
        let bottom = exclusion.block_end();
        match exclusion.kind {
            FloatType::Left => {
                if bottom > self.max_left_bottom {
                    self.max_left_bottom = bottom;
                }
            }
            FloatType::Right => {
                if bottom > self.max_right_bottom {
                    self.max_right_bottom = bottom;
                }
            }
        }
        self.exclusions.push(exclusion);
    }

    /// Calcula em O(1) o offset vertical mínimo necessário para desobstrução (clearance).
    #[inline]
    pub fn clear_offset(&self, clear: ClearType) -> LayoutUnit {
        match clear {
            ClearType::None => LayoutUnit::ZERO,
            ClearType::Left => self.max_left_bottom,
            ClearType::Right => self.max_right_bottom,
            ClearType::Both => self.max_left_bottom.max(self.max_right_bottom),
        }
    }

    /// Localiza a oportunidade de layout livre mais alta em O(F) via algoritmo de prateleiras.
    pub fn find_layout_opportunity(
        &self,
        search_offset: LayoutUnit,
        available_inline_size: LayoutUnit,
        min_inline_size: LayoutUnit,
        block_size: LayoutUnit,
    ) -> LayoutOpportunity {
        if self.exclusions.is_empty() {
            return LayoutOpportunity::new(
                search_offset,
                block_size,
                LayoutUnit::ZERO,
                available_inline_size,
            );
        }

        let mut curr_y = search_offset;

        loop {
            let effective_height = if block_size > LayoutUnit::ZERO {
                block_size
            } else {
                LayoutUnit::from_raw(1)
            };
            let slice_end = curr_y.saturating_add(effective_height);
            let mut left_edge = LayoutUnit::ZERO;
            let mut right_edge = available_inline_size;
            let mut next_shelf: Option<LayoutUnit> = None;

            for e in &self.exclusions {
                let e_top = e.rect.block_start;
                let e_bottom = e.block_end();

                // Verifica se a exclusão intercepta a fatia [curr_y, slice_end]
                if e_top < slice_end && e_bottom > curr_y {
                    match e.kind {
                        FloatType::Left => {
                            let e_right = e.inline_end();
                            if e_right > left_edge {
                                left_edge = e_right;
                            }
                        }
                        FloatType::Right => {
                            let e_left = e.rect.inline_start;
                            if e_left < right_edge {
                                right_edge = e_left;
                            }
                        }
                    }
                }

                // Acompanha a menor prateleira estritamente maior que curr_y para saltar se não couber
                if e_bottom > curr_y {
                    next_shelf = match next_shelf {
                        Some(prev_min) => Some(prev_min.min(e_bottom)),
                        None => Some(e_bottom),
                    };
                }
            }

            // Garante que right_edge nunca seja menor que left_edge
            let available = right_edge.saturating_sub(left_edge).max(LayoutUnit::ZERO);

            if available >= min_inline_size {
                return LayoutOpportunity::new(curr_y, block_size, left_edge, available);
            }

            // Não cabe nesta fatia; avança para a próxima prateleira de término de float
            match next_shelf {
                Some(shelf) if shelf > curr_y => {
                    curr_y = shelf;
                }
                _ => {
                    // Ultrapassou todos os floats ativos; o espaço total do contêiner está liberado
                    return LayoutOpportunity::new(
                        curr_y,
                        block_size,
                        LayoutUnit::ZERO,
                        available_inline_size,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_exclusion_space() {
        let space = ExclusionSpace::new();
        assert!(space.is_empty());
        assert_eq!(space.clear_offset(ClearType::Both), LayoutUnit::ZERO);

        let opp = space.find_layout_opportunity(
            LayoutUnit::from_px(10),
            LayoutUnit::from_px(800),
            LayoutUnit::from_px(100),
            LayoutUnit::from_px(50),
        );
        assert_eq!(opp.block_start, LayoutUnit::from_px(10));
        assert_eq!(opp.inline_start, LayoutUnit::ZERO);
        assert_eq!(opp.inline_size, LayoutUnit::from_px(800));
        assert!(opp.fits(LayoutUnit::from_px(800)));
    }

    #[test]
    fn test_single_float_left_and_clearance() {
        let mut space = ExclusionSpace::new();
        let float_rect = LogicalRect::new(
            LayoutUnit::ZERO,
            LayoutUnit::from_px(0),
            LayoutUnit::from_px(200),
            LayoutUnit::from_px(100),
        );
        space.add_exclusion(Exclusion::new(float_rect, FloatType::Left, NodeId::new()));

        assert_eq!(space.clear_offset(ClearType::Left), LayoutUnit::from_px(100));
        assert_eq!(space.clear_offset(ClearType::Right), LayoutUnit::ZERO);
        assert_eq!(space.clear_offset(ClearType::Both), LayoutUnit::from_px(100));

        // Busca na fatia [0, 50] com largura 800px
        let opp = space.find_layout_opportunity(
            LayoutUnit::ZERO,
            LayoutUnit::from_px(800),
            LayoutUnit::from_px(100),
            LayoutUnit::from_px(50),
        );
        assert_eq!(opp.block_start, LayoutUnit::ZERO);
        assert_eq!(opp.inline_start, LayoutUnit::from_px(200));
        assert_eq!(opp.inline_size, LayoutUnit::from_px(600));

        // Busca na fatia [100, 150] abaixo do float
        let opp_below = space.find_layout_opportunity(
            LayoutUnit::from_px(100),
            LayoutUnit::from_px(800),
            LayoutUnit::from_px(100),
            LayoutUnit::from_px(50),
        );
        assert_eq!(opp_below.block_start, LayoutUnit::from_px(100));
        assert_eq!(opp_below.inline_start, LayoutUnit::ZERO);
        assert_eq!(opp_below.inline_size, LayoutUnit::from_px(800));
    }

    #[test]
    fn test_opposing_floats_narrow_corridor() {
        let mut space = ExclusionSpace::new();
        // Float esquerdo de 300px
        space.add_exclusion(Exclusion::new(
            LogicalRect::new(
                LayoutUnit::ZERO,
                LayoutUnit::ZERO,
                LayoutUnit::from_px(300),
                LayoutUnit::from_px(100),
            ),
            FloatType::Left,
            NodeId::new(),
        ));
        // Float direito de 400px em container de 800px (espaço entre eles = 100px)
        space.add_exclusion(Exclusion::new(
            LogicalRect::new(
                LayoutUnit::from_px(400),
                LayoutUnit::ZERO,
                LayoutUnit::from_px(400),
                LayoutUnit::from_px(50),
            ),
            FloatType::Right,
            NodeId::new(),
        ));

        // Requisição de 80px cabe no corredor de 100px
        let opp = space.find_layout_opportunity(
            LayoutUnit::ZERO,
            LayoutUnit::from_px(800),
            LayoutUnit::from_px(80),
            LayoutUnit::from_px(30),
        );
        assert_eq!(opp.block_start, LayoutUnit::ZERO);
        assert_eq!(opp.inline_start, LayoutUnit::from_px(300));
        assert_eq!(opp.inline_size, LayoutUnit::from_px(100));

        // Requisição de 200px não cabe no corredor; deve descer para a prateleira do float direito (y = 50px)
        let opp2 = space.find_layout_opportunity(
            LayoutUnit::ZERO,
            LayoutUnit::from_px(800),
            LayoutUnit::from_px(200),
            LayoutUnit::from_px(30),
        );
        assert_eq!(opp2.block_start, LayoutUnit::from_px(50));
        assert_eq!(opp2.inline_start, LayoutUnit::from_px(300));
        // Em y = 50, o float direito acabou, então resta 800 - 300 = 500px
        assert_eq!(opp2.inline_size, LayoutUnit::from_px(500));
    }
}
