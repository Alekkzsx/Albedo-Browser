//! # Distribuição Automática de Colunas em Tabelas (`distribution.rs`)
//!
//! Implementa o algoritmo de layout automático de tabelas conforme W3C CSS 2.2 §17.5.2
//! e CSS Table Module Level 3:
//! - Coleta de $min_c$ e $max_c$ por célula (`td`, `th`)
//! - Agregação de restrições por coluna $\text{MinCol}_j$ e $\text{MaxCol}_j$
//! - Distribuição ponderada de largura sob os 3 regimes normativos:
//!   - Caso 1: $W_{\text{avail}} \le \sum MinCol$ (min-clamped / proporcional)
//!   - Caso 2: $\sum MinCol < W_{\text{avail}} < \sum MaxCol$ (proporcional a $MaxCol - MinCol$)
//!   - Caso 3: $W_{\text{avail}} \ge \sum MaxCol$ (proporcional a $MaxCol$)

use ace_core::math::layout_unit::LayoutUnit;

/// Restrições intrínsecas de dimensionamento de uma célula de tabela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellConstraints {
    /// Tamanho mínimo indispensável sem quebras forçadas (min-content).
    pub min_content: LayoutUnit,
    /// Tamanho máximo contínuo em linha única (max-content).
    pub max_content: LayoutUnit,
    /// Quantidade de colunas estendidas (`colspan`).
    pub colspan: usize,
}

impl CellConstraints {
    pub fn new(min_content: LayoutUnit, max_content: LayoutUnit, colspan: usize) -> Self {
        Self {
            min_content,
            max_content: max_content.max(min_content),
            colspan: colspan.max(1),
        }
    }
}

/// Restrições agregadas de uma coluna da tabela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColumnConstraints {
    /// Largura mínima exigida pela coluna.
    pub min_size: LayoutUnit,
    /// Largura máxima desejada pela coluna.
    pub max_size: LayoutUnit,
}

impl ColumnConstraints {
    pub fn new(min_size: LayoutUnit, max_size: LayoutUnit) -> Self {
        Self {
            min_size,
            max_size: max_size.max(min_size),
        }
    }
}

/// Agrega as restrições de cada coluna a partir de uma matriz de células `cells[row][col]`.
pub fn aggregate_column_constraints(
    grid_cells: &[Vec<Option<CellConstraints>>],
    columns_count: usize,
) -> Vec<ColumnConstraints> {
    if columns_count == 0 {
        return Vec::new();
    }

    let mut cols = vec![ColumnConstraints::default(); columns_count];

    // Passo 1: Processa células com colspan == 1
    for row in grid_cells {
        for (c, cell_opt) in row.iter().enumerate() {
            if c >= columns_count {
                break;
            }
            if let Some(cell) = cell_opt {
                if cell.colspan == 1 {
                    cols[c].min_size = cols[c].min_size.max(cell.min_content);
                    cols[c].max_size = cols[c].max_size.max(cell.max_content);
                }
            }
        }
    }

    // Passo 2: Processa células com colspan > 1 (distribui déficit se houver)
    for row in grid_cells {
        for (c, cell_opt) in row.iter().enumerate() {
            if let Some(cell) = cell_opt {
                if cell.colspan > 1 {
                    let end_col = (c + cell.colspan).min(columns_count);
                    let span_len = end_col.saturating_sub(c);
                    if span_len == 0 {
                        continue;
                    }

                    // Verifica min_content
                    let current_min_sum: LayoutUnit = cols[c..end_col]
                        .iter()
                        .fold(LayoutUnit::ZERO, |acc, col| acc.saturating_add(col.min_size));

                    if cell.min_content > current_min_sum {
                        let deficit = cell.min_content.saturating_sub(current_min_sum);
                        let per_col_raw = deficit.raw() / (span_len as i32);
                        for col in &mut cols[c..end_col] {
                            col.min_size = col.min_size.saturating_add(LayoutUnit::from_raw(per_col_raw));
                        }
                    }

                    // Verifica max_content
                    let current_max_sum: LayoutUnit = cols[c..end_col]
                        .iter()
                        .fold(LayoutUnit::ZERO, |acc, col| acc.saturating_add(col.max_size));

                    if cell.max_content > current_max_sum {
                        let deficit = cell.max_content.saturating_sub(current_max_sum);
                        let per_col_raw = deficit.raw() / (span_len as i32);
                        for col in &mut cols[c..end_col] {
                            col.max_size = col.max_size.saturating_add(LayoutUnit::from_raw(per_col_raw));
                        }
                    }
                }
            }
        }
    }

    // Invariante: max_size >= min_size para todas as colunas
    for col in &mut cols {
        if col.max_size < col.min_size {
            col.max_size = col.min_size;
        }
    }

    cols
}

/// Distribui a largura disponível entre as colunas conforme W3C CSS 2.2 §17.5.2.
pub fn distribute_column_widths(
    columns: &[ColumnConstraints],
    available_width: LayoutUnit,
    col_gap: LayoutUnit,
) -> Vec<LayoutUnit> {
    let count = columns.len();
    if count == 0 {
        return Vec::new();
    }

    let total_gaps = if count > 1 {
        col_gap.saturating_mul((count - 1) as i32)
    } else {
        LayoutUnit::ZERO
    };

    let net_available = available_width.saturating_sub(total_gaps).max(LayoutUnit::ZERO);

    let sum_min: LayoutUnit = columns
        .iter()
        .fold(LayoutUnit::ZERO, |acc, c| acc.saturating_add(c.min_size));

    let sum_max: LayoutUnit = columns
        .iter()
        .fold(LayoutUnit::ZERO, |acc, c| acc.saturating_add(c.max_size));

    let mut result = vec![LayoutUnit::ZERO; count];

    if net_available <= sum_min {
        // Caso 1: Déficit de Espaço ($W_{avail} \le \sum MinCol$)
        // Mantém min-clamped para satisfazer o tamanho mínimo essencial de conteúdo
        for (i, col) in columns.iter().enumerate() {
            result[i] = col.min_size;
        }
    } else if net_available < sum_max {
        // Caso 2: Faixa Intermediária ($\sum MinCol < W_{avail} < \sum MaxCol$)
        let excess_raw = net_available.raw().saturating_sub(sum_min.raw());

        let diff_sum: i64 = columns
            .iter()
            .map(|c| (c.max_size.raw() - c.min_size.raw()) as i64)
            .sum();

        let mut allocated_excess = 0i32;

        for (i, col) in columns.iter().enumerate() {
            let diff = (col.max_size.raw() - col.min_size.raw()) as f64;
            let share = if diff_sum > 0 {
                ((excess_raw as f64) * (diff / diff_sum as f64)).round() as i32
            } else {
                excess_raw / (count as i32)
            };

            result[i] = LayoutUnit::from_raw(col.min_size.raw() + share);
            allocated_excess += share;
        }

        // Ajuste de conservação exata
        let leftover = excess_raw - allocated_excess;
        if leftover != 0 && leftover.abs() <= 5 {
            if let Some((idx, _)) = columns.iter().enumerate().max_by_key(|(_, c)| c.max_size.raw() - c.min_size.raw()) {
                result[idx] = LayoutUnit::from_raw(result[idx].raw() + leftover);
            }
        }
    } else {
        // Caso 3: Excesso Total de Espaço ($W_{avail} \ge \sum MaxCol$)
        let excess_raw = net_available.raw().saturating_sub(sum_max.raw());
        let sum_max_raw = sum_max.raw() as f64;

        let mut allocated_excess = 0i32;

        for (i, col) in columns.iter().enumerate() {
            let share = if sum_max_raw > 0.0 {
                ((excess_raw as f64) * (col.max_size.raw() as f64 / sum_max_raw)).round() as i32
            } else {
                excess_raw / (count as i32)
            };

            result[i] = LayoutUnit::from_raw(col.max_size.raw() + share);
            allocated_excess += share;
        }

        // Ajuste de conservação exata
        let leftover = excess_raw - allocated_excess;
        if leftover != 0 && leftover.abs() <= 5 {
            if let Some((idx, _)) = columns.iter().enumerate().max_by_key(|(_, c)| c.max_size.raw()) {
                result[idx] = LayoutUnit::from_raw(result[idx].raw() + leftover);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_distribution_case1_deficit() {
        let cols = vec![
            ColumnConstraints::new(LayoutUnit::from_px(100), LayoutUnit::from_px(200)),
            ColumnConstraints::new(LayoutUnit::from_px(150), LayoutUnit::from_px(300)),
        ];

        // Sum min = 250px. Avail = 200px (déficit).
        let widths = distribute_column_widths(&cols, LayoutUnit::from_px(200), LayoutUnit::ZERO);
        assert_eq!(widths.len(), 2);
        assert_eq!(widths[0], LayoutUnit::from_px(100));
        assert_eq!(widths[1], LayoutUnit::from_px(150));
    }

    #[test]
    fn test_table_distribution_case2_intermediate() {
        let cols = vec![
            ColumnConstraints::new(LayoutUnit::from_px(50), LayoutUnit::from_px(150)), // diff = 100
            ColumnConstraints::new(LayoutUnit::from_px(50), LayoutUnit::from_px(250)), // diff = 200
        ];

        // Sum min = 100px. Sum max = 400px.
        // Avail = 250px. Excess = 150px.
        // Total diff = 300px.
        // Col 0 gets 50 + 150 * (100/300) = 50 + 50 = 100px.
        // Col 1 gets 50 + 150 * (200/300) = 50 + 100 = 150px.
        let widths = distribute_column_widths(&cols, LayoutUnit::from_px(250), LayoutUnit::ZERO);
        assert_eq!(widths.len(), 2);
        assert_eq!(widths[0], LayoutUnit::from_px(100));
        assert_eq!(widths[1], LayoutUnit::from_px(150));
    }

    #[test]
    fn test_table_distribution_case3_excess() {
        let cols = vec![
            ColumnConstraints::new(LayoutUnit::from_px(50), LayoutUnit::from_px(100)),
            ColumnConstraints::new(LayoutUnit::from_px(50), LayoutUnit::from_px(200)),
        ];

        // Sum max = 300px. Avail = 600px. Excess = 300px.
        // Col 0 gets 100 + 300 * (100/300) = 200px.
        // Col 1 gets 200 + 300 * (200/300) = 400px.
        let widths = distribute_column_widths(&cols, LayoutUnit::from_px(600), LayoutUnit::ZERO);
        assert_eq!(widths.len(), 2);
        assert_eq!(widths[0], LayoutUnit::from_px(200));
        assert_eq!(widths[1], LayoutUnit::from_px(400));
    }

    #[test]
    fn test_column_aggregation_with_colspan() {
        let cells = vec![
            vec![
                Some(CellConstraints::new(LayoutUnit::from_px(40), LayoutUnit::from_px(80), 1)),
                Some(CellConstraints::new(LayoutUnit::from_px(60), LayoutUnit::from_px(120), 1)),
            ],
            vec![
                // colspan 2 across both cols: min 150px, max 250px
                Some(CellConstraints::new(LayoutUnit::from_px(150), LayoutUnit::from_px(250), 2)),
                None,
            ],
        ];

        let cols = aggregate_column_constraints(&cells, 2);
        // Original min sum = 100px. Cell colspan min = 150px (deficit = 50px -> +25px each).
        assert_eq!(cols[0].min_size, LayoutUnit::from_px(65));
        assert_eq!(cols[1].min_size, LayoutUnit::from_px(85));
    }
}
