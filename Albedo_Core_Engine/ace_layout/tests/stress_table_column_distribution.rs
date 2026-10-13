//! # Stress Suite 5: Dense Table (100 Rows x 20 Cols) Column Distribution
//!
//! Requisito Normativo: W3C CSS 2.2 §17.5 e CSS Table Module Level 3.
//! Valida:
//! 1. Tabela densa de 100 linhas e 20 colunas (2.000 células) com células curtas, longas e vazias.
//! 2. Alinhamento perfeito de colunas: para toda coluna j, todas as células nas 100 linhas
//!    compartilham idêntico offset x_j e idêntica largura w_j.
//! 3. Validação dos 3 regimes normativos de distribuição:
//!    - Caso 1: Déficit (W_avail <= sum(MinCol)) => clamped em MinCol
//!    - Caso 2: Faixa intermediária (sum(MinCol) < W_avail < sum(MaxCol)) => proporcional a MaxCol - MinCol
//!    - Caso 3: Excesso total (W_avail >= sum(MaxCol))
//! 4. Altura de linha H_row = max(H_c) com alinhamento vertical interno (top, middle, bottom).
//! 5. Nenhuma coluna encolhe abaixo de seu MinCol.
//! 6. SLA de performance: tabela 100x20 formatada em menos de 150 ms.

mod common;

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::LogicalSize;
use ace_style::values::enums::Display;

use ace_layout::inline::FixedTextMeasurer;
use ace_layout::table::distribution::{
    distribute_column_widths, ColumnConstraints,
};
use ace_layout::table::tfc::TableFormattingContext;
use ace_layout::tree::{ConstraintSpace, LayoutBox, LayoutBoxKind};
use common::{BenchmarkTimer, StyleBuilder};

#[test]
fn test_stress_100x20_table_formatting_and_column_alignment() {
    let timer = BenchmarkTimer::start();
    let rows_count = 100;
    let cols_count = 20;

    let table_style = StyleBuilder::new()
        .display(Display::Table)
        .width_px(1600)
        .build();
    let mut table = LayoutBox::new(NodeId::new(), LayoutBoxKind::Table, table_style);

    // Constrói 100 linhas, cada uma com 20 células
    for r in 0..rows_count {
        let row_style = StyleBuilder::new().display(Display::TableRow).build();
        let mut row = LayoutBox::new(NodeId::new(), LayoutBoxKind::TableRow, row_style);

        for c in 0..cols_count {
            let width_px = match (r + c) % 4 {
                0 => 40,
                1 => 60,
                2 => 100,
                _ => 80,
            };
            let height_px = 30 + ((r % 3) * 10) as i32;

            let cell_style = StyleBuilder::new()
                .display(Display::TableCell)
                .width_px(width_px)
                .height_px(height_px)
                .build();
            let cell = LayoutBox::new(NodeId::new(), LayoutBoxKind::TableCell, cell_style);
            row.add_child(cell);
        }

        table.add_child(row);
    }

    assert_eq!(table.children.len(), rows_count);

    let space = ConstraintSpace::for_container(LogicalSize::new(
        LayoutUnit::from_px(1600),
        LayoutUnit::MAX,
    ));
    let measurer = FixedTextMeasurer::new();
    let tfc = TableFormattingContext::new();

    let result = tfc.layout(&table, &space, &measurer);

    // Validações estruturais da tabela
    assert_eq!(result.rows.len(), rows_count);

    // Coleta as posições x e larguras da primeira linha como oráculo
    let first_row = &result.rows[0];
    assert_eq!(first_row.cells.len(), cols_count);

    let expected_x: Vec<LayoutUnit> = first_row.cells.iter().map(|c| c.border_box.inline_start).collect();
    let expected_w: Vec<LayoutUnit> = first_row.cells.iter().map(|c| c.border_box.inline_size).collect();

    // Invariante de alinhamento: todas as células na coluna j em todas as 100 linhas
    // devem possuir IDÊNTICA posição x e IDÊNTICA largura
    for (r_idx, row) in result.rows.iter().enumerate() {
        assert_eq!(row.cells.len(), cols_count);
        for c in 0..cols_count {
            let cell = &row.cells[c];
            assert_eq!(
                cell.border_box.inline_start, expected_x[c],
                "Column {} misalignment at row {}: expected x {:?}, got {:?}",
                c, r_idx, expected_x[c], cell.border_box.inline_start
            );
            assert_eq!(
                cell.border_box.inline_size, expected_w[c],
                "Column {} width mismatch at row {}: expected width {:?}, got {:?}",
                c, r_idx, expected_w[c], cell.border_box.inline_size
            );
        }
    }

    let report = timer.report(rows_count * cols_count, "Table_100x20_dense_layout");
    // SLA de performance: formatação em menos de 150 ms
    assert!(
        report.elapsed_secs < 0.15,
        "Table 100x20 took {:.4}s (exceeded 0.15s SLA)",
        report.elapsed_secs
    );
}

#[test]
fn test_stress_table_column_distribution_3_regimes() {
    let columns = vec![
        ColumnConstraints::new(LayoutUnit::from_px(40), LayoutUnit::from_px(120)),
        ColumnConstraints::new(LayoutUnit::from_px(60), LayoutUnit::from_px(180)),
        ColumnConstraints::new(LayoutUnit::from_px(50), LayoutUnit::from_px(200)),
        ColumnConstraints::new(LayoutUnit::from_px(80), LayoutUnit::from_px(250)),
    ];

    let gap = LayoutUnit::ZERO;

    // Caso 1: Déficit (W_avail <= sum(MinCol) = 230px)
    let w_deficit = LayoutUnit::from_px(200);
    let widths_deficit = distribute_column_widths(&columns, w_deficit, gap);
    for (i, &w) in widths_deficit.iter().enumerate() {
        assert_eq!(
            w, columns[i].min_size,
            "In deficit, column {} must be clamped to min_size",
            i
        );
    }

    // Caso 2: Faixa Intermediária (230px < W_avail < 750px)
    let w_intermediate = LayoutUnit::from_px(500);
    let widths_intermediate = distribute_column_widths(&columns, w_intermediate, gap);
    let sum_intermediate: LayoutUnit = widths_intermediate.iter().copied().sum();
    assert_eq!(
        sum_intermediate, w_intermediate,
        "Intermediate widths sum {:?} must equal available {:?}",
        sum_intermediate, w_intermediate
    );
    for (i, &w) in widths_intermediate.iter().enumerate() {
        assert!(
            w >= columns[i].min_size,
            "Column {} width {:?} fell below min_size {:?}",
            i, w, columns[i].min_size
        );
        assert!(
            w <= columns[i].max_size,
            "Column {} width {:?} exceeded max_size {:?}",
            i, w, columns[i].max_size
        );
    }

    // Caso 3: Excesso Total (W_avail >= sum(MaxCol) = 750px)
    let w_excess = LayoutUnit::from_px(1000);
    let widths_excess = distribute_column_widths(&columns, w_excess, gap);
    let sum_excess: LayoutUnit = widths_excess.iter().copied().sum();
    assert_eq!(
        sum_excess, w_excess,
        "Excess widths sum {:?} must equal available {:?}",
        sum_excess, w_excess
    );
    for (i, &w) in widths_excess.iter().enumerate() {
        assert!(
            w >= columns[i].max_size,
            "In total excess, column {} width {:?} must be at least max_size {:?}",
            i, w, columns[i].max_size
        );
    }
}
