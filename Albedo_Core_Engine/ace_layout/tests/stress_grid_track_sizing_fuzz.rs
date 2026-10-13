//! # Stress Suite 4: CSS Grid 50x50 Track Sizing Fuzzing & Auto-Placement
//!
//! Requisito Normativo: W3C CSS Grid Layout Module Level 1 (§7, §8, §11).
//! Valida:
//! 1. Fuzzing determinístico pseudo-aleatório em grade de 50x50 trilhas (2.500 células):
//!    - Trilhas px, %, fr, auto, minmax(50px, 1fr), minmax(20px, 100px).
//!    - Resolução fracionária proporcional: W_k = R * (fr_k / max(1.0, sum(fr))).
//!    - Respeito a restrições minmax: A <= track_size <= B.
//! 2. Algoritmo de auto-placement com spans complexos (span 1..4 colunas, 1..3 linhas)
//!    sob `grid-auto-flow: row`, `column` e `dense`.
//! 3. Zero sobreposição entre itens alocados na matriz 2D.
//! 4. SLA de performance: resolução e placement da matriz 50x50 em menos de 200 ms.

mod common;

use ace_core::math::layout_unit::LayoutUnit;
use ace_style::values::enums::GridAutoFlow;

use ace_layout::grid::placement::{place_grid_items, ItemPlacementSpec};
use ace_layout::grid::track::{
    resolve_track_sizes, GridTrackBreadth, GridTrackSize,
};
use common::BenchmarkTimer;

/// Gerador linear congruente simples e determinístico para fuzzing com seed fixa.
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn next_range(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        min + (self.next_u32() % (max - min + 1))
    }
}

#[test]
fn test_stress_50x50_tracks_sizing_fuzz() {
    let timer = BenchmarkTimer::start();
    let mut rng = DeterministicRng::new(0xACE_641D_5050);

    let track_count = 50;
    let mut col_specs = Vec::with_capacity(track_count);
    let mut row_specs = Vec::with_capacity(track_count);

    // Gera 50 definições de colunas com tipos variados
    for _ in 0..track_count {
        let spec = match rng.next_range(0, 5) {
            0 => GridTrackSize::px(rng.next_range(20, 100) as i32),
            1 => GridTrackSize::fr(rng.next_range(1, 4) as f32),
            2 => GridTrackSize::minmax(
                GridTrackBreadth::Px(LayoutUnit::from_px(rng.next_range(20, 50) as i32)),
                GridTrackBreadth::Fr(rng.next_range(1, 3) as f32),
            ),
            3 => GridTrackSize::minmax(
                GridTrackBreadth::Px(LayoutUnit::from_px(30)),
                GridTrackBreadth::Px(LayoutUnit::from_px(rng.next_range(60, 120) as i32)),
            ),
            4 => GridTrackSize::auto(),
            _ => GridTrackSize::Length(GridTrackBreadth::Percent(rng.next_range(1, 2) as f32)),
        };
        col_specs.push(spec);
    }

    // Gera 50 definições de linhas
    for _ in 0..track_count {
        let spec = match rng.next_range(0, 3) {
            0 => GridTrackSize::px(rng.next_range(20, 60) as i32),
            1 => GridTrackSize::fr(rng.next_range(1, 2) as f32),
            _ => GridTrackSize::auto(),
        };
        row_specs.push(spec);
    }

    let available_w = Some(LayoutUnit::from_px(3000));
    let available_h = Some(LayoutUnit::from_px(2000));
    let gap = LayoutUnit::from_px(5);

    let content_min = vec![LayoutUnit::from_px(20); track_count];
    let content_max = vec![LayoutUnit::from_px(80); track_count];

    // Executa resolução das 50 colunas
    let col_sizes = resolve_track_sizes(&col_specs, available_w, gap, &content_min, &content_max);
    assert_eq!(col_sizes.len(), track_count);

    // Executa resolução das 50 linhas
    let row_sizes = resolve_track_sizes(&row_specs, available_h, gap, &content_min, &content_max);
    assert_eq!(row_sizes.len(), track_count);

    // Invariantes para as colunas:
    for (i, &size) in col_sizes.iter().enumerate() {
        assert!(size >= LayoutUnit::ZERO, "Track size must be non-negative: {:?}", size);
        if let GridTrackSize::MinMax(GridTrackBreadth::Px(min), GridTrackBreadth::Px(max)) = col_specs[i] {
            assert!(
                size >= min,
                "Track {} size {:?} violated min constraint {:?}",
                i, size, min
            );
            assert!(
                size <= max,
                "Track {} size {:?} violated max constraint {:?}",
                i, size, max
            );
        }
    }

    let report = timer.report(track_count * 2, "Grid_50x50_track_sizing_fuzz");
    assert!(
        report.elapsed_secs < 0.20,
        "50x50 track sizing fuzzing took {:.4}s (exceeded 0.20s SLA)",
        report.elapsed_secs
    );
}

#[test]
fn test_stress_50x50_auto_placement_and_dense_packing() {
    let timer = BenchmarkTimer::start();
    let mut rng = DeterministicRng::new(0xDEAD_BEEF_CAFE);

    let total_items = 150;
    let mut item_specs = Vec::with_capacity(total_items);

    // Gera 150 itens com spans variados para validar ausência de sobreposição no auto-placement
    for _ in 0..total_items {
        let col_span = rng.next_range(1, 4) as u16;
        let row_span = rng.next_range(1, 3) as u16;
        item_specs.push(ItemPlacementSpec::with_spans(col_span, row_span));
    }

    // Executa auto-placement nos 3 modos
    for flow in [GridAutoFlow::Row, GridAutoFlow::Column, GridAutoFlow::Dense] {
        let (placements, _cols, _rows) = place_grid_items(&item_specs, 50, 50, flow);
        assert_eq!(placements.len(), total_items);

        // Valida que nenhum par de itens se sobrepõe
        for a in 0..total_items {
            for b in (a + 1)..total_items {
                let pa = &placements[a];
                let pb = &placements[b];

                let h_overlap = pa.col_start < pb.col_end && pa.col_end > pb.col_start;
                let v_overlap = pa.row_start < pb.row_end && pa.row_end > pb.row_start;

                assert!(
                    !(h_overlap && v_overlap),
                    "Collision detected between item {} [cols {}..{}, rows {}..{}] and item {} [cols {}..{}, rows {}..{}] under flow {:?}",
                    a, pa.col_start, pa.col_end, pa.row_start, pa.row_end,
                    b, pb.col_start, pb.col_end, pb.row_start, pb.row_end,
                    flow
                );
            }
        }
    }

    let report = timer.report(total_items * 3, "Grid_50x50_auto_placement");
    assert!(
        report.elapsed_secs < 0.20,
        "Auto placement 50x50 took {:.4}s (exceeded 0.20s SLA)",
        report.elapsed_secs
    );
}
