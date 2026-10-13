//! # Posicionamento em Matriz 2D Grid CSS L1 (`placement.rs`)
//!
//! Implementa o algoritmo de posicionamento de itens em grade conforme W3C CSS Grid L1 §8:
//! - Posicionamento explícito (`grid-column`, `grid-row`, `span`)
//! - Algoritmo de posicionamento automático (*auto-placement*) via `grid-auto-flow`:
//!   - `row`: linha por linha
//!   - `column`: coluna por coluna
//!   - `dense`, `row-dense`, `column-dense`: preenchimento de lacunas anteriores
//! - Expansão dinâmica de trilhas implícitas na matriz 2D

use ace_style::values::enums::GridAutoFlow;

/// Especificação de posicionamento de uma linha ou coluna.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridPlacement {
    /// Posição automática.
    Auto,
    /// Linha explícita (1-based, conforme CSS).
    Line(i32),
    /// Extensão de trilhas (*span*).
    Span(u16),
}

/// Posição resolvida em coordenadas 0-indexed na matriz de grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridItemPlacement {
    pub col_start: usize,
    pub col_end: usize,
    pub row_start: usize,
    pub row_end: usize,
}

impl GridItemPlacement {
    #[inline]
    pub fn col_span(&self) -> usize {
        self.col_end.saturating_sub(self.col_start).max(1)
    }

    #[inline]
    pub fn row_span(&self) -> usize {
        self.row_end.saturating_sub(self.row_start).max(1)
    }
}

/// Especificação de posicionamento de entrada para um item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPlacementSpec {
    pub col_start: GridPlacement,
    pub col_end: GridPlacement,
    pub row_start: GridPlacement,
    pub row_end: GridPlacement,
}

impl ItemPlacementSpec {
    pub fn auto() -> Self {
        Self {
            col_start: GridPlacement::Auto,
            col_end: GridPlacement::Auto,
            row_start: GridPlacement::Auto,
            row_end: GridPlacement::Auto,
        }
    }

    pub fn with_spans(col_span: u16, row_span: u16) -> Self {
        Self {
            col_start: GridPlacement::Auto,
            col_end: GridPlacement::Span(col_span),
            row_start: GridPlacement::Auto,
            row_end: GridPlacement::Span(row_span),
        }
    }

    pub fn explicit(col: i32, row: i32) -> Self {
        Self {
            col_start: GridPlacement::Line(col),
            col_end: GridPlacement::Line(col + 1),
            row_start: GridPlacement::Line(row),
            row_end: GridPlacement::Line(row + 1),
        }
    }

    pub fn is_explicit(&self) -> bool {
        matches!(self.col_start, GridPlacement::Line(_))
            && matches!(self.row_start, GridPlacement::Line(_))
    }
}

/// Matriz bidimensional de ocupação para resolução de posicionamento.
#[derive(Debug, Clone)]
pub struct GridMatrix {
    /// Ocupação [linha][coluna].
    occupied: Vec<Vec<bool>>,
    cols_count: usize,
    rows_count: usize,
}

impl GridMatrix {
    pub fn new(initial_cols: usize, initial_rows: usize) -> Self {
        let cols = initial_cols.max(1);
        let rows = initial_rows.max(1);
        Self {
            occupied: vec![vec![false; cols]; rows],
            cols_count: cols,
            rows_count: rows,
        }
    }

    /// Garante que a matriz tenha capacidade mínima de linhas e colunas.
    pub fn ensure_capacity(&mut self, req_cols: usize, req_rows: usize) {
        if req_cols > self.cols_count {
            for row in self.occupied.iter_mut() {
                row.resize(req_cols, false);
            }
            self.cols_count = req_cols;
        }

        if req_rows > self.rows_count {
            for _ in self.rows_count..req_rows {
                self.occupied.push(vec![false; self.cols_count]);
            }
            self.rows_count = req_rows;
        }
    }

    /// Verifica se uma área retangular está livre na matriz.
    pub fn is_area_free(
        &self,
        col_start: usize,
        col_end: usize,
        row_start: usize,
        row_end: usize,
    ) -> bool {
        for r in row_start..row_end {
            if r >= self.rows_count {
                continue;
            }
            for c in col_start..col_end {
                if c < self.cols_count && self.occupied[r][c] {
                    return false;
                }
            }
        }
        true
    }

    /// Marca uma área retangular como ocupada.
    pub fn mark_occupied(
        &mut self,
        col_start: usize,
        col_end: usize,
        row_start: usize,
        row_end: usize,
    ) {
        self.ensure_capacity(col_end, row_end);
        for r in row_start..row_end {
            for c in col_start..col_end {
                self.occupied[r][c] = true;
            }
        }
    }
}

/// Executa o algoritmo completo de posicionamento em grade.
///
/// Retorna o vetor de posicionamentos resolvidos e o tamanho final da matriz (cols, rows).
pub fn place_grid_items(
    items: &[ItemPlacementSpec],
    explicit_cols: usize,
    explicit_rows: usize,
    auto_flow: GridAutoFlow,
) -> (Vec<GridItemPlacement>, usize, usize) {
    let mut matrix = GridMatrix::new(explicit_cols, explicit_rows);
    let mut results = vec![None; items.len()];

    // 1. Processa itens com posicionamento explícito em ambas as dimensões
    for (i, spec) in items.iter().enumerate() {
        if let (GridPlacement::Line(c_line), GridPlacement::Line(r_line)) =
            (spec.col_start, spec.row_start)
        {
            let col_start = (c_line - 1).max(0) as usize;
            let col_end = match spec.col_end {
                GridPlacement::Line(end) => (end - 1).max(0) as usize,
                GridPlacement::Span(s) => col_start + (s as usize).max(1),
                GridPlacement::Auto => col_start + 1,
            };

            let row_start = (r_line - 1).max(0) as usize;
            let row_end = match spec.row_end {
                GridPlacement::Line(end) => (end - 1).max(0) as usize,
                GridPlacement::Span(s) => row_start + (s as usize).max(1),
                GridPlacement::Auto => row_start + 1,
            };

            let placement = GridItemPlacement {
                col_start,
                col_end: col_end.max(col_start + 1),
                row_start,
                row_end: row_end.max(row_start + 1),
            };

            matrix.mark_occupied(
                placement.col_start,
                placement.col_end,
                placement.row_start,
                placement.row_end,
            );
            results[i] = Some(placement);
        }
    }

    // 2. Processa itens auto-posicionados
    let is_dense = matches!(
        auto_flow,
        GridAutoFlow::Dense | GridAutoFlow::RowDense | GridAutoFlow::ColumnDense
    );
    let is_column_flow = matches!(auto_flow, GridAutoFlow::Column | GridAutoFlow::ColumnDense);

    let mut cursor_row = 0usize;
    let mut cursor_col = 0usize;

    for (i, spec) in items.iter().enumerate() {
        if results[i].is_some() {
            continue;
        }

        let col_span = match spec.col_end {
            GridPlacement::Span(s) => (s as usize).max(1),
            _ => 1,
        };
        let row_span = match spec.row_end {
            GridPlacement::Span(s) => (s as usize).max(1),
            _ => 1,
        };

        if is_dense {
            cursor_row = 0;
            cursor_col = 0;
        }

        let placement = if !is_column_flow {
            // Fluxo por linha (Row Flow)
            let mut found = None;
            let mut r = cursor_row;
            let mut c = cursor_col;
            let col_boundary = col_span.max(explicit_cols.max(1));

            while found.is_none() {
                matrix.ensure_capacity(col_boundary, r + row_span);

                while c + col_span <= col_boundary {
                    if matrix.is_area_free(c, c + col_span, r, r + row_span) {
                        found = Some(GridItemPlacement {
                            col_start: c,
                            col_end: c + col_span,
                            row_start: r,
                            row_end: r + row_span,
                        });
                        break;
                    }
                    c += 1;
                }

                if found.is_none() {
                    r += 1;
                    c = 0;
                }
            }

            let p = found.unwrap();
            if !is_dense {
                cursor_row = p.row_start;
                cursor_col = p.col_end;
            }
            p
        } else {
            // Fluxo por coluna (Column Flow)
            let mut found = None;
            let mut r = cursor_row;
            let mut c = cursor_col;
            let row_boundary = row_span.max(explicit_rows.max(1));

            while found.is_none() {
                matrix.ensure_capacity(c + col_span, row_boundary);

                while r + row_span <= row_boundary {
                    if matrix.is_area_free(c, c + col_span, r, r + row_span) {
                        found = Some(GridItemPlacement {
                            col_start: c,
                            col_end: c + col_span,
                            row_start: r,
                            row_end: r + row_span,
                        });
                        break;
                    }
                    r += 1;
                }

                if found.is_none() {
                    c += 1;
                    r = 0;
                }
            }

            let p = found.unwrap();
            if !is_dense {
                cursor_col = p.col_start;
                cursor_row = p.row_end;
            }
            p
        };

        matrix.mark_occupied(
            placement.col_start,
            placement.col_end,
            placement.row_start,
            placement.row_end,
        );
        results[i] = Some(placement);
    }

    let final_placements: Vec<GridItemPlacement> = results.into_iter().flatten().collect();

    // Determina dimensões totais finais da grade
    let max_col = final_placements
        .iter()
        .map(|p| p.col_end)
        .max()
        .unwrap_or(0)
        .max(explicit_cols);

    let max_row = final_placements
        .iter()
        .map(|p| p.row_end)
        .max()
        .unwrap_or(0)
        .max(explicit_rows);

    (final_placements, max_col, max_row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_placement_explicit() {
        let specs = vec![
            ItemPlacementSpec::explicit(1, 1),
            ItemPlacementSpec::explicit(2, 1),
            ItemPlacementSpec::explicit(1, 2),
        ];

        let (placements, cols, rows) = place_grid_items(&specs, 2, 2, GridAutoFlow::Row);
        assert_eq!(placements.len(), 3);
        assert_eq!(
            placements[0],
            GridItemPlacement {
                col_start: 0,
                col_end: 1,
                row_start: 0,
                row_end: 1
            }
        );
        assert_eq!(
            placements[1],
            GridItemPlacement {
                col_start: 1,
                col_end: 2,
                row_start: 0,
                row_end: 1
            }
        );
        assert_eq!(
            placements[2],
            GridItemPlacement {
                col_start: 0,
                col_end: 1,
                row_start: 1,
                row_end: 2
            }
        );
        assert_eq!(cols, 2);
        assert_eq!(rows, 2);
    }

    #[test]
    fn test_grid_placement_auto_flow_row_with_spans() {
        let specs = vec![
            ItemPlacementSpec::with_spans(2, 1), // item 1: span 2 cols
            ItemPlacementSpec::auto(),           // item 2: 1x1
            ItemPlacementSpec::auto(),           // item 3: 1x1
        ];

        let (placements, cols, rows) = place_grid_items(&specs, 2, 1, GridAutoFlow::Row);
        assert_eq!(placements.len(), 3);
        // Item 1 occupies (row 0, col 0..2)
        assert_eq!(
            placements[0],
            GridItemPlacement {
                col_start: 0,
                col_end: 2,
                row_start: 0,
                row_end: 1
            }
        );
        // Item 2 wraps to row 1, col 0..1
        assert_eq!(
            placements[1],
            GridItemPlacement {
                col_start: 0,
                col_end: 1,
                row_start: 1,
                row_end: 2
            }
        );
        // Item 3 placed at row 1, col 1..2
        assert_eq!(
            placements[2],
            GridItemPlacement {
                col_start: 1,
                col_end: 2,
                row_start: 1,
                row_end: 2
            }
        );
        assert_eq!(cols, 2);
        assert_eq!(rows, 2);
    }

    #[test]
    fn test_grid_placement_auto_flow_dense() {
        let specs = vec![
            ItemPlacementSpec::explicit(2, 1), // item 0 explicitly at (col 1, row 0)
            ItemPlacementSpec::with_spans(2, 1), // item 1 needs 2 cols -> goes to row 1 (cols 0..2)
            ItemPlacementSpec::auto(), // item 2 needs 1 col -> dense backtracks to fill hole at row 0, col 0!
        ];

        let (placements, _, _) = place_grid_items(&specs, 2, 2, GridAutoFlow::Dense);
        assert_eq!(
            placements[0],
            GridItemPlacement {
                col_start: 1,
                col_end: 2,
                row_start: 0,
                row_end: 1
            }
        );
        assert_eq!(
            placements[1],
            GridItemPlacement {
                col_start: 0,
                col_end: 2,
                row_start: 1,
                row_end: 2
            }
        );
        // Hole filled at row 0, col 0
        assert_eq!(
            placements[2],
            GridItemPlacement {
                col_start: 0,
                col_end: 1,
                row_start: 0,
                row_end: 1
            }
        );
    }
}
