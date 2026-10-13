//! # Table Formatting Context (`TableFormattingContext` / TFC)
//!
//! Orquestrador de layout de tabelas conforme W3C CSS 2.2 §17 e CSS Table Module Level 3:
//! - Normalização estrutural de linhas (`TableRow`) e células (`TableCell`)
//! - Mineração e agregação de restrições de colunas
//! - Distribuição automática de larguras de colunas (`distribute_column_widths`)
//! - Dimensionamento de alturas de linhas baseado no conteúdo das células
//! - Alinhamento vertical interno de células (`vertical-align: top | middle | bottom`)

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{LogicalRect, LogicalSides, LogicalSize};
use ace_style::values::enums::{BoxSizing, Display};

use crate::block::bfc::{BfcLayoutResult, BlockFormattingContext};
use crate::inline::line_box::VerticalAlign;
use crate::inline::text_measurer::TextMeasurer;
use crate::table::distribution::{
    aggregate_column_constraints, distribute_column_widths, CellConstraints, ColumnConstraints,
};
use crate::tree::constraint_space::{AvailableSize, ConstraintSpace};
use crate::tree::layout_box::{LayoutBox, LayoutBoxKind};

#[inline]
fn inline_sum(sides: &LogicalSides<LayoutUnit>) -> LayoutUnit {
    sides.inline_start.saturating_add(sides.inline_end)
}

#[inline]
fn block_sum(sides: &LogicalSides<LayoutUnit>) -> LayoutUnit {
    sides.block_start.saturating_add(sides.block_end)
}

/// Resultado do layout de uma célula individual de tabela.
#[derive(Debug, Clone, PartialEq)]
pub struct TableCellResult {
    pub box_id: NodeId,
    pub col_index: usize,
    pub colspan: usize,
    pub border_box: LogicalRect<LayoutUnit>,
    pub margin_box: LogicalRect<LayoutUnit>,
    pub padding_box: LogicalRect<LayoutUnit>,
    pub content_box: LogicalRect<LayoutUnit>,
    pub margins: LogicalSides<LayoutUnit>,
    pub borders: LogicalSides<LayoutUnit>,
    pub paddings: LogicalSides<LayoutUnit>,
    pub vertical_align: VerticalAlign,
    pub bfc_result: Option<BfcLayoutResult>,
}

impl TableCellResult {
    #[inline]
    pub fn width(&self) -> LayoutUnit {
        self.border_box.inline_size
    }

    #[inline]
    pub fn height(&self) -> LayoutUnit {
        self.border_box.block_size
    }

    #[inline]
    pub fn content_width(&self) -> LayoutUnit {
        self.content_box.inline_size
    }

    #[inline]
    pub fn content_height(&self) -> LayoutUnit {
        self.content_box.block_size
    }
}

/// Resultado do layout de uma linha de tabela.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRowResult {
    pub box_id: NodeId,
    pub row_index: usize,
    pub border_box: LogicalRect<LayoutUnit>,
    pub cells: Vec<TableCellResult>,
}

impl TableRowResult {
    #[inline]
    pub fn width(&self) -> LayoutUnit {
        self.border_box.inline_size
    }

    #[inline]
    pub fn height(&self) -> LayoutUnit {
        self.border_box.block_size
    }
}

/// Resultado completo do layout de uma tabela (TFC).
#[derive(Debug, Clone, PartialEq)]
pub struct TableLayoutResult {
    pub box_id: NodeId,
    pub margin_box: LogicalRect<LayoutUnit>,
    pub border_box: LogicalRect<LayoutUnit>,
    pub padding_box: LogicalRect<LayoutUnit>,
    pub content_box: LogicalRect<LayoutUnit>,
    pub margins: LogicalSides<LayoutUnit>,
    pub borders: LogicalSides<LayoutUnit>,
    pub paddings: LogicalSides<LayoutUnit>,
    pub columns_count: usize,
    pub rows_count: usize,
    pub column_constraints: Vec<ColumnConstraints>,
    pub column_widths: Vec<LayoutUnit>,
    pub row_heights: Vec<LayoutUnit>,
    pub rows: Vec<TableRowResult>,
}

impl TableLayoutResult {
    #[inline]
    pub fn width(&self) -> LayoutUnit {
        self.border_box.inline_size
    }

    #[inline]
    pub fn height(&self) -> LayoutUnit {
        self.border_box.block_size
    }

    #[inline]
    pub fn content_width(&self) -> LayoutUnit {
        self.content_box.inline_size
    }

    #[inline]
    pub fn content_height(&self) -> LayoutUnit {
        self.content_box.block_size
    }
}

/// Motor de execução do Table Formatting Context (TFC).
#[derive(Debug, Clone, Default)]
pub struct TableFormattingContext;

impl TableFormattingContext {
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Executa o layout automático de tabela completo.
    pub fn layout(
        &self,
        node: &LayoutBox,
        space: &ConstraintSpace,
        measurer: &dyn TextMeasurer,
    ) -> TableLayoutResult {
        // 1. Paddings, borders e margens da tabela
        let paddings = LogicalSides::new(
            node.style.padding_left(),
            node.style.padding_right(),
            node.style.padding_top(),
            node.style.padding_bottom(),
        );

        let borders = LogicalSides::new(
            node.style.border_width().left,
            node.style.border_width().right,
            node.style.border_width().top,
            node.style.border_width().bottom,
        );

        let margins = LogicalSides::new(
            node.style.margin_left().unwrap_or(LayoutUnit::ZERO),
            node.style.margin_right().unwrap_or(LayoutUnit::ZERO),
            node.style.margin_top().unwrap_or(LayoutUnit::ZERO),
            node.style.margin_bottom().unwrap_or(LayoutUnit::ZERO),
        );

        let h_surrounding = inline_sum(&borders).saturating_add(inline_sum(&paddings));
        let v_surrounding = block_sum(&borders).saturating_add(block_sum(&paddings));

        // 2. Coleta linhas (`TableRow`) e células (`TableCell`)
        let mut row_boxes: Vec<&LayoutBox> = Vec::new();
        for child in &node.children {
            if child.style.display() == Display::None {
                continue;
            }
            if child.kind == LayoutBoxKind::TableRow {
                row_boxes.push(child);
            } else if child.kind == LayoutBoxKind::TableCell {
                // Célula avulsa: tratada como linha de 1 célula
                row_boxes.push(child);
            }
        }

        // Constrói a matriz de células
        let mut grid_cells: Vec<Vec<Option<(&LayoutBox, CellConstraints)>>> = Vec::new();
        let mut max_cols = 0usize;

        for &row in &row_boxes {
            let mut row_cells = Vec::new();
            if row.kind == LayoutBoxKind::TableRow {
                for cell in &row.children {
                    if cell.style.display() == Display::None {
                        continue;
                    }
                    let cell_constraints = compute_cell_constraints(cell);
                    row_cells.push(Some((cell, cell_constraints)));
                }
            } else {
                let cell_constraints = compute_cell_constraints(row);
                row_cells.push(Some((row, cell_constraints)));
            }
            max_cols = max_cols.max(row_cells.len());
            grid_cells.push(row_cells);
        }

        // Matriz normalizada para agregação
        let constraint_grid: Vec<Vec<Option<CellConstraints>>> = grid_cells
            .iter()
            .map(|r| r.iter().map(|c| c.map(|(_, cc)| cc)).collect())
            .collect();

        // 3. Agrega restrições de colunas
        let column_constraints = aggregate_column_constraints(&constraint_grid, max_cols);

        // 4. Determina a largura disponível para a tabela
        let table_avail_w = if let Some(w) = node.style.width() {
            match node.style.box_sizing() {
                BoxSizing::BorderBox => w.saturating_sub(h_surrounding).max(LayoutUnit::ZERO),
                BoxSizing::ContentBox => w,
            }
        } else if let Some(avail_w) = space.available_inline_size.to_definite() {
            avail_w
                .saturating_sub(inline_sum(&margins))
                .saturating_sub(h_surrounding)
                .max(LayoutUnit::ZERO)
        } else {
            // Indefinido: usa a soma dos max_size das colunas
            column_constraints
                .iter()
                .fold(LayoutUnit::ZERO, |acc, c| acc.saturating_add(c.max_size))
        };

        let col_gap = LayoutUnit::ZERO;

        // 5. Distribui a largura entre as colunas
        let column_widths = distribute_column_widths(&column_constraints, table_avail_w, col_gap);

        // Offsets horizontais das colunas
        let mut col_offsets = Vec::with_capacity(max_cols);
        let mut curr_x = LayoutUnit::ZERO;
        for &w in &column_widths {
            col_offsets.push(curr_x);
            curr_x = curr_x.saturating_add(w).saturating_add(col_gap);
        }

        // 6. Formata células e determina alturas das linhas
        let bfc_engine = BlockFormattingContext::new();
        let mut row_results = Vec::new();
        let mut row_heights = Vec::new();
        let mut curr_y = LayoutUnit::ZERO;

        for (r_idx, row_cells) in grid_cells.iter().enumerate() {
            let row_node = row_boxes[r_idx];

            let mut cell_meas: Vec<(TableCellResult, LayoutUnit)> = Vec::new();
            let mut max_cell_h = LayoutUnit::ZERO;

            let mut curr_c = 0usize;
            for &(cell_box, constraints) in row_cells.iter().flatten() {
                let c = curr_c;
                let colspan = constraints.colspan;
                let end_c = (c + colspan).min(max_cols);

                let cell_x = col_offsets.get(c).copied().unwrap_or(LayoutUnit::ZERO);
                let mut cell_w = LayoutUnit::ZERO;
                for col_idx in c..end_c {
                    cell_w = cell_w.saturating_add(column_widths.get(col_idx).copied().unwrap_or(LayoutUnit::ZERO));
                }

                    let cell_paddings = LogicalSides::new(
                        cell_box.style.padding_left(),
                        cell_box.style.padding_right(),
                        cell_box.style.padding_top(),
                        cell_box.style.padding_bottom(),
                    );
                    let cell_borders = LogicalSides::new(
                        cell_box.style.border_width().left,
                        cell_box.style.border_width().right,
                        cell_box.style.border_width().top,
                        cell_box.style.border_width().bottom,
                    );
                    let cell_margins = LogicalSides::new(
                        cell_box.style.margin_left().unwrap_or(LayoutUnit::ZERO),
                        cell_box.style.margin_right().unwrap_or(LayoutUnit::ZERO),
                        cell_box.style.margin_top().unwrap_or(LayoutUnit::ZERO),
                        cell_box.style.margin_bottom().unwrap_or(LayoutUnit::ZERO),
                    );

                    let cell_inner_w = cell_w
                        .saturating_sub(inline_sum(&cell_borders))
                        .saturating_sub(inline_sum(&cell_paddings))
                        .max(LayoutUnit::ZERO);

                    // Formata conteúdo interno da célula
                    let cell_space = ConstraintSpace::new(
                        AvailableSize::Definite(cell_inner_w),
                        AvailableSize::Indefinite,
                        LogicalSize::new(cell_inner_w, LayoutUnit::ZERO),
                    );

                    let bfc_res = bfc_engine.layout(cell_box, &cell_space, measurer);
                    let content_h = bfc_res.height();

                    let total_cell_h = if let Some(h) = cell_box.style.height() {
                        h
                    } else {
                        content_h
                            .saturating_add(block_sum(&cell_borders))
                            .saturating_add(block_sum(&cell_paddings))
                    };

                    max_cell_h = max_cell_h.max(total_cell_h);

                    let temp_res = TableCellResult {
                        box_id: cell_box.id,
                        col_index: c,
                        colspan,
                        border_box: LogicalRect::new(cell_x, LayoutUnit::ZERO, cell_w, total_cell_h),
                        margin_box: LogicalRect::default(),
                        padding_box: LogicalRect::default(),
                        content_box: LogicalRect::default(),
                        margins: cell_margins,
                        borders: cell_borders,
                        paddings: cell_paddings,
                        vertical_align: VerticalAlign::Middle,
                        bfc_result: Some(bfc_res),
                    };

                    cell_meas.push((temp_res, content_h));
                    curr_c += colspan;
            }

            let final_row_h = max_cell_h.max(LayoutUnit::from_px(20));
            row_heights.push(final_row_h);

            // 7. Alinhamento vertical interno das células (`vertical-align`)
            let mut finalized_cells = Vec::new();
            for (mut cell_res, content_h) in cell_meas {
                cell_res.border_box.block_start = curr_y;
                cell_res.border_box.block_size = final_row_h;

                let padding_top = cell_res.paddings.block_start;
                let border_top = cell_res.borders.block_start;

                let free_v = final_row_h
                    .saturating_sub(content_h)
                    .saturating_sub(block_sum(&cell_res.borders))
                    .saturating_sub(block_sum(&cell_res.paddings))
                    .max(LayoutUnit::ZERO);

                // Aplica offset vertical conforme vertical-align
                let v_offset = match cell_res.vertical_align {
                    VerticalAlign::Top | VerticalAlign::Baseline => LayoutUnit::ZERO,
                    VerticalAlign::Middle => free_v.saturating_div(2),
                    VerticalAlign::Bottom => free_v,
                };

                cell_res.padding_box = LogicalRect {
                    inline_start: cell_res.border_box.inline_start.saturating_add(cell_res.borders.inline_start),
                    block_start: curr_y.saturating_add(border_top),
                    inline_size: cell_res.border_box.inline_size.saturating_sub(inline_sum(&cell_res.borders)),
                    block_size: final_row_h.saturating_sub(block_sum(&cell_res.borders)),
                };

                cell_res.content_box = LogicalRect {
                    inline_start: cell_res.padding_box.inline_start.saturating_add(cell_res.paddings.inline_start),
                    block_start: cell_res.padding_box.block_start.saturating_add(padding_top).saturating_add(v_offset),
                    inline_size: cell_res.padding_box.inline_size.saturating_sub(inline_sum(&cell_res.paddings)),
                    block_size: content_h,
                };

                cell_res.margin_box = cell_res.border_box;

                finalized_cells.push(cell_res);
            }

            let row_w = table_avail_w;
            let row_border_box = LogicalRect::new(LayoutUnit::ZERO, curr_y, row_w, final_row_h);

            row_results.push(TableRowResult {
                box_id: row_node.id,
                row_index: r_idx,
                border_box: row_border_box,
                cells: finalized_cells,
            });

            curr_y = curr_y.saturating_add(final_row_h);
        }

        // 8. Monta a geometria da tabela completa
        let total_table_w = table_avail_w;
        let total_table_h = curr_y;

        let content_box = LogicalRect {
            inline_start: paddings.inline_start.saturating_add(borders.inline_start),
            block_start: paddings.block_start.saturating_add(borders.block_start),
            inline_size: total_table_w,
            block_size: total_table_h,
        };

        let padding_box = LogicalRect {
            inline_start: borders.inline_start,
            block_start: borders.block_start,
            inline_size: total_table_w.saturating_add(inline_sum(&paddings)),
            block_size: total_table_h.saturating_add(block_sum(&paddings)),
        };

        let border_box = LogicalRect {
            inline_start: LayoutUnit::ZERO,
            block_start: LayoutUnit::ZERO,
            inline_size: total_table_w.saturating_add(h_surrounding),
            block_size: total_table_h.saturating_add(v_surrounding),
        };

        let margin_box = LogicalRect {
            inline_start: LayoutUnit::ZERO.saturating_sub(margins.inline_start),
            block_start: LayoutUnit::ZERO.saturating_sub(margins.block_start),
            inline_size: border_box.inline_size.saturating_add(inline_sum(&margins)),
            block_size: border_box.block_size.saturating_add(block_sum(&margins)),
        };

        TableLayoutResult {
            box_id: node.id,
            margin_box,
            border_box,
            padding_box,
            content_box,
            margins,
            borders,
            paddings,
            columns_count: max_cols,
            rows_count: row_results.len(),
            column_constraints,
            column_widths,
            row_heights,
            rows: row_results,
        }
    }
}

/// Extrai restrições de dimensionamento de uma célula.
fn compute_cell_constraints(cell: &LayoutBox) -> CellConstraints {
    let bp_h = cell
        .style
        .padding_left()
        .saturating_add(cell.style.padding_right())
        .saturating_add(cell.style.border_width().left)
        .saturating_add(cell.style.border_width().right);
    let min_content = if let Some(w) = cell.style.min_width() {
        w
    } else {
        cell.compute_min_max_sizes().min_content.saturating_add(bp_h)
    };

    let max_content = if let Some(w) = cell.style.width() {
        w
    } else {
        cell.compute_min_max_sizes().max_content.saturating_add(bp_h)
    };

    CellConstraints::new(min_content, max_content, 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline::text_measurer::FixedTextMeasurer;
    use ace_style::computed::ComputedStyle;
    use std::sync::Arc;

    fn make_table(children: Vec<LayoutBox>) -> LayoutBox {
        let mut style = ComputedStyle::initial();
        let mut b = (*style.box_style).clone();
        b.display = Display::Table;
        b.width = Some(LayoutUnit::from_px(400));
        style.box_style = Arc::new(b);

        let mut table = LayoutBox::new(NodeId::new(), LayoutBoxKind::Table, Arc::new(style));
        for child in children {
            table.add_child(child);
        }
        table
    }

    fn make_row(cells: Vec<LayoutBox>) -> LayoutBox {
        let mut style = ComputedStyle::initial();
        let mut b = (*style.box_style).clone();
        b.display = Display::TableRow;
        style.box_style = Arc::new(b);

        let mut row = LayoutBox::new(NodeId::new(), LayoutBoxKind::TableRow, Arc::new(style));
        for cell in cells {
            row.add_child(cell);
        }
        row
    }

    fn make_cell(min_w: i32, max_w: i32) -> LayoutBox {
        let mut style = ComputedStyle::initial();
        let mut b = (*style.box_style).clone();
        b.display = Display::TableCell;
        b.min_width = Some(LayoutUnit::from_px(min_w));
        b.width = Some(LayoutUnit::from_px(max_w));
        style.box_style = Arc::new(b);

        LayoutBox::new(NodeId::new(), LayoutBoxKind::TableCell, Arc::new(style))
    }

    #[test]
    fn test_tfc_table_layout_proportional_distribution() {
        let cell1 = make_cell(50, 100);
        let cell2 = make_cell(50, 200);
        let row = make_row(vec![cell1, cell2]);
        let table = make_table(vec![row]);

        let tfc = TableFormattingContext::new();
        let space = ConstraintSpace::new(
            AvailableSize::Definite(LayoutUnit::from_px(400)),
            AvailableSize::Indefinite,
            LogicalSize::new(LayoutUnit::from_px(400), LayoutUnit::ZERO),
        );
        let measurer = FixedTextMeasurer::default();

        let res = tfc.layout(&table, &space, &measurer);
        assert_eq!(res.columns_count, 2);
        assert_eq!(res.rows_count, 1);
        // Table avail = 400px.
        // Sum max = 300px. Excess = 100px.
        // Col 0: 100 + 100 * (100/300) = 133px.
        // Col 1: 200 + 100 * (200/300) = 267px.
        let sum_w = res.column_widths[0] + res.column_widths[1];
        assert_eq!(sum_w, LayoutUnit::from_px(400));
        assert_eq!(res.rows[0].cells[0].border_box.inline_start, LayoutUnit::from_px(0));
        assert_eq!(res.rows[0].cells[1].border_box.inline_start, res.column_widths[0]);
    }
}
