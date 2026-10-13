//! # Stress Suite 7: Subpixel Snapping Accuracy (10,000 Elements & Multi-DPI)
//!
//! Requisito Normativo: Aritmética de Snapping Subpixel sem Fendas (Zero Pixel Cracking).
//! Valida:
//! 1. 10.000 elementos fracionários adjacentes subdividindo contêineres sob frações periódicas:
//!    - Terços (33.333...px = 2.000 au)
//!    - Sétimos (14.2857...px = 857.14 au)
//!    - Onze avos (9.0909...px = 545.45 au)
//!    - Treze avos, dezessete avos, etc.
//! 2. Teorema de Adjacência: snapped_end(B_i) == snapped_origin(B_{i+1}) para todo par contíguo.
//! 3. Conservação exata de extensão: sum(snapped_sizes) == snapped_total_size.
//! 4. Contagem de lacunas de 1px: estritamente 0.
//! 5. Contagem de sobreposições de 1px: estritamente 0.
//! 6. Múltiplas escalas de densidade de tela (1.0x, 1.25x, 1.5x, 2.0x, 2.5x, 3.0x).
//! 7. SLA de performance: 10.000 elementos verificados em menos de 50 ms.

mod common;

use ace_core::math::layout_unit::{snap_box, LayoutUnit};
use ace_core::math::logical::LogicalRect;
use ace_layout::fragment::snap_logical_rect;
use common::{assert_zero_gaps_1d, verify_zero_pixel_cracking_1d, BenchmarkTimer};

#[test]
fn test_stress_10k_elements_subpixel_snapping_continuous_zero_gaps() {
    let timer = BenchmarkTimer::start();
    let total_elements = 10_000;

    let denominators = [3, 7, 11, 13, 17, 19, 23, 29, 31];
    let mut slices = Vec::with_capacity(total_elements);

    let mut current_origin = LayoutUnit::ZERO;

    for i in 0..total_elements {
        let denom = denominators[i % denominators.len()];
        // Divide 100px pelo denominador em base-60 LayoutUnit (6.000 au / denom)
        let raw_size = 6000 / denom;
        let size = LayoutUnit::from_raw(raw_size);

        slices.push((current_origin, size));
        current_origin += size;
    }

    assert_eq!(slices.len(), total_elements);

    // Valida através do verificador formal de zero pixel cracking
    let result = verify_zero_pixel_cracking_1d(&slices);
    assert!(
        result.is_ok(),
        "Subpixel snapping violation in 10,000 elements: {:?}",
        result
    );

    // Valida o Teorema de Adjacência par a par manualmente
    let mut detected_gaps = 0;
    let mut detected_overlaps = 0;

    for i in 1..total_elements {
        let (prev_orig, prev_size) = snap_box(slices[i - 1].0, slices[i - 1].1);
        let (curr_orig, _curr_size) = snap_box(slices[i].0, slices[i].1);

        let prev_end = prev_orig + prev_size;
        if curr_orig > prev_end {
            detected_gaps += 1;
        } else if curr_orig < prev_end {
            detected_overlaps += 1;
        }
    }

    // Asserções estritas normativas
    assert_eq!(detected_gaps, 0, "Detected {} 1px gaps", detected_gaps);
    assert_eq!(
        detected_overlaps, 0,
        "Detected {} 1px overlaps",
        detected_overlaps
    );

    let report = timer.report(total_elements, "Subpixel_Snapping_10k_elements");
    // SLA de performance: < 50 ms para 10.000 elementos
    assert!(
        report.elapsed_secs < 0.050,
        "Subpixel snapping 10k verification took {:.4}s (exceeded 50 ms SLA)",
        report.elapsed_secs
    );
}

#[test]
fn test_stress_multi_device_pixel_ratios_snapping() {
    // Fatores de escala DPR comuns em monitores modernos:
    // 100%, 125%, 150%, 175%, 200%, 250%, 300%
    let scales = [1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];

    for &dpr in &scales {
        let count = 1_000;
        let mut slices = Vec::with_capacity(count);
        let mut curr_orig = LayoutUnit::ZERO;

        for _i in 0..count {
            // Tamanho subpixel escalonado por DPR
            let px: f64 = 10.0 / dpr;
            let size = LayoutUnit::from_raw(((px * 60.0).round()) as i32);
            slices.push((curr_orig, size));
            curr_orig += size;
        }

        assert_zero_gaps_1d(&slices);
    }
}

#[test]
fn test_stress_2d_grid_logical_rect_snapping_10k_boxes() {
    let timer = BenchmarkTimer::start();
    let rows = 100;
    let cols = 100;
    let total_boxes = rows * cols; // 10.000 caixas

    let col_w = LayoutUnit::from_raw(2000); // 33.333px
    let row_h = LayoutUnit::from_raw(1500); // 25.000px

    let mut snapped_boxes = Vec::with_capacity(total_boxes);

    for r in 0..rows {
        let y = LayoutUnit::from_raw(r as i32 * 1500);
        for c in 0..cols {
            let x = LayoutUnit::from_raw(c as i32 * 2000);
            let rect = LogicalRect::new(x, y, col_w, row_h);
            let snapped = snap_logical_rect(&rect);
            snapped_boxes.push((r, c, snapped));
        }
    }

    assert_eq!(snapped_boxes.len(), total_boxes);

    // Valida que em qualquer linha horizontal, vizinhos adjacentes compartilham bordas inteiras x exatas
    for r in 0..rows {
        for c in 1..cols {
            let prev_idx = r * cols + (c - 1);
            let curr_idx = r * cols + c;

            let prev = &snapped_boxes[prev_idx].2;
            let curr = &snapped_boxes[curr_idx].2;

            let prev_right = prev.origin.x + prev.size.width;
            assert_eq!(
                curr.origin.x, prev_right,
                "Horizontal cracking between (row {}, col {}) and col {}: prev_right={}, curr_left={}",
                r, c - 1, c, prev_right, curr.origin.x
            );
        }
    }

    // Valida que em qualquer coluna vertical, vizinhos adjacentes compartilham bordas inteiras y exatas
    for c in 0..cols {
        for r in 1..rows {
            let prev_idx = (r - 1) * cols + c;
            let curr_idx = r * cols + c;

            let prev = &snapped_boxes[prev_idx].2;
            let curr = &snapped_boxes[curr_idx].2;

            let prev_bottom = prev.origin.y + prev.size.height;
            assert_eq!(
                curr.origin.y, prev_bottom,
                "Vertical cracking between (col {}, row {}) and row {}: prev_bottom={}, curr_top={}",
                c, r - 1, r, prev_bottom, curr.origin.y
            );
        }
    }

    let report = timer.report(total_boxes, "2D_Grid_Snapping_10k_boxes");
    assert!(
        report.elapsed_secs < 0.050,
        "2D grid snapping took {:.4}s (exceeded 50 ms SLA)",
        report.elapsed_secs
    );
}
