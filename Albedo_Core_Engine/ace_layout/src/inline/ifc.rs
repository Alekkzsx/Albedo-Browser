//! # Contexto de Formatação em Linha (`InlineFormattingContext`)
//!
//! Processa caixas de nível inline e sequências de texto (`TextRun`), invoca o `LineBreaker`,
//! empacota fragmentos em `LineBoxFragment`s, executa alinhamento vertical e horizontal,
//! e calcula as geometrias completas do IFC.

use std::sync::Arc;

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{LogicalRect, LogicalSides, LogicalSize};
use ace_style::computed::structs::StyleFont;
use ace_style::computed::ComputedStyle;
use smol_str::SmolStr;

use crate::block::exclusion::{Exclusion, ExclusionSpace, FloatType};
use crate::inline::line_box::{InlineFragment, LineBoxFragment, VerticalAlign};
use crate::inline::line_breaker::LineBreaker;
use crate::inline::text_measurer::TextMeasurer;
use crate::tree::constraint_space::ConstraintSpace;
use crate::tree::layout_box::{LayoutBox, LayoutBoxKind};

/// Resultado do cálculo geométrico do Inline Formatting Context.
#[derive(Debug, Clone, PartialEq)]
pub struct IfcLayoutResult {
    /// Lista ordenada de caixas de linha físicas formatadas.
    pub line_boxes: Vec<LineBoxFragment>,
    /// Dimensões lógicas totais ocupadas pelo IFC (`inline_size` e `block_size`).
    pub size: LogicalSize<LayoutUnit>,
}

/// Item intermediário descompactado para empacotamento em linhas.
#[derive(Debug, Clone)]
enum UnpackedInlineItem {
    TextWord {
        text: SmolStr,
        width: LayoutUnit,
        height: LayoutUnit,
        font: StyleFont,
        ascent: LayoutUnit,
        descent: LayoutUnit,
        vertical_align: VerticalAlign,
        is_space: bool,
        is_mandatory_break: bool,
    },
    AtomicBox {
        id: NodeId,
        size: LogicalSize<LayoutUnit>,
        baseline: LayoutUnit,
        vertical_align: VerticalAlign,
    },
    FloatBox {
        box_id: NodeId,
        kind: FloatType,
        size: LogicalSize<LayoutUnit>,
        margins: LogicalSides<LayoutUnit>,
    },
}

/// Motor de execução do Inline Formatting Context.
#[derive(Debug, Clone, Default)]
pub struct InlineFormattingContext;

impl InlineFormattingContext {
    /// Cria uma nova instância do motor IFC.
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Executa a formatação em linha para uma caixa contêiner e suas filhas inline.
    pub fn layout(
        &self,
        container: &LayoutBox,
        space: &ConstraintSpace,
        measurer: &dyn TextMeasurer,
    ) -> IfcLayoutResult {
        self.layout_children(&container.children, &container.style, space, measurer)
    }

    /// Executa a formatação em linha para uma lista de nós filhos inline sob o estilo do contêiner.
    pub fn layout_children(
        &self,
        children: &[LayoutBox],
        container_style: &Arc<ComputedStyle>,
        space: &ConstraintSpace,
        measurer: &dyn TextMeasurer,
    ) -> IfcLayoutResult {
        let mut exclusion_space = ExclusionSpace::new();
        self.layout_children_with_exclusions(
            children,
            container_style,
            space,
            measurer,
            &mut exclusion_space,
        )
    }

    /// Executa a formatação em linha consultando dinamicamente o `ExclusionSpace` para wrapping de floats.
    pub fn layout_children_with_exclusions(
        &self,
        children: &[LayoutBox],
        container_style: &Arc<ComputedStyle>,
        space: &ConstraintSpace,
        measurer: &dyn TextMeasurer,
        exclusion_space: &mut ExclusionSpace,
    ) -> IfcLayoutResult {
        if children.is_empty() {
            return IfcLayoutResult {
                line_boxes: Vec::new(),
                size: LogicalSize::new(LayoutUnit::ZERO, LayoutUnit::ZERO),
            };
        }

        let container_width = space
            .available_inline_size
            .to_definite()
            .unwrap_or(LayoutUnit::MAX);

        let text_align = container_style.text_align();
        let direction = space.direction;

        // 1. Coleta e descompacta todos os itens inline recursivamente
        let mut items = Vec::new();
        for child in children {
            Self::collect_items(child, measurer, &mut items);
        }

        if items.is_empty() {
            return IfcLayoutResult {
                line_boxes: Vec::new(),
                size: LogicalSize::new(LayoutUnit::ZERO, LayoutUnit::ZERO),
            };
        }

        // 2. Empacota itens em caixas de linha (LineBoxFragment) com consulta dinâmica ao ExclusionSpace
        let mut line_boxes = Vec::new();
        let mut current_line = LineBoxFragment::new(LayoutUnit::ZERO);
        let mut current_line_width = LayoutUnit::ZERO;
        let mut current_line_y = LayoutUnit::ZERO;
        let estimated_line_height = LayoutUnit::from_px(18);

        let mut current_opp = exclusion_space.find_layout_opportunity(
            current_line_y,
            container_width,
            LayoutUnit::ZERO,
            estimated_line_height,
        );
        let mut line_inline_start = current_opp.inline_start;
        let mut line_available_width = current_opp.inline_size;

        for item in items {
            match item {
                UnpackedInlineItem::FloatBox {
                    box_id,
                    kind,
                    size,
                    margins,
                } => {
                    let float_margin_w = size
                        .inline_size
                        .saturating_add(margins.inline_start)
                        .saturating_add(margins.inline_end);
                    let float_margin_h = size
                        .block_size
                        .saturating_add(margins.block_start)
                        .saturating_add(margins.block_end);

                    let float_opp = exclusion_space.find_layout_opportunity(
                        current_line_y,
                        container_width,
                        float_margin_w,
                        float_margin_h,
                    );

                    let (margin_x, margin_y) = match kind {
                        FloatType::Left => (float_opp.inline_start, float_opp.block_start),
                        FloatType::Right => (
                            float_opp
                                .inline_end()
                                .saturating_sub(float_margin_w)
                                .max(float_opp.inline_start),
                            float_opp.block_start,
                        ),
                    };

                    let margin_rect =
                        LogicalRect::new(margin_x, margin_y, float_margin_w, float_margin_h);
                    exclusion_space.add_exclusion(Exclusion::new(margin_rect, kind, box_id));

                    // Atualiza oportunidade restante na linha atual
                    current_opp = exclusion_space.find_layout_opportunity(
                        current_line_y,
                        container_width,
                        LayoutUnit::ZERO,
                        estimated_line_height,
                    );
                    line_inline_start = current_opp.inline_start;
                    line_available_width = current_opp.inline_size;
                }
                UnpackedInlineItem::TextWord {
                    text,
                    width,
                    height,
                    font,
                    ascent,
                    descent,
                    vertical_align,
                    is_space,
                    is_mandatory_break,
                } => {
                    if is_mandatory_break {
                        // Quebra de linha forçada mandatória ('\n')
                        if !current_line.fragments.is_empty() {
                            current_line.bounds.inline_size = current_line_width;
                            current_line.apply_baseline_alignment();
                            current_line.apply_horizontal_alignment_with_inset(
                                line_inline_start,
                                line_available_width,
                                text_align,
                                direction,
                                false,
                            );
                            current_line.bounds.block_start = current_line_y;
                            current_line_y =
                                current_line_y.saturating_add(current_line.bounds.block_size);
                            line_boxes.push(current_line);
                            current_line = LineBoxFragment::new(current_line_y);
                            current_line_width = LayoutUnit::ZERO;

                            current_opp = exclusion_space.find_layout_opportunity(
                                current_line_y,
                                container_width,
                                LayoutUnit::ZERO,
                                estimated_line_height,
                            );
                            line_inline_start = current_opp.inline_start;
                            line_available_width = current_opp.inline_size;
                        }
                        continue;
                    }

                    // Se a linha estiver vazia e a palavra não couber no espaço disponível,
                    // salta para a próxima prateleira onde ela caiba
                    if current_line.fragments.is_empty() && width > line_available_width {
                        let opp = exclusion_space.find_layout_opportunity(
                            current_line_y,
                            container_width,
                            width,
                            estimated_line_height,
                        );
                        current_line_y = opp.block_start;
                        current_line.bounds.block_start = current_line_y;
                        line_inline_start = opp.inline_start;
                        line_available_width = opp.inline_size;
                    }

                    // Se não couber na linha atual e a linha atual já possuir itens -> quebra de linha
                    if current_line_width.saturating_add(width) > line_available_width
                        && !current_line.fragments.is_empty()
                    {
                        current_line.bounds.inline_size = current_line_width;
                        current_line.apply_baseline_alignment();
                        current_line.apply_horizontal_alignment_with_inset(
                            line_inline_start,
                            line_available_width,
                            text_align,
                            direction,
                            false,
                        );
                        current_line.bounds.block_start = current_line_y;
                        current_line_y =
                            current_line_y.saturating_add(current_line.bounds.block_size);
                        line_boxes.push(current_line);
                        current_line = LineBoxFragment::new(current_line_y);
                        current_line_width = LayoutUnit::ZERO;

                        current_opp = exclusion_space.find_layout_opportunity(
                            current_line_y,
                            container_width,
                            LayoutUnit::ZERO,
                            estimated_line_height,
                        );
                        if width > current_opp.inline_size {
                            current_opp = exclusion_space.find_layout_opportunity(
                                current_line_y,
                                container_width,
                                width,
                                estimated_line_height,
                            );
                            current_line_y = current_opp.block_start;
                            current_line.bounds.block_start = current_line_y;
                        }
                        line_inline_start = current_opp.inline_start;
                        line_available_width = current_opp.inline_size;
                    }

                    // Se for espaço no início da linha nova (colapsável), descarta
                    if is_space && current_line.fragments.is_empty() {
                        continue;
                    }

                    let frag = InlineFragment::Text {
                        text,
                        font,
                        bounds: LogicalRect::new(
                            current_line_width,
                            LayoutUnit::ZERO,
                            width,
                            height,
                        ),
                        ascent,
                        descent,
                        vertical_align,
                    };

                    current_line.add_fragment(frag);
                    current_line_width = current_line_width.saturating_add(width);
                }
                UnpackedInlineItem::AtomicBox {
                    id,
                    size,
                    baseline,
                    vertical_align,
                } => {
                    let w = size.inline_size;
                    let h = size.block_size;

                    if current_line.fragments.is_empty() && w > line_available_width {
                        let opp = exclusion_space.find_layout_opportunity(
                            current_line_y,
                            container_width,
                            w,
                            estimated_line_height,
                        );
                        current_line_y = opp.block_start;
                        current_line.bounds.block_start = current_line_y;
                        line_inline_start = opp.inline_start;
                        line_available_width = opp.inline_size;
                    }

                    if current_line_width.saturating_add(w) > line_available_width
                        && !current_line.fragments.is_empty()
                    {
                        current_line.bounds.inline_size = current_line_width;
                        current_line.apply_baseline_alignment();
                        current_line.apply_horizontal_alignment_with_inset(
                            line_inline_start,
                            line_available_width,
                            text_align,
                            direction,
                            false,
                        );
                        current_line.bounds.block_start = current_line_y;
                        current_line_y =
                            current_line_y.saturating_add(current_line.bounds.block_size);
                        line_boxes.push(current_line);
                        current_line = LineBoxFragment::new(current_line_y);
                        current_line_width = LayoutUnit::ZERO;

                        current_opp = exclusion_space.find_layout_opportunity(
                            current_line_y,
                            container_width,
                            LayoutUnit::ZERO,
                            estimated_line_height,
                        );
                        if w > current_opp.inline_size {
                            current_opp = exclusion_space.find_layout_opportunity(
                                current_line_y,
                                container_width,
                                w,
                                estimated_line_height,
                            );
                            current_line_y = current_opp.block_start;
                            current_line.bounds.block_start = current_line_y;
                        }
                        line_inline_start = current_opp.inline_start;
                        line_available_width = current_opp.inline_size;
                    }

                    let frag = InlineFragment::Box {
                        id,
                        bounds: LogicalRect::new(current_line_width, LayoutUnit::ZERO, w, h),
                        baseline,
                        ascent: baseline,
                        descent: h.saturating_sub(baseline),
                        vertical_align,
                    };

                    current_line.add_fragment(frag);
                    current_line_width = current_line_width.saturating_add(w);
                }
            }
        }

        // Adiciona a última linha restante
        if !current_line.fragments.is_empty() {
            current_line.bounds.inline_size = current_line_width;
            current_line.apply_baseline_alignment();
            current_line.apply_horizontal_alignment_with_inset(
                line_inline_start,
                line_available_width,
                text_align,
                direction,
                true,
            );
            current_line.bounds.block_start = current_line_y;
            current_line_y = current_line_y.saturating_add(current_line.bounds.block_size);
            line_boxes.push(current_line);
        }

        let max_inline_size = line_boxes
            .iter()
            .map(|l| l.bounds.inline_start.saturating_add(l.bounds.inline_size))
            .max()
            .unwrap_or(LayoutUnit::ZERO);

        IfcLayoutResult {
            line_boxes,
            size: LogicalSize::new(max_inline_size, current_line_y),
        }
    }

    /// Descompacta recursivamente nós da Box Tree em itens inline primitivos.
    fn collect_items(
        node: &LayoutBox,
        measurer: &dyn TextMeasurer,
        out: &mut Vec<UnpackedInlineItem>,
    ) {
        if node.style.box_style.float.is_floating() {
            let float_kind =
                FloatType::from_style(node.style.box_style.float).unwrap_or(FloatType::Left);
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
            let w = node
                .style
                .width()
                .unwrap_or_else(|| node.compute_min_max_sizes().min_content);
            let h = node.style.height().unwrap_or(LayoutUnit::from_px(20));
            let border_w = w
                .saturating_add(paddings.inline_start)
                .saturating_add(paddings.inline_end)
                .saturating_add(borders.inline_start)
                .saturating_add(borders.inline_end);
            let border_h = h
                .saturating_add(paddings.block_start)
                .saturating_add(paddings.block_end)
                .saturating_add(borders.block_start)
                .saturating_add(borders.block_end);
            out.push(UnpackedInlineItem::FloatBox {
                box_id: node.id,
                kind: float_kind,
                size: LogicalSize::new(border_w, border_h),
                margins,
            });
            return;
        }

        match node.kind {
            LayoutBoxKind::TextRun => {
                let text_str = node.text.as_deref().unwrap_or("");
                if text_str.is_empty() {
                    return;
                }

                let white_space = node.style.white_space();
                let word_break = node.style.word_break();
                let font = (*node.style.font).clone();
                let metrics = measurer.measure_text("M", &font);
                let vertical_align = VerticalAlign::Baseline;

                // Usa o LineBreaker para quebrar o texto em tokens/palavras normativos
                let prepared = LineBreaker::prepare_text(text_str, white_space);
                let opps =
                    LineBreaker::find_break_opportunities(&prepared, white_space, word_break);

                let mut last_idx = 0;
                for opp in opps {
                    if opp.byte_offset <= last_idx || opp.byte_offset > prepared.len() {
                        continue;
                    }
                    let chunk = &prepared[last_idx..opp.byte_offset];
                    let trimmed = chunk.trim_end_matches([' ', '\t']);
                    if !trimmed.is_empty() {
                        let word_metrics = measurer.measure_text(trimmed, &font);
                        out.push(UnpackedInlineItem::TextWord {
                            text: SmolStr::new(trimmed),
                            width: word_metrics.width,
                            height: word_metrics.height,
                            font: font.clone(),
                            ascent: metrics.ascent,
                            descent: metrics.descent,
                            vertical_align,
                            is_space: false,
                            is_mandatory_break: false,
                        });
                    }
                    let spaces = &chunk[trimmed.len()..];
                    if !spaces.is_empty() {
                        let space_metrics = measurer.measure_text(spaces, &font);
                        out.push(UnpackedInlineItem::TextWord {
                            text: SmolStr::new(spaces),
                            width: space_metrics.width,
                            height: space_metrics.height,
                            font: font.clone(),
                            ascent: metrics.ascent,
                            descent: metrics.descent,
                            vertical_align,
                            is_space: true,
                            is_mandatory_break: opp.is_mandatory,
                        });
                    } else if opp.is_mandatory {
                        out.push(UnpackedInlineItem::TextWord {
                            text: SmolStr::default(),
                            width: LayoutUnit::ZERO,
                            height: metrics.height,
                            font: font.clone(),
                            ascent: metrics.ascent,
                            descent: metrics.descent,
                            vertical_align,
                            is_space: false,
                            is_mandatory_break: true,
                        });
                    }

                    last_idx = opp.byte_offset;
                }

                // Chunk final
                if last_idx < prepared.len() {
                    let chunk = &prepared[last_idx..];
                    let trimmed = chunk.trim_end_matches([' ', '\t']);
                    if !trimmed.is_empty() {
                        let word_metrics = measurer.measure_text(trimmed, &font);
                        out.push(UnpackedInlineItem::TextWord {
                            text: SmolStr::new(trimmed),
                            width: word_metrics.width,
                            height: word_metrics.height,
                            font: font.clone(),
                            ascent: metrics.ascent,
                            descent: metrics.descent,
                            vertical_align,
                            is_space: false,
                            is_mandatory_break: false,
                        });
                    }
                    let spaces = &chunk[trimmed.len()..];
                    if !spaces.is_empty() {
                        let space_metrics = measurer.measure_text(spaces, &font);
                        out.push(UnpackedInlineItem::TextWord {
                            text: SmolStr::new(spaces),
                            width: space_metrics.width,
                            height: space_metrics.height,
                            font: font.clone(),
                            ascent: metrics.ascent,
                            descent: metrics.descent,
                            vertical_align,
                            is_space: true,
                            is_mandatory_break: false,
                        });
                    }
                }
            }
            LayoutBoxKind::Inline => {
                for child in &node.children {
                    Self::collect_items(child, measurer, out);
                }
            }
            LayoutBoxKind::InlineBlock => {
                // Caixa atômica inline-block
                let w = node
                    .style
                    .width()
                    .unwrap_or_else(|| node.compute_min_max_sizes().min_content);
                let h = node.style.height().unwrap_or(LayoutUnit::from_px(20));
                let vertical_align = VerticalAlign::Baseline;

                out.push(UnpackedInlineItem::AtomicBox {
                    id: node.id,
                    size: LogicalSize::new(w, h),
                    baseline: h, // Regra normativa: baseline de inline-block sem linha interna é a borda inferior
                    vertical_align,
                });
            }
            _ => {
                // Outras caixas em contexto inline (ex: anônimas)
                for child in &node.children {
                    Self::collect_items(child, measurer, out);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline::text_measurer::FixedTextMeasurer;
    use ace_style::values::enums::Display;

    fn create_test_text_node(text: &str) -> LayoutBox {
        let style = Arc::new(ComputedStyle::initial());
        LayoutBox::new_text(NodeId::new(), style, SmolStr::new(text))
    }

    #[test]
    fn test_ifc_single_line_layout() {
        let measurer = FixedTextMeasurer::new(); // 8px por char, 18px altura
        let ifc = InlineFormattingContext::new();

        let node = create_test_text_node("hello world");
        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(200),
            LayoutUnit::from_px(100),
        ));

        let result = ifc.layout_children(
            &[node],
            &Arc::new(ComputedStyle::initial()),
            &space,
            &measurer,
        );

        assert_eq!(result.line_boxes.len(), 1);
        assert_eq!(result.size.block_size, LayoutUnit::from_px(18));
        // "hello world" tem 11 chars * 8px = 88px
        assert_eq!(result.size.inline_size, LayoutUnit::from_px(88));
    }

    #[test]
    fn test_ifc_multi_line_wrapping() {
        let measurer = FixedTextMeasurer::new(); // 8px por char
        let ifc = InlineFormattingContext::new();

        // 3 palavras: "apple" (40px) + " " + "banana" (48px) + " " + "cherry" (48px)
        let node = create_test_text_node("apple banana cherry");
        // Largura = 70px (cabe "apple", mas não "apple banana")
        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(70),
            LayoutUnit::from_px(200),
        ));

        let result = ifc.layout_children(
            &[node],
            &Arc::new(ComputedStyle::initial()),
            &space,
            &measurer,
        );

        assert_eq!(result.line_boxes.len(), 3);
        assert_eq!(result.size.block_size, LayoutUnit::from_px(54)); // 3 * 18px = 54px
    }

    #[test]
    fn test_ifc_inline_block_packing() {
        let measurer = FixedTextMeasurer::new();
        let ifc = InlineFormattingContext::new();

        let mut inline_block = LayoutBox::new(
            NodeId::new(),
            LayoutBoxKind::InlineBlock,
            Arc::new(ComputedStyle::initial()),
        );
        let mut b = (*inline_block.style.box_style).clone();
        b.display = Display::InlineBlock;
        b.width = Some(LayoutUnit::from_px(30));
        b.height = Some(LayoutUnit::from_px(30));
        inline_block.style = Arc::new(ComputedStyle {
            box_style: Arc::new(b),
            ..ComputedStyle::initial()
        });

        let text = create_test_text_node("hi"); // 2 * 8px = 16px

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(100),
            LayoutUnit::from_px(100),
        ));

        let result = ifc.layout_children(
            &[text, inline_block],
            &Arc::new(ComputedStyle::initial()),
            &space,
            &measurer,
        );

        assert_eq!(result.line_boxes.len(), 1);
        assert_eq!(result.line_boxes[0].fragments.len(), 2);
    }
}
