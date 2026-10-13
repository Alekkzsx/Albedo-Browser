//! # Algoritmo de Decomposição e Alinhamento Flexbox L1 (§5, §8, §9)
//!
//! Implementa os passos do algoritmo de layout flexível:
//! - Decomposição em eixos principal (main) e transversal (cross)
//! - Determinação do tamanho base hipotético (`flex-basis`, `min`, `max`)
//! - Quebra de linhas flexíveis (`flex-wrap: nowrap | wrap | wrap-reverse`)
//! - Alinhamento no eixo principal (`justify-content`) com calhas (`gap`)
//! - Alinhamento no eixo transversal (`align-items`, `align-self`, `align-content`)

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{LogicalRect, LogicalSides};
use ace_style::values::enums::{
    AlignContent, AlignItems, AlignSelf, FlexDirection, FlexWrap, JustifyContent,
};

use crate::flex::freeze_loop::{resolve_flexible_lengths, FlexFreezeItem};

/// Eixo de orientação flexível.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexAxis {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl FlexAxis {
    pub fn from_direction(dir: FlexDirection) -> Self {
        match dir {
            FlexDirection::Row => Self::Row,
            FlexDirection::RowReverse => Self::RowReverse,
            FlexDirection::Column => Self::Column,
            FlexDirection::ColumnReverse => Self::ColumnReverse,
        }
    }

    #[inline]
    pub fn is_row(self) -> bool {
        matches!(self, Self::Row | Self::RowReverse)
    }

    #[inline]
    pub fn is_column(self) -> bool {
        matches!(self, Self::Column | Self::ColumnReverse)
    }

    #[inline]
    pub fn is_reversed(self) -> bool {
        matches!(self, Self::RowReverse | Self::ColumnReverse)
    }
}

/// Dados internos de um item flexível durante o cálculo geométrico.
#[derive(Debug, Clone, PartialEq)]
pub struct FlexItem {
    pub node_id: NodeId,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: LayoutUnit,
    pub min_main_size: LayoutUnit,
    pub max_main_size: Option<LayoutUnit>,
    pub hypothetical_main_size: LayoutUnit,
    pub target_main_size: LayoutUnit,
    pub hypothetical_cross_size: LayoutUnit,
    pub target_cross_size: LayoutUnit,
    /// Margens no eixo principal: (início, fim).
    pub margins_main: (LayoutUnit, LayoutUnit),
    /// Margens no eixo transversal: (início, fim).
    pub margins_cross: (LayoutUnit, LayoutUnit),
    /// Borda + padding no eixo principal.
    pub borders_padding_main: LayoutUnit,
    /// Borda + padding no eixo transversal.
    pub borders_padding_cross: LayoutUnit,
    /// Alinhamento individual no eixo transversal.
    pub align_self: AlignSelf,
    /// Posição computada no eixo principal relativa à linha.
    pub main_offset: LayoutUnit,
    /// Posição computada no eixo transversal relativa à linha.
    pub cross_offset: LayoutUnit,
    /// Margens lógicas originais.
    pub margins: LogicalSides<LayoutUnit>,
    /// Bordas lógicas originais.
    pub borders: LogicalSides<LayoutUnit>,
    /// Paddings lógicos originais.
    pub paddings: LogicalSides<LayoutUnit>,
}

impl FlexItem {
    /// Tamanho externo total no eixo principal (margens + target).
    #[inline]
    pub fn outer_main_size(&self) -> LayoutUnit {
        self.margins_main
            .0
            .saturating_add(self.target_main_size)
            .saturating_add(self.margins_main.1)
    }

    /// Tamanho externo hipotético no eixo principal.
    #[inline]
    pub fn hypothetical_outer_main_size(&self) -> LayoutUnit {
        self.margins_main
            .0
            .saturating_add(self.hypothetical_main_size)
            .saturating_add(self.margins_main.1)
    }

    /// Tamanho externo total no eixo transversal (margens + target).
    #[inline]
    pub fn outer_cross_size(&self) -> LayoutUnit {
        self.margins_cross
            .0
            .saturating_add(self.target_cross_size)
            .saturating_add(self.margins_cross.1)
    }
}

/// Representa uma linha flexível contendo uma sequência contígua de itens.
#[derive(Debug, Clone, PartialEq)]
pub struct FlexLine {
    pub items: Vec<FlexItem>,
    pub line_main_size: LayoutUnit,
    pub line_cross_size: LayoutUnit,
    pub cross_offset: LayoutUnit,
}

impl FlexLine {
    pub fn new(items: Vec<FlexItem>) -> Self {
        Self {
            items,
            line_main_size: LayoutUnit::ZERO,
            line_cross_size: LayoutUnit::ZERO,
            cross_offset: LayoutUnit::ZERO,
        }
    }
}

/// Configurações do contêiner Flexbox para execução do algoritmo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlexContainerConfig {
    pub axis: FlexAxis,
    pub wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_content: AlignContent,
    pub main_gap: LayoutUnit,
    pub cross_gap: LayoutUnit,
    pub container_main_size: Option<LayoutUnit>,
    pub container_cross_size: Option<LayoutUnit>,
}

/// Quebra os itens em linhas flexíveis de acordo com `flex-wrap`.
pub fn split_into_flex_lines(
    items: Vec<FlexItem>,
    wrap: FlexWrap,
    container_main_size: Option<LayoutUnit>,
    main_gap: LayoutUnit,
) -> Vec<FlexLine> {
    if items.is_empty() {
        return Vec::new();
    }

    if wrap == FlexWrap::NoWrap || container_main_size.is_none() {
        return vec![FlexLine::new(items)];
    }

    let max_main = container_main_size.unwrap_or(LayoutUnit::MAX);
    let mut lines = Vec::new();
    let mut current_line_items = Vec::new();
    let mut accumulated_main = LayoutUnit::ZERO;

    for item in items {
        let item_outer = item.hypothetical_outer_main_size();
        let gap_needed = if current_line_items.is_empty() {
            LayoutUnit::ZERO
        } else {
            main_gap
        };

        let projected_main = accumulated_main
            .saturating_add(gap_needed)
            .saturating_add(item_outer);

        if !current_line_items.is_empty() && projected_main > max_main {
            // Fecha a linha atual e inicia uma nova
            lines.push(FlexLine::new(current_line_items));
            current_line_items = vec![item];
            accumulated_main = item_outer;
        } else {
            accumulated_main = projected_main;
            current_line_items.push(item);
        }
    }

    if !current_line_items.is_empty() {
        lines.push(FlexLine::new(current_line_items));
    }

    lines
}

/// Executa o layout flexível completo em todas as linhas.
pub fn layout_flex_lines(
    mut lines: Vec<FlexLine>,
    config: &FlexContainerConfig,
) -> (Vec<FlexLine>, LayoutUnit, LayoutUnit) {
    if lines.is_empty() {
        let c_main = config.container_main_size.unwrap_or(LayoutUnit::ZERO);
        let c_cross = config.container_cross_size.unwrap_or(LayoutUnit::ZERO);
        return (lines, c_main, c_cross);
    }

    // 1. Resolve comprimentos flexíveis no eixo principal para cada linha
    for line in lines.iter_mut() {
        let line_len = line.items.len();
        let total_gap = if line_len > 1 {
            config.main_gap.saturating_mul((line_len - 1) as i32)
        } else {
            LayoutUnit::ZERO
        };

        // Tamanho principal de referência da linha
        let target_line_main = if let Some(c_main) = config.container_main_size {
            c_main
        } else {
            // Se indefinido, a linha ajusta ao tamanho hipotético dos itens + gaps
            let sum_hyp: LayoutUnit = line.items.iter().fold(LayoutUnit::ZERO, |acc, it| {
                acc.saturating_add(it.flex_basis)
            });
            sum_hyp.saturating_add(total_gap)
        };

        line.line_main_size = target_line_main;

        // Monta lista de FlexFreezeItem
        let mut freeze_items: Vec<FlexFreezeItem> = line
            .items
            .iter()
            .map(|it| {
                FlexFreezeItem::new(
                    it.flex_grow,
                    it.flex_shrink,
                    it.flex_basis,
                    it.min_main_size,
                    it.max_main_size,
                )
            })
            .collect();

        let _ = resolve_flexible_lengths(&mut freeze_items, target_line_main, total_gap);

        // Atualiza os tamanhos alvo dos itens
        for (i, freeze_item) in freeze_items.into_iter().enumerate() {
            line.items[i].target_main_size = freeze_item.target_size;
        }

        // 2. Alinha os itens no eixo principal da linha (`justify-content`)
        align_items_main_axis(
            line,
            config.justify_content,
            config.main_gap,
            config.axis.is_reversed(),
        );

        // 3. Determina a dimensão transversal inicial da linha
        let max_cross: LayoutUnit = line
            .items
            .iter()
            .fold(LayoutUnit::ZERO, |acc, it| acc.max(it.outer_cross_size()));
        line.line_cross_size = max_cross;
    }

    // 4. Se align-content distribuir espaço transversal (quando há múltiplas linhas e altura definida)
    let total_lines = lines.len();
    let total_cross_gaps = if total_lines > 1 {
        config.cross_gap.saturating_mul((total_lines - 1) as i32)
    } else {
        LayoutUnit::ZERO
    };

    let sum_lines_cross: LayoutUnit = lines.iter().fold(LayoutUnit::ZERO, |acc, l| {
        acc.saturating_add(l.line_cross_size)
    });

    let available_cross = config
        .container_cross_size
        .unwrap_or_else(|| sum_lines_cross.saturating_add(total_cross_gaps));

    // Posiciona as linhas no eixo transversal
    position_lines_cross_axis(
        &mut lines,
        available_cross,
        sum_lines_cross,
        total_cross_gaps,
        config.cross_gap,
        config.align_content,
        config.wrap == FlexWrap::WrapReverse,
    );

    // 5. Alinha itens individualmente no eixo transversal da sua respectiva linha (`align-items` / `align-self`)
    for line in lines.iter_mut() {
        let line_cross = line.line_cross_size;
        for item in line.items.iter_mut() {
            let alignment = match item.align_self {
                AlignSelf::Auto => match config.align_items {
                    AlignItems::Stretch => AlignSelf::Stretch,
                    AlignItems::FlexStart => AlignSelf::FlexStart,
                    AlignItems::FlexEnd => AlignSelf::FlexEnd,
                    AlignItems::Center => AlignSelf::Center,
                    AlignItems::Baseline => AlignSelf::Baseline,
                },
                other => other,
            };

            // Se for stretch e item não tiver cross size fixo rígido
            if alignment == AlignSelf::Stretch {
                let available_for_item = line_cross
                    .saturating_sub(item.margins_cross.0)
                    .saturating_sub(item.margins_cross.1);
                if available_for_item > item.target_cross_size {
                    item.target_cross_size = available_for_item;
                }
            }

            let item_outer_cross = item.outer_cross_size();
            let cross_free = line_cross
                .saturating_sub(item_outer_cross)
                .max(LayoutUnit::ZERO);

            item.cross_offset = match alignment {
                AlignSelf::FlexStart | AlignSelf::Baseline => item.margins_cross.0,
                AlignSelf::FlexEnd => line_cross
                    .saturating_sub(item.margins_cross.1)
                    .saturating_sub(item.target_cross_size),
                AlignSelf::Center => item
                    .margins_cross
                    .0
                    .saturating_add(cross_free.saturating_div(2)),
                AlignSelf::Stretch | AlignSelf::Auto => item.margins_cross.0,
            };
        }
    }

    // Calcula dimensões finais do contêiner
    let final_main = if let Some(m) = config.container_main_size {
        m
    } else {
        lines
            .iter()
            .fold(LayoutUnit::ZERO, |acc, l| acc.max(l.line_main_size))
    };

    let final_cross = available_cross;

    (lines, final_main, final_cross)
}

/// Alinha itens ao longo do eixo principal de uma linha (`justify-content`).
fn align_items_main_axis(
    line: &mut FlexLine,
    justify: JustifyContent,
    gap: LayoutUnit,
    is_reversed: bool,
) {
    let count = line.items.len();
    if count == 0 {
        return;
    }

    let sum_outer_items: LayoutUnit = line.items.iter().fold(LayoutUnit::ZERO, |acc, it| {
        acc.saturating_add(it.outer_main_size())
    });

    let total_gaps = if count > 1 {
        gap.saturating_mul((count - 1) as i32)
    } else {
        LayoutUnit::ZERO
    };

    let free_space = line
        .line_main_size
        .saturating_sub(sum_outer_items)
        .saturating_sub(total_gaps);

    let free_positive = free_space.max(LayoutUnit::ZERO);

    let (start_offset, item_spacing) = match justify {
        JustifyContent::FlexStart => (LayoutUnit::ZERO, gap),
        JustifyContent::FlexEnd => (free_positive, gap),
        JustifyContent::Center => (free_positive.saturating_div(2), gap),
        JustifyContent::SpaceBetween => {
            if count == 1 {
                (LayoutUnit::ZERO, gap)
            } else {
                let extra_gap = free_positive.saturating_div((count - 1) as i32);
                (LayoutUnit::ZERO, gap.saturating_add(extra_gap))
            }
        }
        JustifyContent::SpaceAround => {
            let per_item_margin = free_positive.saturating_div(count as i32);
            let half = per_item_margin.saturating_div(2);
            (half, gap.saturating_add(per_item_margin))
        }
        JustifyContent::SpaceEvenly => {
            let space_unit = free_positive.saturating_div((count + 1) as i32);
            (space_unit, gap.saturating_add(space_unit))
        }
    };

    let mut current_offset = start_offset;
    for item in line.items.iter_mut() {
        item.main_offset = current_offset.saturating_add(item.margins_main.0);
        current_offset = current_offset
            .saturating_add(item.outer_main_size())
            .saturating_add(item_spacing);
    }

    // Se a direção for reversa (RowReverse ou ColumnReverse), inverte os offsets
    if is_reversed {
        let line_len = line.line_main_size;
        for item in line.items.iter_mut() {
            item.main_offset = line_len
                .saturating_sub(item.main_offset)
                .saturating_sub(item.target_main_size);
        }
    }
}

/// Posiciona múltiplas linhas no eixo transversal (`align-content`).
fn position_lines_cross_axis(
    lines: &mut [FlexLine],
    container_cross: LayoutUnit,
    sum_lines_cross: LayoutUnit,
    total_gaps: LayoutUnit,
    cross_gap: LayoutUnit,
    align_content: AlignContent,
    wrap_reverse: bool,
) {
    let count = lines.len();
    if count == 0 {
        return;
    }

    let free_cross = container_cross
        .saturating_sub(sum_lines_cross)
        .saturating_sub(total_gaps)
        .max(LayoutUnit::ZERO);

    let (start_offset, line_spacing) = match align_content {
        AlignContent::FlexStart => (LayoutUnit::ZERO, cross_gap),
        AlignContent::FlexEnd => (free_cross, cross_gap),
        AlignContent::Center => (free_cross.saturating_div(2), cross_gap),
        AlignContent::SpaceBetween => {
            if count == 1 {
                (LayoutUnit::ZERO, cross_gap)
            } else {
                let extra = free_cross.saturating_div((count - 1) as i32);
                (LayoutUnit::ZERO, cross_gap.saturating_add(extra))
            }
        }
        AlignContent::SpaceAround => {
            let per_line = free_cross.saturating_div(count as i32);
            (
                per_line.saturating_div(2),
                cross_gap.saturating_add(per_line),
            )
        }
        AlignContent::Stretch => {
            // Se stretch e houver espaço livre, distribui equitativamente entre as linhas
            let extra = free_cross.saturating_div(count as i32);
            for line in lines.iter_mut() {
                line.line_cross_size = line.line_cross_size.saturating_add(extra);
            }
            (LayoutUnit::ZERO, cross_gap)
        }
    };

    let mut current_cross = start_offset;
    for line in lines.iter_mut() {
        line.cross_offset = current_cross;
        current_cross = current_cross
            .saturating_add(line.line_cross_size)
            .saturating_add(line_spacing);
    }

    if wrap_reverse {
        for line in lines.iter_mut() {
            line.cross_offset = container_cross
                .saturating_sub(line.cross_offset)
                .saturating_sub(line.line_cross_size);
        }
    }
}

/// Mapeia coordenadas de linha (main, cross) para coordenadas lógicas da caixa (inline, block).
pub fn map_to_logical_rect(
    axis: FlexAxis,
    item_main_offset: LayoutUnit,
    item_main_size: LayoutUnit,
    item_cross_offset: LayoutUnit,
    item_cross_size: LayoutUnit,
    line_cross_offset: LayoutUnit,
) -> LogicalRect<LayoutUnit> {
    let global_cross = line_cross_offset.saturating_add(item_cross_offset);

    if axis.is_row() {
        LogicalRect {
            inline_start: item_main_offset,
            block_start: global_cross,
            inline_size: item_main_size,
            block_size: item_cross_size,
        }
    } else {
        LogicalRect {
            inline_start: global_cross,
            block_start: item_main_offset,
            inline_size: item_cross_size,
            block_size: item_main_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_dummy_item(id: u64, grow: f32, basis_px: i32, cross_px: i32) -> FlexItem {
        FlexItem {
            node_id: NodeId::from_raw(id).unwrap_or_default(),
            flex_grow: grow,
            flex_shrink: 1.0,
            flex_basis: LayoutUnit::from_px(basis_px),
            min_main_size: LayoutUnit::ZERO,
            max_main_size: None,
            hypothetical_main_size: LayoutUnit::from_px(basis_px),
            target_main_size: LayoutUnit::from_px(basis_px),
            hypothetical_cross_size: LayoutUnit::from_px(cross_px),
            target_cross_size: LayoutUnit::from_px(cross_px),
            margins_main: (LayoutUnit::ZERO, LayoutUnit::ZERO),
            margins_cross: (LayoutUnit::ZERO, LayoutUnit::ZERO),
            borders_padding_main: LayoutUnit::ZERO,
            borders_padding_cross: LayoutUnit::ZERO,
            align_self: AlignSelf::Auto,
            main_offset: LayoutUnit::ZERO,
            cross_offset: LayoutUnit::ZERO,
            margins: LogicalSides::default(),
            borders: LogicalSides::default(),
            paddings: LogicalSides::default(),
        }
    }

    #[test]
    fn test_flex_line_splitting_wrap() {
        let items = vec![
            create_dummy_item(1, 0.0, 100, 50),
            create_dummy_item(2, 0.0, 150, 50),
            create_dummy_item(3, 0.0, 100, 50),
        ];

        let container_w = Some(LayoutUnit::from_px(200));
        let lines = split_into_flex_lines(items, FlexWrap::Wrap, container_w, LayoutUnit::ZERO);

        // 100 + 150 = 250 > 200, so item 2 breaks onto line 2!
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].items.len(), 1);
        assert_eq!(lines[1].items.len(), 1);
        assert_eq!(lines[2].items.len(), 1);
    }

    #[test]
    fn test_justify_content_center_and_space_between() {
        let items = vec![
            create_dummy_item(1, 0.0, 50, 40),
            create_dummy_item(2, 0.0, 50, 40),
        ];

        let config_center = FlexContainerConfig {
            axis: FlexAxis::Row,
            wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            align_content: AlignContent::Stretch,
            main_gap: LayoutUnit::ZERO,
            cross_gap: LayoutUnit::ZERO,
            container_main_size: Some(LayoutUnit::from_px(200)),
            container_cross_size: Some(LayoutUnit::from_px(100)),
        };

        let lines = vec![FlexLine::new(items.clone())];
        let (res_center, _, _) = layout_flex_lines(lines, &config_center);

        // Free space = 100px. Offset start = 50px.
        assert_eq!(res_center[0].items[0].main_offset, LayoutUnit::from_px(50));
        assert_eq!(res_center[0].items[1].main_offset, LayoutUnit::from_px(100));

        let config_space_between = FlexContainerConfig {
            justify_content: JustifyContent::SpaceBetween,
            ..config_center
        };
        let lines_sb = vec![FlexLine::new(items)];
        let (res_sb, _, _) = layout_flex_lines(lines_sb, &config_space_between);

        // Item 0 at 0, Item 1 at 200 - 50 = 150px
        assert_eq!(res_sb[0].items[0].main_offset, LayoutUnit::from_px(0));
        assert_eq!(res_sb[0].items[1].main_offset, LayoutUnit::from_px(150));
    }
}
