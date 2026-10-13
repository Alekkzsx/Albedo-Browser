//! # Contexto de Formatação em Linha (`InlineFormattingContext`)
//!
//! Processa caixas de nível inline e sequências de texto (`TextRun`), invoca o `LineBreaker`,
//! empacota fragmentos em `LineBoxFragment`s, executa alinhamento vertical e horizontal,
//! e calcula as geometrias completas do IFC.

use std::sync::Arc;

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{Direction, LogicalRect, LogicalSize};
use ace_style::computed::structs::StyleFont;
use ace_style::computed::ComputedStyle;
use ace_style::values::enums::{TextAlign, VerticalAlign as StyleVerticalAlign, WhiteSpace, WordBreak};
use smol_str::SmolStr;

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
        if children.is_empty() {
            return IfcLayoutResult {
                line_boxes: Vec::new(),
                size: LogicalSize::new(LayoutUnit::ZERO, LayoutUnit::ZERO),
            };
        }

        let available_width = space
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

        // 2. Empacota itens em caixas de linha (LineBoxFragment)
        let mut line_boxes = Vec::new();
        let mut current_line = LineBoxFragment::new(LayoutUnit::ZERO);
        let mut current_line_width = LayoutUnit::ZERO;

        for item in items {
            match item {
                UnpackedInlineItem::TextWord {
                    text,
                    width,
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
                            line_boxes.push(current_line);
                            current_line = LineBoxFragment::new(LayoutUnit::ZERO);
                            current_line_width = LayoutUnit::ZERO;
                        }
                        continue;
                    }

                    // Se não couber na linha atual e a linha atual já possuir itens
                    if current_line_width.saturating_add(width) > available_width
                        && !current_line.fragments.is_empty()
                        && !is_space
                    {
                        current_line.bounds.inline_size = current_line_width;
                        line_boxes.push(current_line);
                        current_line = LineBoxFragment::new(LayoutUnit::ZERO);
                        current_line_width = LayoutUnit::ZERO;
                    }

                    // Se for espaço no início da linha nova (colapsável), descarta
                    if is_space && current_line.fragments.is_empty() {
                        continue;
                    }

                    let height = font
                        .line_height
                        .unwrap_or_else(|| ascent.saturating_add(descent));

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
                    // Se não couber na linha atual e a linha não estiver vazia
                    if current_line_width.saturating_add(size.inline_size) > available_width
                        && !current_line.fragments.is_empty()
                    {
                        current_line.bounds.inline_size = current_line_width;
                        line_boxes.push(current_line);
                        current_line = LineBoxFragment::new(LayoutUnit::ZERO);
                        current_line_width = LayoutUnit::ZERO;
                    }

                    let frag = InlineFragment::Box {
                        id,
                        bounds: LogicalRect::new(
                            current_line_width,
                            LayoutUnit::ZERO,
                            size.inline_size,
                            size.block_size,
                        ),
                        baseline,
                        ascent: baseline,
                        descent: size.block_size.saturating_sub(baseline),
                        vertical_align,
                    };

                    current_line.add_fragment(frag);
                    current_line_width = current_line_width.saturating_add(size.inline_size);
                }
            }
        }

        // Adiciona a última linha restante
        if !current_line.fragments.is_empty() {
            current_line.bounds.inline_size = current_line_width;
            line_boxes.push(current_line);
        }

        // 3. Aplica alinhamento de baseline e horizontal e acumula alturas verticais
        let mut total_block_size = LayoutUnit::ZERO;
        let mut max_inline_size = LayoutUnit::ZERO;
        let line_count = line_boxes.len();

        for (idx, line) in line_boxes.iter_mut().enumerate() {
            let is_last_line = idx + 1 == line_count;

            // Alinhamento vertical das baselines
            line.apply_baseline_alignment();

            // Alinhamento horizontal de texto
            line.apply_horizontal_alignment(available_width, text_align, direction, is_last_line);

            // Posiciona a linha no eixo vertical (block)
            line.bounds.block_start = total_block_size;
            total_block_size = total_block_size.saturating_add(line.bounds.block_size);
            max_inline_size = max_inline_size.max(line.bounds.inline_size);
        }

        IfcLayoutResult {
            line_boxes,
            size: LogicalSize::new(max_inline_size, total_block_size),
        }
    }

    /// Descompacta recursivamente nós da Box Tree em itens inline primitivos.
    fn collect_items(
        node: &LayoutBox,
        measurer: &dyn TextMeasurer,
        out: &mut Vec<UnpackedInlineItem>,
    ) {
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
                let vertical_align = Self::map_vertical_align(node.style.box_style.vertical_align);

                // Usa o LineBreaker para quebrar o texto em tokens/palavras normativos
                let prepared = LineBreaker::prepare_text(text_str, white_space);
                let opps = LineBreaker::find_break_opportunities(&prepared, white_space, word_break);

                let mut last_idx = 0;
                for opp in opps {
                    if opp.byte_offset <= last_idx || opp.byte_offset > prepared.len() {
                        continue;
                    }
                    let chunk = &prepared[last_idx..opp.byte_offset];
                    let chunk_metrics = measurer.measure_text(chunk, &font);

                    out.push(UnpackedInlineItem::TextWord {
                        text: SmolStr::new(chunk),
                        width: chunk_metrics.width,
                        font: font.clone(),
                        ascent: metrics.ascent,
                        descent: metrics.descent,
                        vertical_align,
                        is_space: opp.is_space,
                        is_mandatory_break: opp.is_mandatory,
                    });

                    last_idx = opp.byte_offset;
                }

                // Chunk final
                if last_idx < prepared.len() {
                    let chunk = &prepared[last_idx..];
                    let chunk_metrics = measurer.measure_text(chunk, &font);

                    out.push(UnpackedInlineItem::TextWord {
                        text: SmolStr::new(chunk),
                        width: chunk_metrics.width,
                        font: font.clone(),
                        ascent: metrics.ascent,
                        descent: metrics.descent,
                        vertical_align,
                        is_space: false,
                        is_mandatory_break: false,
                    });
                }
            }
            LayoutBoxKind::Inline => {
                for child in &node.children {
                    Self::collect_items(child, measurer, out);
                }
            }
            LayoutBoxKind::InlineBlock => {
                // Caixa atômica inline-block
                let w = node.style.width().unwrap_or_else(|| {
                    node.compute_min_max_sizes().min_content
                });
                let h = node.style.height().unwrap_or(LayoutUnit::from_px(20));
                let vertical_align = Self::map_vertical_align(node.style.box_style.vertical_align);

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

    /// Mapeia o alinhamento de estilo para a representação local de linha.
    fn map_vertical_align(align: StyleVerticalAlign) -> VerticalAlign {
        match align {
            StyleVerticalAlign::Baseline => VerticalAlign::Baseline,
            StyleVerticalAlign::Top => VerticalAlign::Top,
            StyleVerticalAlign::Middle => VerticalAlign::Middle,
            StyleVerticalAlign::Bottom => VerticalAlign::Bottom,
            _ => VerticalAlign::Baseline,
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

        let result = ifc.layout_children(&[node], &Arc::new(ComputedStyle::initial()), &space, &measurer);

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

        let result = ifc.layout_children(&[node], &Arc::new(ComputedStyle::initial()), &space, &measurer);

        assert_eq!(result.line_boxes.len(), 3);
        assert_eq!(result.size.block_size, LayoutUnit::from_px(54)); // 3 * 18px = 54px
    }

    #[test]
    fn test_ifc_inline_block_packing() {
        let measurer = FixedTextMeasurer::new();
        let ifc = InlineFormattingContext::new();

        let mut inline_block = LayoutBox::new(NodeId::new(), LayoutBoxKind::InlineBlock, Arc::new(ComputedStyle::initial()));
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

        let result = ifc.layout_children(&[text, inline_block], &Arc::new(ComputedStyle::initial()), &space, &measurer);

        assert_eq!(result.line_boxes.len(), 1);
        assert_eq!(result.line_boxes[0].fragments.len(), 2);
    }
}
