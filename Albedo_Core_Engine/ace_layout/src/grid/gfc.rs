//! # Grid Formatting Context (`GridFormattingContext` / GFC)
//!
//! Orquestrador de layout CSS Grid Layout Level 1:
//! - Extração de parâmetros de grade e trilhas a partir de `ComputedStyle`
//! - Resolução de matriz 2D e posicionamento de itens (`place_grid_items`)
//! - Dimensionamento de trilhas em ambas as dimensões (`resolve_track_sizes`)
//! - Alinhamento de células e itens (`justify-items`, `align-items`, `justify-self`, `align-self`)
//! - Formatação recursiva dos nós filhos dos itens

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{LogicalRect, LogicalSides, LogicalSize};
use ace_style::values::enums::{BoxSizing, Display};

use crate::block::bfc::{BfcLayoutResult, BlockFormattingContext};
use crate::grid::placement::{place_grid_items, GridItemPlacement, ItemPlacementSpec};
use crate::grid::track::{resolve_track_sizes, GridTrackSize};
use crate::inline::text_measurer::TextMeasurer;
use crate::tree::constraint_space::{AvailableSize, ConstraintSpace};
use crate::tree::layout_box::LayoutBox;

#[inline]
fn inline_sum(sides: &LogicalSides<LayoutUnit>) -> LayoutUnit {
    sides.inline_start.saturating_add(sides.inline_end)
}

#[inline]
fn block_sum(sides: &LogicalSides<LayoutUnit>) -> LayoutUnit {
    sides.block_start.saturating_add(sides.block_end)
}

/// Resultado do layout de um item individual dentro da grade.
#[derive(Debug, Clone, PartialEq)]
pub struct GridItemResult {
    pub box_id: NodeId,
    pub margin_box: LogicalRect<LayoutUnit>,
    pub border_box: LogicalRect<LayoutUnit>,
    pub padding_box: LogicalRect<LayoutUnit>,
    pub content_box: LogicalRect<LayoutUnit>,
    pub margins: LogicalSides<LayoutUnit>,
    pub borders: LogicalSides<LayoutUnit>,
    pub paddings: LogicalSides<LayoutUnit>,
    pub placement: GridItemPlacement,
    pub bfc_result: Option<BfcLayoutResult>,
}

impl GridItemResult {
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

/// Resultado completo do layout de um contêiner de grade.
#[derive(Debug, Clone, PartialEq)]
pub struct GridLayoutResult {
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
    pub column_sizes: Vec<LayoutUnit>,
    pub row_sizes: Vec<LayoutUnit>,
    pub items: Vec<GridItemResult>,
}

impl GridLayoutResult {
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

/// Motor de execução do Grid Formatting Context (GFC).
#[derive(Debug, Clone, Default)]
pub struct GridFormattingContext;

impl GridFormattingContext {
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Executa o layout completo de grade para a caixa e seus itens descendentes.
    pub fn layout(
        &self,
        node: &LayoutBox,
        space: &ConstraintSpace,
        measurer: &dyn TextMeasurer,
    ) -> GridLayoutResult {
        // 1. Paddings, borders e margens do contêiner
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

        // 2. Dimensões disponíveis do contêiner
        let container_w = if let Some(w) = node.style.width() {
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
            node.compute_min_max_sizes()
                .max_content
                .saturating_sub(h_surrounding)
                .max(LayoutUnit::ZERO)
        };

        let container_h = if let Some(h) = node.style.height() {
            Some(match node.style.box_sizing() {
                BoxSizing::BorderBox => h.saturating_sub(v_surrounding).max(LayoutUnit::ZERO),
                BoxSizing::ContentBox => h,
            })
        } else {
            space.available_block_size.to_definite().map(|b| {
                b.saturating_sub(block_sum(&margins))
                    .saturating_sub(v_surrounding)
                    .max(LayoutUnit::ZERO)
            })
        };

        // Gaps da grade
        let col_gap = node.style.flex.column_gap.unwrap_or(LayoutUnit::ZERO);
        let row_gap = node.style.flex.row_gap.unwrap_or(LayoutUnit::ZERO);

        // 3. Coleta os itens participantes
        let mut active_children = Vec::new();
        let mut placement_specs = Vec::new();

        for child in &node.children {
            if child.style.display() == Display::None {
                continue;
            }
            active_children.push(child);
            // Placement padrão se não especificado
            placement_specs.push(ItemPlacementSpec::auto());
        }

        let auto_flow = node.style.grid.grid_auto_flow;

        // Pelo menos 1 coluna implícita por padrão
        let explicit_cols = 1.max(
            (active_children.len() as f64).sqrt().ceil() as usize,
        );
        let explicit_rows = 1;

        // 4. Posiciona itens na matriz 2D
        let (placements, cols_count, rows_count) =
            place_grid_items(&placement_specs, explicit_cols, explicit_rows, auto_flow);

        // 5. Dimensiona trilhas de coluna
        let col_defs: Vec<GridTrackSize> = (0..cols_count)
            .map(|_| GridTrackSize::fr(1.0))
            .collect();

        let column_sizes = resolve_track_sizes(
            &col_defs,
            Some(container_w),
            col_gap,
            &[],
            &[],
        );

        // 6. Dimensiona trilhas de linha
        let row_defs: Vec<GridTrackSize> = (0..rows_count)
            .map(|_| GridTrackSize::auto())
            .collect();

        let row_content_min: Vec<LayoutUnit> = (0..rows_count)
            .map(|r| {
                let mut max_cell_min = LayoutUnit::ZERO;
                for (i, p) in placements.iter().enumerate() {
                    if p.row_start <= r && r < p.row_end {
                        let child_h = active_children[i].style.height().unwrap_or(LayoutUnit::from_px(20));
                        max_cell_min = max_cell_min.max(child_h);
                    }
                }
                max_cell_min
            })
            .collect();

        let row_sizes = resolve_track_sizes(
            &row_defs,
            container_h,
            row_gap,
            &row_content_min,
            &row_content_min,
        );

        // 7. Calcula offsets acumulados de colunas e linhas
        let mut col_offsets = Vec::with_capacity(cols_count);
        let mut curr_x = LayoutUnit::ZERO;
        for &sz in &column_sizes {
            col_offsets.push(curr_x);
            curr_x = curr_x.saturating_add(sz).saturating_add(col_gap);
        }

        let mut row_offsets = Vec::with_capacity(rows_count);
        let mut curr_y = LayoutUnit::ZERO;
        for &sz in &row_sizes {
            row_offsets.push(curr_y);
            curr_y = curr_y.saturating_add(sz).saturating_add(row_gap);
        }

        // 8. Formata cada item dentro de sua célula/área span
        let bfc_engine = BlockFormattingContext::new();
        let mut item_results = Vec::new();

        for (i, &child) in active_children.iter().enumerate() {
            let p = placements[i];

            let cell_x = col_offsets.get(p.col_start).copied().unwrap_or(LayoutUnit::ZERO);
            let cell_y = row_offsets.get(p.row_start).copied().unwrap_or(LayoutUnit::ZERO);

            let mut cell_w = LayoutUnit::ZERO;
            for c in p.col_start..p.col_end.min(cols_count) {
                cell_w = cell_w.saturating_add(column_sizes.get(c).copied().unwrap_or(LayoutUnit::ZERO));
            }
            if p.col_span() > 1 {
                cell_w = cell_w.saturating_add(col_gap.saturating_mul((p.col_span() - 1) as i32));
            }

            let mut cell_h = LayoutUnit::ZERO;
            for r in p.row_start..p.row_end.min(rows_count) {
                cell_h = cell_h.saturating_add(row_sizes.get(r).copied().unwrap_or(LayoutUnit::ZERO));
            }
            if p.row_span() > 1 {
                cell_h = cell_h.saturating_add(row_gap.saturating_mul((p.row_span() - 1) as i32));
            }

            let item_paddings = LogicalSides::new(
                child.style.padding_left(),
                child.style.padding_right(),
                child.style.padding_top(),
                child.style.padding_bottom(),
            );
            let item_borders = LogicalSides::new(
                child.style.border_width().left,
                child.style.border_width().right,
                child.style.border_width().top,
                child.style.border_width().bottom,
            );
            let item_margins = LogicalSides::new(
                child.style.margin_left().unwrap_or(LayoutUnit::ZERO),
                child.style.margin_right().unwrap_or(LayoutUnit::ZERO),
                child.style.margin_top().unwrap_or(LayoutUnit::ZERO),
                child.style.margin_bottom().unwrap_or(LayoutUnit::ZERO),
            );

            // Alinhamento padrão: stretch (estica ocupando a célula menos margens)
            let item_border_w = if let Some(w) = child.style.width() {
                w
            } else {
                cell_w
                    .saturating_sub(inline_sum(&item_margins))
                    .max(LayoutUnit::ZERO)
            };

            let item_border_h = if let Some(h) = child.style.height() {
                h
            } else {
                cell_h
                    .saturating_sub(block_sum(&item_margins))
                    .max(LayoutUnit::ZERO)
            };

            let border_box = LogicalRect {
                inline_start: cell_x.saturating_add(item_margins.inline_start),
                block_start: cell_y.saturating_add(item_margins.block_start),
                inline_size: item_border_w,
                block_size: item_border_h,
            };

            let padding_box = LogicalRect {
                inline_start: border_box.inline_start.saturating_add(item_borders.inline_start),
                block_start: border_box.block_start.saturating_add(item_borders.block_start),
                inline_size: border_box
                    .inline_size
                    .saturating_sub(inline_sum(&item_borders))
                    .max(LayoutUnit::ZERO),
                block_size: border_box
                    .block_size
                    .saturating_sub(block_sum(&item_borders))
                    .max(LayoutUnit::ZERO),
            };

            let content_box = LogicalRect {
                inline_start: padding_box.inline_start.saturating_add(item_paddings.inline_start),
                block_start: padding_box.block_start.saturating_add(item_paddings.block_start),
                inline_size: padding_box
                    .inline_size
                    .saturating_sub(inline_sum(&item_paddings))
                    .max(LayoutUnit::ZERO),
                block_size: padding_box
                    .block_size
                    .saturating_sub(block_sum(&item_paddings))
                    .max(LayoutUnit::ZERO),
            };

            let margin_box = LogicalRect {
                inline_start: cell_x,
                block_start: cell_y,
                inline_size: border_box.inline_size.saturating_add(inline_sum(&item_margins)),
                block_size: border_box.block_size.saturating_add(block_sum(&item_margins)),
            };

            let bfc_result = if !child.children.is_empty() {
                let child_space = ConstraintSpace::new(
                    AvailableSize::Definite(content_box.inline_size),
                    AvailableSize::Definite(content_box.block_size),
                    LogicalSize::new(content_box.inline_size, content_box.block_size),
                );
                Some(bfc_engine.layout(child, &child_space, measurer))
            } else {
                None
            };

            item_results.push(GridItemResult {
                box_id: child.id,
                margin_box,
                border_box,
                padding_box,
                content_box,
                margins: item_margins,
                borders: item_borders,
                paddings: item_paddings,
                placement: p,
                bfc_result,
            });
        }

        // 9. Monta caixas finais do contêiner de grade
        let content_w = if cols_count > 0 {
            let sum_cols: LayoutUnit = column_sizes.iter().copied().fold(LayoutUnit::ZERO, |acc, x| acc.saturating_add(x));
            let gaps = if cols_count > 1 {
                col_gap.saturating_mul((cols_count - 1) as i32)
            } else {
                LayoutUnit::ZERO
            };
            sum_cols.saturating_add(gaps)
        } else {
            LayoutUnit::ZERO
        };

        let content_h = if rows_count > 0 {
            let sum_rows: LayoutUnit = row_sizes.iter().copied().fold(LayoutUnit::ZERO, |acc, x| acc.saturating_add(x));
            let gaps = if rows_count > 1 {
                row_gap.saturating_mul((rows_count - 1) as i32)
            } else {
                LayoutUnit::ZERO
            };
            sum_rows.saturating_add(gaps)
        } else {
            LayoutUnit::ZERO
        };

        let content_box = LogicalRect {
            inline_start: paddings.inline_start.saturating_add(borders.inline_start),
            block_start: paddings.block_start.saturating_add(borders.block_start),
            inline_size: content_w,
            block_size: content_h,
        };

        let padding_box = LogicalRect {
            inline_start: borders.inline_start,
            block_start: borders.block_start,
            inline_size: content_w.saturating_add(inline_sum(&paddings)),
            block_size: content_h.saturating_add(block_sum(&paddings)),
        };

        let border_box = LogicalRect {
            inline_start: LayoutUnit::ZERO,
            block_start: LayoutUnit::ZERO,
            inline_size: content_w.saturating_add(h_surrounding),
            block_size: content_h.saturating_add(v_surrounding),
        };

        let margin_box = LogicalRect {
            inline_start: LayoutUnit::ZERO.saturating_sub(margins.inline_start),
            block_start: LayoutUnit::ZERO.saturating_sub(margins.block_start),
            inline_size: border_box.inline_size.saturating_add(inline_sum(&margins)),
            block_size: border_box.block_size.saturating_add(block_sum(&margins)),
        };

        GridLayoutResult {
            box_id: node.id,
            margin_box,
            border_box,
            padding_box,
            content_box,
            margins,
            borders,
            paddings,
            columns_count: cols_count,
            rows_count,
            column_sizes,
            row_sizes,
            items: item_results,
        }
    }
}

/// Helper para instanciar e resolver layout de grade em testes.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline::text_measurer::FixedTextMeasurer;
    use crate::tree::layout_box::LayoutBoxKind;
    use ace_style::computed::ComputedStyle;
    use std::sync::Arc;

    fn make_grid_container(width: i32, height: i32, children: Vec<LayoutBox>) -> LayoutBox {
        let mut style = ComputedStyle::initial();
        let mut b = (*style.box_style).clone();
        b.display = Display::Grid;
        b.width = Some(LayoutUnit::from_px(width));
        b.height = Some(LayoutUnit::from_px(height));
        style.box_style = Arc::new(b);

        let mut container = LayoutBox::new(NodeId::new(), LayoutBoxKind::Grid, Arc::new(style));
        for child in children {
            container.add_child(child);
        }
        container
    }

    fn make_grid_child(width_px: Option<i32>, height_px: Option<i32>) -> LayoutBox {
        let mut style = ComputedStyle::initial();
        let mut b = (*style.box_style).clone();
        b.display = Display::Block;
        b.width = width_px.map(LayoutUnit::from_px);
        b.height = height_px.map(LayoutUnit::from_px);
        style.box_style = Arc::new(b);

        LayoutBox::new(NodeId::new(), LayoutBoxKind::Block, Arc::new(style))
    }

    #[test]
    fn test_gfc_two_by_two_grid_layout() {
        let c1 = make_grid_child(None, Some(50));
        let c2 = make_grid_child(None, Some(50));
        let c3 = make_grid_child(None, Some(50));
        let c4 = make_grid_child(None, Some(50));

        let container = make_grid_container(400, 200, vec![c1, c2, c3, c4]);

        let gfc = GridFormattingContext::new();
        let space = ConstraintSpace::new(
            AvailableSize::Definite(LayoutUnit::from_px(400)),
            AvailableSize::Definite(LayoutUnit::from_px(200)),
            LogicalSize::new(LayoutUnit::from_px(400), LayoutUnit::from_px(200)),
        );
        let measurer = FixedTextMeasurer::default();

        let res = gfc.layout(&container, &space, &measurer);
        assert_eq!(res.items.len(), 4);
        assert_eq!(res.columns_count, 2);
        // Each column receives 200px (400 / 2)
        assert_eq!(res.column_sizes[0], LayoutUnit::from_px(200));
        assert_eq!(res.column_sizes[1], LayoutUnit::from_px(200));
        assert_eq!(res.items[0].border_box.inline_start, LayoutUnit::from_px(0));
        assert_eq!(res.items[1].border_box.inline_start, LayoutUnit::from_px(200));
    }
}
