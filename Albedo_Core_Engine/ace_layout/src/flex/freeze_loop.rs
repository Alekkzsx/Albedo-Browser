//! # Algoritmo de Freeze Loop do Flexbox (§9.7)
//!
//! Implementa a resolução de comprimentos flexíveis em tempo $O(N)$ conforme
//! W3C CSS Flexible Box Layout Module Level 1 §9.7.
//!
//! O loop distribui espaço livre positivo (`flex-grow`) ou negativo (`flex-shrink`),
//! detecta violações de limites mínimos (`min-width/height`) e máximos (`max-width/height`),
//! congela os itens violadores em seus limites e itera até estabilizar todos os itens.

use ace_core::math::layout_unit::LayoutUnit;

/// Descritor de um item flexível para o loop de congelamento.
#[derive(Debug, Clone, PartialEq)]
pub struct FlexFreezeItem {
    /// Fator de crescimento flexível (`flex-grow`).
    pub flex_grow: f32,
    /// Fator de encolhimento flexível (`flex-shrink`).
    pub flex_shrink: f32,
    /// Tamanho base flexível (`flex-basis`).
    pub flex_basis: LayoutUnit,
    /// Tamanho mínimo principal permitido (`min-width` ou `min-height`).
    pub min_size: LayoutUnit,
    /// Tamanho máximo principal permitido (`max-width` ou `max-height`), se houver.
    pub max_size: Option<LayoutUnit>,
    /// Tamanho principal alvo computado pelo algoritmo.
    pub target_size: LayoutUnit,
    /// Indica se o item já foi congelado no loop.
    pub frozen: bool,
}

impl FlexFreezeItem {
    /// Cria um novo item flexível para resolução de comprimento.
    pub fn new(
        flex_grow: f32,
        flex_shrink: f32,
        flex_basis: LayoutUnit,
        min_size: LayoutUnit,
        max_size: Option<LayoutUnit>,
    ) -> Self {
        // Clampa a base inicial contra min e max se aplicável
        let clamped_basis = match max_size {
            Some(max) => flex_basis.max(min_size).min(max),
            None => flex_basis.max(min_size),
        };

        Self {
            flex_grow: flex_grow.max(0.0),
            flex_shrink: flex_shrink.max(0.0),
            flex_basis,
            min_size,
            max_size,
            target_size: clamped_basis,
            frozen: false,
        }
    }
}

/// Tipo de violação de restrição em um passo do freeze loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Violation {
    None,
    Min(LayoutUnit),
    Max(LayoutUnit),
}

/// Executa o algoritmo de Freeze Loop (§9.7) em uma linha flexível.
///
/// Retorna o espaço livre restante após a distribuição de todos os comprimentos flexíveis.
pub fn resolve_flexible_lengths(
    items: &mut [FlexFreezeItem],
    line_main_size: LayoutUnit,
    total_gap: LayoutUnit,
) -> LayoutUnit {
    if items.is_empty() {
        return line_main_size.saturating_sub(total_gap).max(LayoutUnit::ZERO);
    }

    // 1. Determina a soma das bases flexíveis e o espaço livre inicial
    let sum_flex_basis: LayoutUnit = items
        .iter()
        .fold(LayoutUnit::ZERO, |acc, item| acc.saturating_add(item.flex_basis));

    let available_for_items = line_main_size.saturating_sub(total_gap);
    let initial_free_space = available_for_items.saturating_sub(sum_flex_basis);

    let is_growing = initial_free_space.raw() > 0;
    let is_shrinking = initial_free_space.raw() < 0;

    // Inicializa todos os itens como não congelados e define o target inicial como flex_basis
    for item in items.iter_mut() {
        item.target_size = item.flex_basis;
        item.frozen = false;

        // Congela imediatamente itens inflexíveis conforme §9.7
        if (is_growing && item.flex_grow <= 0.0)
            || (is_shrinking && item.flex_shrink <= 0.0)
            || (!is_growing && !is_shrinking)
        {
            item.target_size = item.flex_basis.max(item.min_size);
            if let Some(max) = item.max_size {
                item.target_size = item.target_size.min(max);
            }
            item.frozen = true;
        }
    }

    // Se todos foram congelados inicialmente, finaliza
    if items.iter().all(|it| it.frozen) {
        return compute_remaining_space(items, line_main_size, total_gap);
    }

    // 2. Loop de Congelamento (executa no máximo N iterações)
    let n = items.len();
    for _ in 0..n {
        let all_frozen = items.iter().all(|it| it.frozen);
        if all_frozen {
            break;
        }

        // Calcula o espaço livre restante
        let mut sum_frozen = LayoutUnit::ZERO;
        let mut sum_unfrozen_basis = LayoutUnit::ZERO;

        for item in items.iter() {
            if item.frozen {
                sum_frozen = sum_frozen.saturating_add(item.target_size);
            } else {
                sum_unfrozen_basis = sum_unfrozen_basis.saturating_add(item.flex_basis);
            }
        }

        let remaining_free_space = available_for_items
            .saturating_sub(sum_frozen)
            .saturating_sub(sum_unfrozen_basis);

        let mut candidate_sizes = vec![LayoutUnit::ZERO; items.len()];
        let mut violations = vec![Violation::None; items.len()];
        let mut has_violation = false;

        if is_growing {
            let sum_grow: f32 = items
                .iter()
                .filter(|it| !it.frozen)
                .map(|it| it.flex_grow)
                .sum();

            if sum_grow <= 0.0 {
                // Nenhum item restante pode crescer
                for item in items.iter_mut().filter(|it| !it.frozen) {
                    item.frozen = true;
                }
                break;
            }

            let factor_scale = if sum_grow < 1.0 { 1.0 } else { sum_grow };

            for (i, item) in items.iter().enumerate() {
                if item.frozen {
                    candidate_sizes[i] = item.target_size;
                    continue;
                }

                let ratio = (item.flex_grow / factor_scale) as f64;
                let delta_raw = (remaining_free_space.raw() as f64 * ratio).round() as i32;
                let candidate = item.flex_basis.saturating_add(LayoutUnit::from_raw(delta_raw));

                let (viol, clamped) = check_violation(candidate, item.min_size, item.max_size);
                candidate_sizes[i] = clamped;
                violations[i] = viol;
                if viol != Violation::None {
                    has_violation = true;
                }
            }
        } else if is_shrinking {
            let sum_scaled_shrink: f64 = items
                .iter()
                .filter(|it| !it.frozen)
                .map(|it| it.flex_shrink as f64 * it.flex_basis.raw() as f64)
                .sum();

            let sum_shrink: f32 = items
                .iter()
                .filter(|it| !it.frozen)
                .map(|it| it.flex_shrink)
                .sum();

            if sum_scaled_shrink <= 0.0 || sum_shrink <= 0.0 {
                for item in items.iter_mut().filter(|it| !it.frozen) {
                    item.frozen = true;
                }
                break;
            }

            for (i, item) in items.iter().enumerate() {
                if item.frozen {
                    candidate_sizes[i] = item.target_size;
                    continue;
                }

                let scaled_shrink = item.flex_shrink as f64 * item.flex_basis.raw() as f64;
                let ratio = if sum_shrink < 1.0 {
                    (item.flex_shrink as f64) * (item.flex_basis.raw() as f64)
                        / (items.len() as f64 * 1.0)
                } else {
                    scaled_shrink / sum_scaled_shrink
                };

                let delta_raw = (remaining_free_space.raw() as f64 * ratio).round() as i32;
                let candidate = item.flex_basis.saturating_add(LayoutUnit::from_raw(delta_raw));

                let (viol, clamped) = check_violation(candidate, item.min_size, item.max_size);
                candidate_sizes[i] = clamped;
                violations[i] = viol;
                if viol != Violation::None {
                    has_violation = true;
                }
            }
        } else {
            // Nem cresce nem encolhe
            for item in items.iter_mut() {
                item.frozen = true;
            }
            break;
        }

        if has_violation {
            // Congela todos os itens com violações
            for (i, item) in items.iter_mut().enumerate() {
                if !item.frozen && violations[i] != Violation::None {
                    item.target_size = candidate_sizes[i];
                    item.frozen = true;
                }
            }
        } else {
            // Sem violações: atribui tamanhos candidatos e congela todos
            for (i, item) in items.iter_mut().enumerate() {
                if !item.frozen {
                    item.target_size = candidate_sizes[i];
                    item.frozen = true;
                }
            }
            break;
        }
    }

    // Conservação exata de ponto fixo: se houver sobras de arredondamento e itens elásticos,
    // ajusta o menor delta possível para garantir invariante exata se possível
    enforce_exact_conservation(items, line_main_size, total_gap);

    compute_remaining_space(items, line_main_size, total_gap)
}

/// Verifica se um tamanho viola as restrições de min e max.
#[inline]
fn check_violation(
    candidate: LayoutUnit,
    min_size: LayoutUnit,
    max_size: Option<LayoutUnit>,
) -> (Violation, LayoutUnit) {
    if candidate < min_size {
        (Violation::Min(min_size), min_size)
    } else if let Some(max) = max_size {
        if candidate > max {
            (Violation::Max(max), max)
        } else {
            (Violation::None, candidate)
        }
    } else {
        (Violation::None, candidate)
    }
}

/// Ajusta discrepâncias residuais de arredondamento para conservação perfeita.
fn enforce_exact_conservation(
    items: &mut [FlexFreezeItem],
    line_main_size: LayoutUnit,
    total_gap: LayoutUnit,
) {
    let available = line_main_size.saturating_sub(total_gap);
    let current_sum: LayoutUnit = items
        .iter()
        .fold(LayoutUnit::ZERO, |acc, it| acc.saturating_add(it.target_size));

    let diff = available.raw().saturating_sub(current_sum.raw());
    // Se a diferença for minúscula (resíduo de arredondamento de alguns subpixels, |diff| <= 5 au)
    if diff != 0 && diff.abs() <= 5 {
        // Encontra um item com flex_grow > 0 ou flex_shrink > 0 que não esteja preso rigidamente
        if let Some(item) = items
            .iter_mut()
            .find(|it| it.flex_grow > 0.0 || it.flex_shrink > 0.0)
        {
            let adjusted = LayoutUnit::from_raw(item.target_size.raw() + diff);
            let clamped = match item.max_size {
                Some(max) => adjusted.max(item.min_size).min(max),
                None => adjusted.max(item.min_size),
            };
            item.target_size = clamped;
        }
    }
}

/// Computa o espaço livre restante na linha flexível.
#[inline]
fn compute_remaining_space(
    items: &[FlexFreezeItem],
    line_main_size: LayoutUnit,
    total_gap: LayoutUnit,
) -> LayoutUnit {
    let sum_items: LayoutUnit = items
        .iter()
        .fold(LayoutUnit::ZERO, |acc, it| acc.saturating_add(it.target_size));

    let used = sum_items.saturating_add(total_gap);
    line_main_size.saturating_sub(used)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freeze_loop_pure_grow_equal() {
        let mut items = vec![
            FlexFreezeItem::new(1.0, 1.0, LayoutUnit::from_px(100), LayoutUnit::ZERO, None),
            FlexFreezeItem::new(1.0, 1.0, LayoutUnit::from_px(100), LayoutUnit::ZERO, None),
        ];

        let line_size = LayoutUnit::from_px(400);
        let gap = LayoutUnit::ZERO;

        let free = resolve_flexible_lengths(&mut items, line_size, gap);
        assert_eq!(items[0].target_size, LayoutUnit::from_px(200));
        assert_eq!(items[1].target_size, LayoutUnit::from_px(200));
        assert_eq!(free, LayoutUnit::ZERO);
    }

    #[test]
    fn test_freeze_loop_pure_grow_weighted() {
        let mut items = vec![
            FlexFreezeItem::new(1.0, 1.0, LayoutUnit::from_px(100), LayoutUnit::ZERO, None),
            FlexFreezeItem::new(2.0, 1.0, LayoutUnit::from_px(100), LayoutUnit::ZERO, None),
        ];

        let line_size = LayoutUnit::from_px(500);
        let gap = LayoutUnit::ZERO;

        let free = resolve_flexible_lengths(&mut items, line_size, gap);
        // Free space = 300px. Grow sum = 3. Item 1 receives 100px -> 200px. Item 2 receives 200px -> 300px.
        assert_eq!(items[0].target_size, LayoutUnit::from_px(200));
        assert_eq!(items[1].target_size, LayoutUnit::from_px(300));
        assert_eq!(free, LayoutUnit::ZERO);
    }

    #[test]
    fn test_freeze_loop_grow_with_max_clamping() {
        let mut items = vec![
            FlexFreezeItem::new(
                1.0,
                1.0,
                LayoutUnit::from_px(100),
                LayoutUnit::ZERO,
                Some(LayoutUnit::from_px(150)), // Max-width 150px
            ),
            FlexFreezeItem::new(1.0, 1.0, LayoutUnit::from_px(100), LayoutUnit::ZERO, None),
        ];

        let line_size = LayoutUnit::from_px(400);
        let gap = LayoutUnit::ZERO;

        // Total free space = 200px. Item 1 would like 200px, but clamps to 150px.
        // Item 1 freezes at 150px (+50px).
        // Remaining free space = 150px goes to Item 2 -> 100 + 150 = 250px.
        let free = resolve_flexible_lengths(&mut items, line_size, gap);
        assert_eq!(items[0].target_size, LayoutUnit::from_px(150));
        assert_eq!(items[1].target_size, LayoutUnit::from_px(250));
        assert_eq!(free, LayoutUnit::ZERO);
    }

    #[test]
    fn test_freeze_loop_pure_shrink_weighted() {
        let mut items = vec![
            // basis 200px, shrink 1 -> scaled shrink = 200
            FlexFreezeItem::new(0.0, 1.0, LayoutUnit::from_px(200), LayoutUnit::ZERO, None),
            // basis 400px, shrink 1 -> scaled shrink = 400
            FlexFreezeItem::new(0.0, 1.0, LayoutUnit::from_px(400), LayoutUnit::ZERO, None),
        ];

        let line_size = LayoutUnit::from_px(300);
        let gap = LayoutUnit::ZERO;

        // Total basis = 600px. Space deficit = -300px.
        // Scaled sum = 200 + 400 = 600.
        // Item 0 delta = -300 * (200/600) = -100px -> 100px.
        // Item 1 delta = -300 * (400/600) = -200px -> 200px.
        let free = resolve_flexible_lengths(&mut items, line_size, gap);
        assert_eq!(items[0].target_size, LayoutUnit::from_px(100));
        assert_eq!(items[1].target_size, LayoutUnit::from_px(200));
        assert_eq!(free, LayoutUnit::ZERO);
    }

    #[test]
    fn test_freeze_loop_shrink_with_min_clamping() {
        let mut items = vec![
            FlexFreezeItem::new(
                0.0,
                1.0,
                LayoutUnit::from_px(200),
                LayoutUnit::from_px(150), // Min-width 150px
                None,
            ),
            FlexFreezeItem::new(0.0, 1.0, LayoutUnit::from_px(200), LayoutUnit::ZERO, None),
        ];

        let line_size = LayoutUnit::from_px(250);
        let gap = LayoutUnit::ZERO;

        // Total basis = 400px. Deficit = -150px.
        // Equal bases and shrinks -> each would get -75px -> 125px.
        // But Item 0 has min 150px! Clamps to 150px (-50px) and freezes.
        // Remaining deficit = -100px goes to Item 1 -> 200 - 100 = 100px.
        let free = resolve_flexible_lengths(&mut items, line_size, gap);
        assert_eq!(items[0].target_size, LayoutUnit::from_px(150));
        assert_eq!(items[1].target_size, LayoutUnit::from_px(100));
        assert_eq!(free, LayoutUnit::ZERO);
    }

    #[test]
    fn test_freeze_loop_with_gap() {
        let mut items = vec![
            FlexFreezeItem::new(1.0, 1.0, LayoutUnit::from_px(50), LayoutUnit::ZERO, None),
            FlexFreezeItem::new(1.0, 1.0, LayoutUnit::from_px(50), LayoutUnit::ZERO, None),
        ];

        let line_size = LayoutUnit::from_px(200);
        let gap = LayoutUnit::from_px(20);

        // Available for items = 200 - 20 = 180px.
        // Bases sum = 100px. Free space = 80px.
        // Each grows by 40px -> 90px each.
        // Sum items (180) + gap (20) = 200px.
        let free = resolve_flexible_lengths(&mut items, line_size, gap);
        assert_eq!(items[0].target_size, LayoutUnit::from_px(90));
        assert_eq!(items[1].target_size, LayoutUnit::from_px(90));
        assert_eq!(free, LayoutUnit::ZERO);
    }
}
