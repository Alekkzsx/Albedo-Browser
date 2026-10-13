//! # Flexbox Formatting Context (`FlexFormattingContext` / FFC)
//!
//! Orquestrador de layout do Flexbox implementando W3C CSS Flexible Box Layout L1:
//! - Extração de parâmetros de container e itens a partir de `ComputedStyle`
//! - Resolução de eixos principal e transversal
//! - Integração com o Freeze Loop ($O(N)$) e algoritmo de quebra de linhas
//! - Formatação das subárvores dos itens flexíveis (novos BFCs independentes)

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{LogicalRect, LogicalSides, LogicalSize};
use ace_style::values::enums::{BoxSizing, Display};

use crate::block::bfc::{BfcLayoutResult, BlockFormattingContext};
use crate::flex::algorithm::{
    layout_flex_lines, map_to_logical_rect, split_into_flex_lines, FlexAxis,
    FlexContainerConfig, FlexItem,
};
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

/// Resultado da formatação de um item individual dentro do FFC.
#[derive(Debug, Clone, PartialEq)]
pub struct FlexItemResult {
    pub box_id: NodeId,
    pub margin_box: LogicalRect<LayoutUnit>,
    pub border_box: LogicalRect<LayoutUnit>,
    pub padding_box: LogicalRect<LayoutUnit>,
    pub content_box: LogicalRect<LayoutUnit>,
    pub margins: LogicalSides<LayoutUnit>,
    pub borders: LogicalSides<LayoutUnit>,
    pub paddings: LogicalSides<LayoutUnit>,
    pub line_index: usize,
    pub bfc_result: Option<BfcLayoutResult>,
}

impl FlexItemResult {
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

/// Resultado completo do layout de um contêiner flexível.
#[derive(Debug, Clone, PartialEq)]
pub struct FlexLayoutResult {
    pub box_id: NodeId,
    pub margin_box: LogicalRect<LayoutUnit>,
    pub border_box: LogicalRect<LayoutUnit>,
    pub padding_box: LogicalRect<LayoutUnit>,
    pub content_box: LogicalRect<LayoutUnit>,
    pub margins: LogicalSides<LayoutUnit>,
    pub borders: LogicalSides<LayoutUnit>,
    pub paddings: LogicalSides<LayoutUnit>,
    pub items: Vec<FlexItemResult>,
    pub lines_count: usize,
}

impl FlexLayoutResult {
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

/// Motor de execução do Flexbox Formatting Context (FFC).
#[derive(Debug, Clone, Default)]
pub struct FlexFormattingContext;

impl FlexFormattingContext {
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Executa o layout flexível completo para a caixa e seus itens descendentes.
    pub fn layout(
        &self,
        node: &LayoutBox,
        space: &ConstraintSpace,
        measurer: &dyn TextMeasurer,
    ) -> FlexLayoutResult {
        // 1. Resolução de paddings, borders e margins do contêiner
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

        // 2. Determinação das dimensões do contêiner
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
            // Indefinido: usa max-content
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

        // 3. Eixos e Gaps
        let axis = FlexAxis::from_direction(node.style.flex_direction());
        let (row_gap, col_gap) = (
            node.style.flex.row_gap.unwrap_or(LayoutUnit::ZERO),
            node.style.flex.column_gap.unwrap_or(LayoutUnit::ZERO),
        );

        let (main_gap, cross_gap) = if axis.is_row() {
            (col_gap, row_gap)
        } else {
            (row_gap, col_gap)
        };

        let (container_main_size, container_cross_size) = if axis.is_row() {
            (Some(container_w), container_h)
        } else {
            (container_h, Some(container_w))
        };

        // 4. Converte os filhos diretos em FlexItems
        let mut flex_items = Vec::new();
        for child in &node.children {
            if child.style.display() == Display::None {
                continue;
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

            let (margins_main, margins_cross) = if axis.is_row() {
                (
                    (item_margins.inline_start, item_margins.inline_end),
                    (item_margins.block_start, item_margins.block_end),
                )
            } else {
                (
                    (item_margins.block_start, item_margins.block_end),
                    (item_margins.inline_start, item_margins.inline_end),
                )
            };

            let (bp_main, bp_cross) = if axis.is_row() {
                (
                    inline_sum(&item_borders).saturating_add(inline_sum(&item_paddings)),
                    block_sum(&item_borders).saturating_add(block_sum(&item_paddings)),
                )
            } else {
                (
                    block_sum(&item_borders).saturating_add(block_sum(&item_paddings)),
                    inline_sum(&item_borders).saturating_add(inline_sum(&item_paddings)),
                )
            };

            // Determina flex-basis
            let flex_basis = if let Some(basis) = child.style.flex.flex_basis {
                basis
            } else if axis.is_row() {
                if let Some(w) = child.style.width() {
                    match child.style.box_sizing() {
                        BoxSizing::BorderBox => w,
                        BoxSizing::ContentBox => w.saturating_add(bp_main),
                    }
                } else {
                    child.compute_min_max_sizes().max_content.saturating_add(bp_main)
                }
            } else if let Some(h) = child.style.height() {
                match child.style.box_sizing() {
                    BoxSizing::BorderBox => h,
                    BoxSizing::ContentBox => h.saturating_add(bp_main),
                }
            } else {
                child.compute_min_max_sizes().max_content.saturating_add(bp_main)
            };

            let min_main = if axis.is_row() {
                child.style.min_width().unwrap_or(LayoutUnit::ZERO)
            } else {
                child.style.min_height().unwrap_or(LayoutUnit::ZERO)
            };

            let max_main = if axis.is_row() {
                child.style.max_width()
            } else {
                child.style.max_height()
            };

            let hypothetical_cross = if axis.is_row() {
                if let Some(h) = child.style.height() {
                    match child.style.box_sizing() {
                        BoxSizing::BorderBox => h,
                        BoxSizing::ContentBox => h.saturating_add(bp_cross),
                    }
                } else {
                    LayoutUnit::from_px(20).saturating_add(bp_cross)
                }
            } else if let Some(w) = child.style.width() {
                match child.style.box_sizing() {
                    BoxSizing::BorderBox => w,
                    BoxSizing::ContentBox => w.saturating_add(bp_cross),
                }
            } else {
                child.compute_min_max_sizes().max_content.saturating_add(bp_cross)
            };

            flex_items.push(FlexItem {
                node_id: child.id,
                flex_grow: child.style.flex.flex_grow,
                flex_shrink: child.style.flex.flex_shrink,
                flex_basis,
                min_main_size: min_main,
                max_main_size: max_main,
                hypothetical_main_size: flex_basis,
                target_main_size: flex_basis,
                hypothetical_cross_size: hypothetical_cross,
                target_cross_size: hypothetical_cross,
                margins_main,
                margins_cross,
                borders_padding_main: bp_main,
                borders_padding_cross: bp_cross,
                align_self: child.style.flex.align_self,
                main_offset: LayoutUnit::ZERO,
                cross_offset: LayoutUnit::ZERO,
                margins: item_margins,
                borders: item_borders,
                paddings: item_paddings,
            });
        }

        let config = FlexContainerConfig {
            axis,
            wrap: node.style.flex_wrap(),
            justify_content: node.style.justify_content(),
            align_items: node.style.align_items(),
            align_content: node.style.align_content(),
            main_gap,
            cross_gap,
            container_main_size,
            container_cross_size,
        };

        // 5. Divide em linhas e resolve comprimentos e alinhamentos
        let lines = split_into_flex_lines(flex_items, config.wrap, container_main_size, main_gap);
        let (resolved_lines, used_main, used_cross) = layout_flex_lines(lines, &config);

        let lines_count = resolved_lines.len();

        // 6. Formata itens e mapeia retângulos geométricos
        let mut item_results = Vec::new();
        let bfc_engine = BlockFormattingContext::new();

        let mut child_map: rustc_hash::FxHashMap<NodeId, &LayoutBox> =
            rustc_hash::FxHashMap::default();
        for child in &node.children {
            child_map.insert(child.id, child);
        }

        for (line_idx, line) in resolved_lines.iter().enumerate() {
            for item in &line.items {
                let border_box = map_to_logical_rect(
                    axis,
                    item.main_offset,
                    item.target_main_size,
                    item.cross_offset,
                    item.target_cross_size,
                    line.cross_offset,
                );

                let padding_box = LogicalRect {
                    inline_start: border_box.inline_start.saturating_add(item.borders.inline_start),
                    block_start: border_box.block_start.saturating_add(item.borders.block_start),
                    inline_size: border_box
                        .inline_size
                        .saturating_sub(inline_sum(&item.borders))
                        .max(LayoutUnit::ZERO),
                    block_size: border_box
                        .block_size
                        .saturating_sub(block_sum(&item.borders))
                        .max(LayoutUnit::ZERO),
                };

                let content_box = LogicalRect {
                    inline_start: padding_box.inline_start.saturating_add(item.paddings.inline_start),
                    block_start: padding_box.block_start.saturating_add(item.paddings.block_start),
                    inline_size: padding_box
                        .inline_size
                        .saturating_sub(inline_sum(&item.paddings))
                        .max(LayoutUnit::ZERO),
                    block_size: padding_box
                        .block_size
                        .saturating_sub(block_sum(&item.paddings))
                        .max(LayoutUnit::ZERO),
                };

                let margin_box = LogicalRect {
                    inline_start: border_box.inline_start.saturating_sub(item.margins.inline_start),
                    block_start: border_box.block_start.saturating_sub(item.margins.block_start),
                    inline_size: border_box.inline_size.saturating_add(inline_sum(&item.margins)),
                    block_size: border_box.block_size.saturating_add(block_sum(&item.margins)),
                };

                // Formata os filhos do item se for nó de bloco participante
                let bfc_result = if let Some(&child_box) = child_map.get(&item.node_id) {
                    if !child_box.children.is_empty() {
                        let child_space = ConstraintSpace::new(
                            AvailableSize::Definite(content_box.inline_size),
                            AvailableSize::Definite(content_box.block_size),
                            LogicalSize::new(content_box.inline_size, content_box.block_size),
                        );
                        Some(bfc_engine.layout(child_box, &child_space, measurer))
                    } else {
                        None
                    }
                } else {
                    None
                };

                item_results.push(FlexItemResult {
                    box_id: item.node_id,
                    margin_box,
                    border_box,
                    padding_box,
                    content_box,
                    margins: item.margins,
                    borders: item.borders,
                    paddings: item.paddings,
                    line_index: line_idx,
                    bfc_result,
                });
            }
        }

        // 7. Monta caixas finais do contêiner flexível
        let (final_content_w, final_content_h) = if axis.is_row() {
            (used_main, used_cross)
        } else {
            (used_cross, used_main)
        };

        let content_box = LogicalRect {
            inline_start: paddings.inline_start.saturating_add(borders.inline_start),
            block_start: paddings.block_start.saturating_add(borders.block_start),
            inline_size: final_content_w,
            block_size: final_content_h,
        };

        let padding_box = LogicalRect {
            inline_start: borders.inline_start,
            block_start: borders.block_start,
            inline_size: final_content_w.saturating_add(inline_sum(&paddings)),
            block_size: final_content_h.saturating_add(block_sum(&paddings)),
        };

        let border_box = LogicalRect {
            inline_start: LayoutUnit::ZERO,
            block_start: LayoutUnit::ZERO,
            inline_size: final_content_w.saturating_add(h_surrounding),
            block_size: final_content_h.saturating_add(v_surrounding),
        };

        let margin_box = LogicalRect {
            inline_start: LayoutUnit::ZERO.saturating_sub(margins.inline_start),
            block_start: LayoutUnit::ZERO.saturating_sub(margins.block_start),
            inline_size: border_box.inline_size.saturating_add(inline_sum(&margins)),
            block_size: border_box.block_size.saturating_add(block_sum(&margins)),
        };

        FlexLayoutResult {
            box_id: node.id,
            margin_box,
            border_box,
            padding_box,
            content_box,
            margins,
            borders,
            paddings,
            items: item_results,
            lines_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline::text_measurer::FixedTextMeasurer;
    use crate::tree::layout_box::LayoutBoxKind;
    use ace_style::computed::ComputedStyle;
    use ace_style::values::enums::{AlignItems, FlexDirection, FlexWrap, JustifyContent};
    use std::sync::Arc;

    fn make_flex_container(
        width: i32,
        height: i32,
        direction: FlexDirection,
        wrap: FlexWrap,
        justify: JustifyContent,
        align_items: AlignItems,
        children: Vec<LayoutBox>,
    ) -> LayoutBox {
        let mut style = ComputedStyle::initial();
        let mut b = (*style.box_style).clone();
        b.display = Display::Flex;
        b.width = Some(LayoutUnit::from_px(width));
        b.height = Some(LayoutUnit::from_px(height));
        style.box_style = Arc::new(b);

        let mut f = (*style.flex).clone();
        f.flex_direction = direction;
        f.flex_wrap = wrap;
        f.justify_content = justify;
        f.align_items = align_items;
        style.flex = Arc::new(f);

        let mut container = LayoutBox::new(NodeId::new(), LayoutBoxKind::Flex, Arc::new(style));
        for child in children {
            container.add_child(child);
        }
        container
    }

    fn make_flex_child(grow: f32, shrink: f32, width_px: Option<i32>, height_px: Option<i32>) -> LayoutBox {
        let mut style = ComputedStyle::initial();
        let mut b = (*style.box_style).clone();
        b.display = Display::Block;
        b.width = width_px.map(LayoutUnit::from_px);
        b.height = height_px.map(LayoutUnit::from_px);
        style.box_style = Arc::new(b);

        let mut f = (*style.flex).clone();
        f.flex_grow = grow;
        f.flex_shrink = shrink;
        style.flex = Arc::new(f);

        LayoutBox::new(NodeId::new(), LayoutBoxKind::Block, Arc::new(style))
    }

    #[test]
    fn test_ffc_row_flex_grow_distribution() {
        let child1 = make_flex_child(1.0, 1.0, Some(100), Some(40));
        let child2 = make_flex_child(2.0, 1.0, Some(100), Some(40));

        let container = make_flex_container(
            500,
            100,
            FlexDirection::Row,
            FlexWrap::NoWrap,
            JustifyContent::FlexStart,
            AlignItems::Stretch,
            vec![child1, child2],
        );

        let ffc = FlexFormattingContext::new();
        let space = ConstraintSpace::new(
            AvailableSize::Definite(LayoutUnit::from_px(500)),
            AvailableSize::Definite(LayoutUnit::from_px(100)),
            LogicalSize::new(LayoutUnit::from_px(500), LayoutUnit::from_px(100)),
        );
        let measurer = FixedTextMeasurer::default();

        let res = ffc.layout(&container, &space, &measurer);
        assert_eq!(res.items.len(), 2);
        // Free space = 300px. Grow ratio 1:2 -> +100px and +200px.
        assert_eq!(res.items[0].width(), LayoutUnit::from_px(200));
        assert_eq!(res.items[1].width(), LayoutUnit::from_px(300));
        // Item 0 at x=0, Item 1 at x=200
        assert_eq!(res.items[0].border_box.inline_start, LayoutUnit::from_px(0));
        assert_eq!(res.items[1].border_box.inline_start, LayoutUnit::from_px(200));
    }

    #[test]
    fn test_ffc_column_direction() {
        let child1 = make_flex_child(1.0, 1.0, Some(100), Some(50));
        let child2 = make_flex_child(1.0, 1.0, Some(100), Some(50));

        let container = make_flex_container(
            200,
            300,
            FlexDirection::Column,
            FlexWrap::NoWrap,
            JustifyContent::FlexStart,
            AlignItems::Stretch,
            vec![child1, child2],
        );

        let ffc = FlexFormattingContext::new();
        let space = ConstraintSpace::new(
            AvailableSize::Definite(LayoutUnit::from_px(200)),
            AvailableSize::Definite(LayoutUnit::from_px(300)),
            LogicalSize::new(LayoutUnit::from_px(200), LayoutUnit::from_px(300)),
        );
        let measurer = FixedTextMeasurer::default();

        let res = ffc.layout(&container, &space, &measurer);
        assert_eq!(res.items.len(), 2);
        // In column direction: main axis is block/height.
        // Available height = 300px. Initial bases = 50 + 50 = 100px. Free = 200px.
        // Equal grow 1:1 -> +100px each -> 150px each.
        assert_eq!(res.items[0].height(), LayoutUnit::from_px(150));
        assert_eq!(res.items[1].height(), LayoutUnit::from_px(150));
        assert_eq!(res.items[0].border_box.block_start, LayoutUnit::from_px(0));
        assert_eq!(res.items[1].border_box.block_start, LayoutUnit::from_px(150));
    }
}
