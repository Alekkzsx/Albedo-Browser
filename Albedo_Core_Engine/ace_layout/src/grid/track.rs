//! # Dimensionamento de Trilhas Grid CSS L1 (`track.rs`)
//!
//! Implementa o dimensionamento de trilhas em grade conforme W3C CSS Grid Layout L1 §7 e §11:
//! - Unidades absolutas `px`, relativas `%`, flexíveis `fr`, automáticas `auto` e `minmax(min, max)`
//! - Cálculo de tamanho base (*base size*) e limite de crescimento (*growth limit*)
//! - Alocação proporcional de espaço para unidades `fr` do espaço disponível remanescente

use ace_core::math::layout_unit::LayoutUnit;

/// Unidade de largura de uma trilha de grade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridTrackBreadth {
    /// Comprimento absoluto em pixels / LayoutUnit.
    Px(LayoutUnit),
    /// Percentual em relação ao contêiner de grade.
    Percent(f32),
    /// Fração do espaço livre remanescente (`fr`).
    Fr(f32),
    /// Tamanho automático baseado no conteúdo.
    Auto,
    /// Tamanho mínimo do conteúdo (min-content).
    MinContent,
    /// Tamanho máximo do conteúdo (max-content).
    MaxContent,
}

impl GridTrackBreadth {
    #[inline]
    pub fn is_flexible(&self) -> bool {
        matches!(self, Self::Fr(_))
    }

    #[inline]
    pub fn fr_value(&self) -> f32 {
        match self {
            Self::Fr(f) => *f,
            _ => 0.0,
        }
    }
}

/// Especificação dimensional de uma trilha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridTrackSize {
    /// Tamanho único fixo ou flexível.
    Length(GridTrackBreadth),
    /// Faixa funcional delimitada por mínimo e máximo: `minmax(min, max)`.
    MinMax(GridTrackBreadth, GridTrackBreadth),
    /// Tamanho limitado por `fit-content(limit)`.
    FitContent(LayoutUnit),
}

impl GridTrackSize {
    /// Cria uma trilha com tamanho fixo em pixels.
    pub fn px(px: i32) -> Self {
        Self::Length(GridTrackBreadth::Px(LayoutUnit::from_px(px)))
    }

    /// Cria uma trilha flexível em unidades `fr`.
    pub fn fr(fr: f32) -> Self {
        Self::Length(GridTrackBreadth::Fr(fr))
    }

    /// Cria uma trilha com função `minmax`.
    pub fn minmax(min: GridTrackBreadth, max: GridTrackBreadth) -> Self {
        Self::MinMax(min, max)
    }

    /// Cria uma trilha automática `auto`.
    pub fn auto() -> Self {
        Self::Length(GridTrackBreadth::Auto)
    }
}

/// Estado de uma trilha durante a resolução do algoritmo de layout.
#[derive(Debug, Clone, PartialEq)]
pub struct GridTrack {
    /// Tamanho base computado da trilha.
    pub base_size: LayoutUnit,
    /// Limite de crescimento superior da trilha (`None` indica infinito).
    pub growth_limit: Option<LayoutUnit>,
    /// Indica se a trilha possui fração flexível `fr`.
    pub is_flexible: bool,
    /// Fator `fr` associado se flexível.
    pub fr_factor: f32,
}

impl GridTrack {
    pub fn new() -> Self {
        Self {
            base_size: LayoutUnit::ZERO,
            growth_limit: None,
            is_flexible: false,
            fr_factor: 0.0,
        }
    }
}

impl Default for GridTrack {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve os tamanhos definitivos de um conjunto de trilhas em uma dimensão.
///
/// Implementa a resolução de trilhas com:
/// 1. Inicialização de base sizes e growth limits
/// 2. Resolução de frações flexíveis `fr` sobre o espaço restante
/// 3. Respeito irrestrito a restrições `minmax`
pub fn resolve_track_sizes(
    track_definitions: &[GridTrackSize],
    available_space: Option<LayoutUnit>,
    gap: LayoutUnit,
    content_min_sizes: &[LayoutUnit],
    content_max_sizes: &[LayoutUnit],
) -> Vec<LayoutUnit> {
    let count = track_definitions.len();
    if count == 0 {
        return Vec::new();
    }

    let total_gaps = if count > 1 {
        gap.saturating_mul((count - 1) as i32)
    } else {
        LayoutUnit::ZERO
    };

    let mut tracks = Vec::with_capacity(count);

    // Passo 1: Inicializa base size e growth limit para cada trilha
    for (i, def) in track_definitions.iter().enumerate() {
        let mut track = GridTrack::new();
        let content_min = content_min_sizes
            .get(i)
            .copied()
            .unwrap_or(LayoutUnit::ZERO);
        let content_max = content_max_sizes
            .get(i)
            .copied()
            .unwrap_or(LayoutUnit::ZERO);

        match def {
            GridTrackSize::Length(breadth) => {
                apply_breadth(
                    &mut track,
                    *breadth,
                    available_space,
                    content_min,
                    content_max,
                );
            }
            GridTrackSize::MinMax(min_breadth, max_breadth) => {
                apply_breadth(
                    &mut track,
                    *min_breadth,
                    available_space,
                    content_min,
                    content_max,
                );
                // O limite superior ajusta o growth_limit
                match max_breadth {
                    GridTrackBreadth::Px(px) => track.growth_limit = Some(*px),
                    GridTrackBreadth::Percent(p) => {
                        if let Some(avail) = available_space {
                            let raw = (avail.raw() as f64 * (*p as f64 / 100.0)).round() as i32;
                            track.growth_limit = Some(LayoutUnit::from_raw(raw));
                        }
                    }
                    GridTrackBreadth::Fr(fr) => {
                        track.is_flexible = true;
                        track.fr_factor = *fr;
                        track.growth_limit = None;
                    }
                    GridTrackBreadth::Auto | GridTrackBreadth::MaxContent => {
                        track.growth_limit = Some(content_max);
                    }
                    GridTrackBreadth::MinContent => {
                        track.growth_limit = Some(content_min);
                    }
                }
            }
            GridTrackSize::FitContent(limit) => {
                track.base_size = content_min.min(*limit);
                track.growth_limit = Some(content_max.min(*limit));
            }
        }

        // Invariante: base_size não pode exceder growth_limit quando este é definido
        if let Some(limit) = track.growth_limit {
            if track.base_size > limit {
                track.growth_limit = Some(track.base_size);
            }
        }

        tracks.push(track);
    }

    // Passo 2: Resolução de trilhas flexíveis `fr`
    let sum_non_flex_base: LayoutUnit = tracks
        .iter()
        .filter(|t| !t.is_flexible)
        .fold(LayoutUnit::ZERO, |acc, t| acc.saturating_add(t.base_size));

    let sum_fr: f32 = tracks.iter().map(|t| t.fr_factor).sum();

    if let Some(avail) = available_space {
        let used_so_far = sum_non_flex_base.saturating_add(total_gaps);
        let remaining = avail.saturating_sub(used_so_far);

        if remaining.raw() > 0 && sum_fr > 0.0 {
            let flex_space_raw = if sum_fr < 1.0 {
                (remaining.raw() as f32 * sum_fr) as i32
            } else {
                remaining.raw()
            };

            let divisor = sum_fr.max(1.0) as f64;
            let mut allocated_sum = 0i32;

            for track in tracks.iter_mut().filter(|t| t.is_flexible) {
                let share_raw =
                    (flex_space_raw as f64 * (track.fr_factor as f64 / divisor)).round() as i32;
                let track_size = LayoutUnit::from_raw(share_raw);

                track.base_size = track.base_size.max(track_size);
                if let Some(limit) = track.growth_limit {
                    track.base_size = track.base_size.min(limit);
                }
                allocated_sum += track.base_size.raw();
            }

            // Conservação exata: se sobrou resíduo pequeno (<= 5 au) e sum_fr >= 1.0, ajusta a maior trilha
            let leftover = flex_space_raw - allocated_sum;
            if leftover != 0 && leftover.abs() <= 5 && sum_fr >= 1.0 {
                if let Some(largest_track) =
                    tracks.iter_mut().filter(|t| t.is_flexible).max_by(|a, b| {
                        a.fr_factor
                            .partial_cmp(&b.fr_factor)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                {
                    largest_track.base_size =
                        LayoutUnit::from_raw(largest_track.base_size.raw() + leftover);
                }
            }
        }
    }

    tracks.into_iter().map(|t| t.base_size).collect()
}

fn apply_breadth(
    track: &mut GridTrack,
    breadth: GridTrackBreadth,
    available_space: Option<LayoutUnit>,
    content_min: LayoutUnit,
    content_max: LayoutUnit,
) {
    match breadth {
        GridTrackBreadth::Px(px) => {
            track.base_size = px;
            track.growth_limit = Some(px);
        }
        GridTrackBreadth::Percent(p) => {
            if let Some(avail) = available_space {
                let raw = (avail.raw() as f64 * (p as f64 / 100.0)).round() as i32;
                let size = LayoutUnit::from_raw(raw);
                track.base_size = size;
                track.growth_limit = Some(size);
            } else {
                track.base_size = LayoutUnit::ZERO;
                track.growth_limit = None;
            }
        }
        GridTrackBreadth::Fr(fr) => {
            track.base_size = LayoutUnit::ZERO;
            track.growth_limit = None;
            track.is_flexible = true;
            track.fr_factor = fr;
        }
        GridTrackBreadth::Auto => {
            track.base_size = content_min;
            track.growth_limit = Some(content_max);
        }
        GridTrackBreadth::MinContent => {
            track.base_size = content_min;
            track.growth_limit = Some(content_min);
        }
        GridTrackBreadth::MaxContent => {
            track.base_size = content_max;
            track.growth_limit = Some(content_max);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_track_sizing_fr_equal() {
        let defs = vec![GridTrackSize::fr(1.0), GridTrackSize::fr(1.0)];
        let avail = Some(LayoutUnit::from_px(400));
        let gap = LayoutUnit::ZERO;

        let sizes = resolve_track_sizes(&defs, avail, gap, &[], &[]);
        assert_eq!(sizes.len(), 2);
        assert_eq!(sizes[0], LayoutUnit::from_px(200));
        assert_eq!(sizes[1], LayoutUnit::from_px(200));
    }

    #[test]
    fn test_grid_track_sizing_mixed_px_and_fr() {
        let defs = vec![
            GridTrackSize::px(100),
            GridTrackSize::fr(1.0),
            GridTrackSize::fr(2.0),
        ];
        let avail = Some(LayoutUnit::from_px(400));
        let gap = LayoutUnit::from_px(10); // 2 gaps of 10px = 20px

        // Available for tracks: 400 - 20 (gaps) = 380px.
        // Track 0 takes 100px. Remaining = 280px.
        // Total fr = 3.
        // Track 1: 280 * 1/3 = 93.33px (5600 au).
        // Track 2: 280 * 2/3 = 186.67px (11200 au).
        let sizes = resolve_track_sizes(&defs, avail, gap, &[], &[]);
        assert_eq!(sizes.len(), 3);
        assert_eq!(sizes[0], LayoutUnit::from_px(100));
        let sum_flex = sizes[1] + sizes[2];
        assert_eq!(sum_flex, LayoutUnit::from_px(280));
    }

    #[test]
    fn test_grid_track_sizing_minmax_clamping() {
        let defs = vec![
            GridTrackSize::minmax(
                GridTrackBreadth::Px(LayoutUnit::from_px(80)),
                GridTrackBreadth::Px(LayoutUnit::from_px(150)),
            ),
            GridTrackSize::fr(1.0),
        ];
        let avail = Some(LayoutUnit::from_px(300));
        let gap = LayoutUnit::ZERO;

        let sizes = resolve_track_sizes(&defs, avail, gap, &[], &[]);
        assert_eq!(sizes.len(), 2);
        assert_eq!(sizes[0], LayoutUnit::from_px(80));
        assert_eq!(sizes[1], LayoutUnit::from_px(220));
    }
}
