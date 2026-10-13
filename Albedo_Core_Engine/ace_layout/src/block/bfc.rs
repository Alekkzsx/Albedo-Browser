//! # Contexto de Formatação em Bloco (`BlockFormattingContext` / BFC)
//!
//! Implementa o motor de layout em bloco conforme W3C CSS 2.2 Capítulo 8, 9 e 10:
//! - Dimensionamento de blocos: `width: auto` (`stretch`), `box-sizing` (`border-box` vs `content-box`)
//! - Centralização horizontal e auto-margens: `margin: 0 auto` (equação §10.3.3)
//! - Colapso de margens verticais em $O(1)$ via `MarginStrut`:
//!   - Irmãos adjacentes no fluxo normal
//!   - Pai e primeiro filho (na ausência de `border-top` / `padding-top`)
//!   - Pai e último filho (na ausência de `border-bottom` / `padding-bottom` e com `height: auto`)
//!   - Blocos vazios sem backtracking recursivo
//! - Contenção de novos BFCs independentes (`overflow != visible`, `inline-block`, etc.)
//! - Integração harmoniosa com o `InlineFormattingContext` para filhos inline

use std::sync::Arc;

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{Direction, LogicalRect, LogicalSides, LogicalSize};
use ace_style::computed::ComputedStyle;
use ace_style::values::enums::{BoxSizing, Display, Overflow};

use crate::block::margin_strut::MarginStrut;
use crate::inline::ifc::InlineFormattingContext;
use crate::inline::line_box::LineBoxFragment;
use crate::inline::text_measurer::TextMeasurer;
use crate::tree::constraint_space::ConstraintSpace;
use crate::tree::layout_box::{LayoutBox, LayoutBoxKind};

/// Resultado do layout de um bloco no BFC.
#[derive(Debug, Clone, PartialEq)]
pub struct BfcLayoutResult {
    /// Identificador do nó DOM.
    pub box_id: NodeId,
    /// Caixa de margem em coordenadas lógicas relativas ao contêiner pai.
    pub margin_box: LogicalRect<LayoutUnit>,
    /// Caixa de borda (border-box).
    pub border_box: LogicalRect<LayoutUnit>,
    /// Caixa de preenchimento (padding-box).
    pub padding_box: LogicalRect<LayoutUnit>,
    /// Caixa de conteúdo (content-box).
    pub content_box: LogicalRect<LayoutUnit>,
    /// Margens computadas após resolução de auto e colapso.
    pub margins: LogicalSides<LayoutUnit>,
    /// Espessuras das bordas.
    pub borders: LogicalSides<LayoutUnit>,
    /// Espessuras dos preenchimentos (padding).
    pub paddings: LogicalSides<LayoutUnit>,
    /// Strut de margem resultante que vaza para fora (para colapso pai/último filho).
    pub end_margin_strut: MarginStrut,
    /// Filhos de bloco formatados.
    pub children: Vec<BfcLayoutResult>,
    /// Caixas de linha geradas se o bloco formatou conteúdo inline via IFC.
    pub line_boxes: Vec<LineBoxFragment>,
}

impl BfcLayoutResult {
    /// Retorna a largura da caixa de borda.
    #[inline]
    pub fn width(&self) -> LayoutUnit {
        self.border_box.inline_size
    }

    /// Retorna a altura da caixa de borda.
    #[inline]
    pub fn height(&self) -> LayoutUnit {
        self.border_box.block_size
    }

    /// Retorna a largura da caixa de conteúdo.
    #[inline]
    pub fn content_width(&self) -> LayoutUnit {
        self.content_box.inline_size
    }

    /// Retorna a altura da caixa de conteúdo.
    #[inline]
    pub fn content_height(&self) -> LayoutUnit {
        self.content_box.block_size
    }
}

/// Motor de execução do Block Formatting Context.
#[derive(Debug, Clone, Default)]
pub struct BlockFormattingContext;

impl BlockFormattingContext {
    /// Cria uma nova instância do motor BFC.
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Verifica se a caixa de layout estabelece um novo BFC independente.
    pub fn establishes_independent_bfc(node: &LayoutBox) -> bool {
        // CSS 2.2 §9.4.1: overflow != visible, inline-block, flex/grid items, etc.
        let overflow_x = node.style.overflow_x();
        let overflow_y = node.style.overflow_y();
        if overflow_x != Overflow::Visible || overflow_y != Overflow::Visible {
            return true;
        }

        let display = node.style.display();
        if display == Display::InlineBlock || display == Display::FlowRoot {
            return true;
        }

        node.kind.establishes_independent_formatting_context()
    }

    /// Executa o layout em bloco completo para uma caixa e sua subárvore.
    pub fn layout(
        &self,
        node: &LayoutBox,
        space: &ConstraintSpace,
        measurer: &dyn TextMeasurer,
    ) -> BfcLayoutResult {
        let is_new_bfc = Self::establishes_independent_bfc(node) || space.is_new_fc;

        // 1. Resolve paddings e borders
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

        let containing_width = space
            .available_inline_size
            .to_definite()
            .unwrap_or(LayoutUnit::ZERO);

        // 2. Resolve dimensionamento inline e auto-margens horizontais (CSS 2.2 §10.3.3)
        let (inline_content_size, margin_inline_start, margin_inline_end) =
            Self::compute_inline_sizing(node, containing_width, &paddings, &borders, space.direction);

        let border_box_inline_size = inline_content_size
            .saturating_add(paddings.inline_start)
            .saturating_add(paddings.inline_end)
            .saturating_add(borders.inline_start)
            .saturating_add(borders.inline_end);

        // 3. Verifica se os filhos são de nível inline ou de nível de bloco
        let has_inline_children = node.children.iter().any(|c| c.kind.is_inline_level());

        let mut children_results = Vec::new();
        let mut line_boxes = Vec::new();
        let mut content_block_size = LayoutUnit::ZERO;
        let mut end_strut = MarginStrut::empty();

        if has_inline_children {
            // Formata via InlineFormattingContext
            let ifc_space = ConstraintSpace::for_container(LogicalSize::new(
                inline_content_size,
                LayoutUnit::MAX,
            ))
            .with_direction(space.direction)
            .with_writing_mode(space.writing_mode);

            let ifc = InlineFormattingContext::new();
            let ifc_res = ifc.layout_children(&node.children, &node.style, &ifc_space, measurer);

            content_block_size = ifc_res.size.block_size;
            line_boxes = ifc_res.line_boxes;
        } else if !node.children.is_empty() {
            // Formata filhos de bloco recursivamente com colapso de margens
            let child_containing_space = ConstraintSpace::for_container(LogicalSize::new(
                inline_content_size,
                LayoutUnit::MAX,
            ))
            .with_direction(space.direction)
            .with_writing_mode(space.writing_mode);

            let mut running_strut = MarginStrut::empty();
            let mut current_block_offset = LayoutUnit::ZERO;

            let parent_can_collapse_first_child = !is_new_bfc
                && borders.block_start == LayoutUnit::ZERO
                && paddings.block_start == LayoutUnit::ZERO;

            for (child_idx, child) in node.children.iter().enumerate() {
                let child_is_new_bfc = Self::establishes_independent_bfc(child);

                let child_margin_top = child.style.margin_top().unwrap_or(LayoutUnit::ZERO);
                let child_margin_bottom = child.style.margin_bottom().unwrap_or(LayoutUnit::ZERO);

                // Verifica se o filho é um bloco vazio
                let child_paddings = LogicalSides::new(
                    child.style.padding_left(),
                    child.style.padding_right(),
                    child.style.padding_top(),
                    child.style.padding_bottom(),
                );
                let child_borders = LogicalSides::new(
                    child.style.border_width().left,
                    child.style.border_width().right,
                    child.style.border_width().top,
                    child.style.border_width().bottom,
                );
                let is_empty = Self::is_empty_block(child, &child_paddings, &child_borders);

                if is_empty {
                    // Bloco vazio: colapsa margin-top e margin-bottom no strut em O(1) sem backtracking
                    running_strut.append(child_margin_top);
                    running_strut.append(child_margin_bottom);

                    let mut child_res = self.layout(child, &child_containing_space, measurer);
                    child_res.border_box.block_start = current_block_offset;
                    child_res.border_box.block_size = LayoutUnit::ZERO;
                    child_res.margins.block_start = child_margin_top;
                    child_res.margins.block_end = child_margin_bottom;
                    children_results.push(child_res);
                    continue;
                }

                if child_is_new_bfc {
                    // Novo BFC independente: não colapsa com o strut interno
                    running_strut.append(child_margin_top);
                    let collapsed_top = running_strut.solve();
                    running_strut.reset();

                    current_block_offset = current_block_offset.saturating_add(collapsed_top);

                    let child_space = child_containing_space.clone().with_is_new_fc(true);
                    let mut child_res = self.layout(child, &child_space, measurer);
                    child_res.border_box.block_start = current_block_offset;
                    child_res.margins.block_start = collapsed_top;

                    current_block_offset = current_block_offset.saturating_add(child_res.border_box.block_size);
                    running_strut.append(child_margin_bottom);

                    children_results.push(child_res);
                    continue;
                }

                // Filho em fluxo normal
                running_strut.append(child_margin_top);

                // Colapso pai/primeiro filho
                if child_idx == 0 && parent_can_collapse_first_child {
                    // Margem do primeiro filho vaza/colapsa com o pai (não adiciona offset)
                    let mut child_res = self.layout(child, &child_containing_space, measurer);
                    child_res.border_box.block_start = LayoutUnit::ZERO;
                    current_block_offset = child_res.border_box.block_size;

                    running_strut.reset();
                    running_strut.append(child_margin_bottom);
                    running_strut.append_strut(&child_res.end_margin_strut);

                    children_results.push(child_res);
                } else {
                    let collapsed_top = running_strut.solve();
                    running_strut.reset();

                    current_block_offset = current_block_offset.saturating_add(collapsed_top);

                    let mut child_res = self.layout(child, &child_containing_space, measurer);
                    child_res.border_box.block_start = current_block_offset;
                    child_res.margins.block_start = collapsed_top;

                    current_block_offset = current_block_offset.saturating_add(child_res.border_box.block_size);

                    running_strut.append(child_margin_bottom);
                    running_strut.append_strut(&child_res.end_margin_strut);

                    children_results.push(child_res);
                }
            }

            // Colapso pai/último filho
            let parent_can_collapse_last_child = !is_new_bfc
                && node.style.height().is_none()
                && borders.block_end == LayoutUnit::ZERO
                && paddings.block_end == LayoutUnit::ZERO;

            if parent_can_collapse_last_child {
                end_strut = running_strut;
                content_block_size = current_block_offset;
            } else {
                let trailing_bottom = running_strut.solve();
                content_block_size = current_block_offset.saturating_add(trailing_bottom);
            }
        }

        // 4. Resolve block-size explícito ou automático
        let explicit_block_size = node.style.height();
        let final_content_block_size = if let Some(h) = explicit_block_size {
            match node.style.box_sizing() {
                BoxSizing::BorderBox => {
                    let non_content_v = paddings
                        .block_start
                        .saturating_add(paddings.block_end)
                        .saturating_add(borders.block_start)
                        .saturating_add(borders.block_end);
                    h.saturating_sub(non_content_v).max(LayoutUnit::ZERO)
                }
                BoxSizing::ContentBox => h,
            }
        } else {
            content_block_size
        };

        // Respeita min-height e max-height
        let final_content_block_size = if let Some(min_h) = node.style.min_height() {
            final_content_block_size.max(min_h)
        } else {
            final_content_block_size
        };

        let final_content_block_size = if let Some(max_h) = node.style.max_height() {
            final_content_block_size.min(max_h)
        } else {
            final_content_block_size
        };

        let border_box_block_size = final_content_block_size
            .saturating_add(paddings.block_start)
            .saturating_add(paddings.block_end)
            .saturating_add(borders.block_start)
            .saturating_add(borders.block_end);

        // 5. Monta retângulos geométricos
        let border_box = LogicalRect::new(
            margin_inline_start,
            LayoutUnit::ZERO,
            border_box_inline_size,
            border_box_block_size,
        );

        let padding_box = LogicalRect::new(
            border_box.inline_start.saturating_add(borders.inline_start),
            border_box.block_start.saturating_add(borders.block_start),
            border_box
                .inline_size
                .saturating_sub(borders.inline_start)
                .saturating_sub(borders.inline_end)
                .max(LayoutUnit::ZERO),
            border_box
                .block_size
                .saturating_sub(borders.block_start)
                .saturating_sub(borders.block_end)
                .max(LayoutUnit::ZERO),
        );

        let content_box = LogicalRect::new(
            padding_box.inline_start.saturating_add(paddings.inline_start),
            padding_box.block_start.saturating_add(paddings.block_start),
            inline_content_size,
            final_content_block_size,
        );

        let margin_box = LogicalRect::new(
            LayoutUnit::ZERO,
            LayoutUnit::ZERO,
            border_box_inline_size
                .saturating_add(margin_inline_start)
                .saturating_add(margin_inline_end),
            border_box_block_size,
        );

        let style_margin_top = node.style.margin_top().unwrap_or(LayoutUnit::ZERO);
        let style_margin_bottom = node.style.margin_bottom().unwrap_or(LayoutUnit::ZERO);

        BfcLayoutResult {
            box_id: node.id,
            margin_box,
            border_box,
            padding_box,
            content_box,
            margins: LogicalSides::new(
                margin_inline_start,
                margin_inline_end,
                style_margin_top,
                style_margin_bottom,
            ),
            borders,
            paddings,
            end_margin_strut: end_strut,
            children: children_results,
            line_boxes,
        }
    }

    /// Calcula o dimensionamento inline e a resolução de auto-margens (CSS 2.2 §10.3.3).
    fn compute_inline_sizing(
        node: &LayoutBox,
        containing_width: LayoutUnit,
        paddings: &LogicalSides<LayoutUnit>,
        borders: &LogicalSides<LayoutUnit>,
        direction: Direction,
    ) -> (LayoutUnit, LayoutUnit, LayoutUnit) {
        let non_content_h = paddings
            .inline_start
            .saturating_add(paddings.inline_end)
            .saturating_add(borders.inline_start)
            .saturating_add(borders.inline_end);

        let margin_left_opt = node.style.margin_left();
        let margin_right_opt = node.style.margin_right();

        let explicit_width_opt = node.style.width();

        let mut content_width = match explicit_width_opt {
            Some(w) => match node.style.box_sizing() {
                BoxSizing::BorderBox => w.saturating_sub(non_content_h).max(LayoutUnit::ZERO),
                BoxSizing::ContentBox => w,
            },
            None => {
                // width: auto (stretch)
                let ml = margin_left_opt.unwrap_or(LayoutUnit::ZERO);
                let mr = margin_right_opt.unwrap_or(LayoutUnit::ZERO);
                let total_deduction = non_content_h.saturating_add(ml).saturating_add(mr);
                containing_width
                    .saturating_sub(total_deduction)
                    .max(LayoutUnit::ZERO)
            }
        };

        // Aplica min-width e max-width
        if let Some(min_w) = node.style.min_width() {
            let eff_min = match node.style.box_sizing() {
                BoxSizing::BorderBox => min_w.saturating_sub(non_content_h).max(LayoutUnit::ZERO),
                BoxSizing::ContentBox => min_w,
            };
            content_width = content_width.max(eff_min);
        }

        if let Some(max_w) = node.style.max_width() {
            let eff_max = match node.style.box_sizing() {
                BoxSizing::BorderBox => max_w.saturating_sub(non_content_h).max(LayoutUnit::ZERO),
                BoxSizing::ContentBox => max_w,
            };
            content_width = content_width.min(eff_max);
        }

        let total_border_box_width = content_width.saturating_add(non_content_h);
        let free_space = containing_width.saturating_sub(total_border_box_width);

        // Resolução de auto margins conforme CSS 2.2 §10.3.3
        let (margin_inline_start, margin_inline_end) = match (margin_left_opt, margin_right_opt) {
            (None, None) => {
                // margin: 0 auto (ambas auto)
                if free_space >= LayoutUnit::ZERO {
                    let half = free_space.saturating_div(2);
                    let other = free_space.saturating_sub(half);
                    (half, other)
                } else {
                    // Over-constrained
                    if direction == Direction::Ltr {
                        (LayoutUnit::ZERO, free_space)
                    } else {
                        (free_space, LayoutUnit::ZERO)
                    }
                }
            }
            (None, Some(mr)) => {
                // margin-left: auto
                let ml = free_space.saturating_sub(mr);
                (ml, mr)
            }
            (Some(ml), None) => {
                // margin-right: auto
                let mr = free_space.saturating_sub(ml);
                (ml, mr)
            }
            (Some(ml), Some(mr)) => {
                // Ambas definidas
                if direction == Direction::Ltr {
                    (ml, mr)
                } else {
                    (mr, ml)
                }
            }
        };

        (content_width, margin_inline_start, margin_inline_end)
    }

    /// Detecta se um bloco é vazio para fins de colapso de margens conforme CSS 2.2 §8.3.1.
    fn is_empty_block(
        node: &LayoutBox,
        paddings: &LogicalSides<LayoutUnit>,
        borders: &LogicalSides<LayoutUnit>,
    ) -> bool {
        // Tem padding ou borda vertical -> não é vazio
        if paddings.block_start > LayoutUnit::ZERO
            || paddings.block_end > LayoutUnit::ZERO
            || borders.block_start > LayoutUnit::ZERO
            || borders.block_end > LayoutUnit::ZERO
        {
            return false;
        }

        // Tem altura explícita > 0 -> não é vazio
        if let Some(h) = node.style.height() {
            if h > LayoutUnit::ZERO {
                return false;
            }
        }

        // Se tem texto -> não é vazio
        if let Some(ref txt) = node.text {
            if !txt.trim().is_empty() {
                return false;
            }
        }

        // Sem filhos -> é vazio
        if node.children.is_empty() {
            return true;
        }

        // Todos os filhos são vazios -> é vazio
        node.children.iter().all(|c| {
            let cp = LogicalSides::new(
                c.style.padding_left(),
                c.style.padding_right(),
                c.style.padding_top(),
                c.style.padding_bottom(),
            );
            let cb = LogicalSides::new(
                c.style.border_width().left,
                c.style.border_width().right,
                c.style.border_width().top,
                c.style.border_width().bottom,
            );
            Self::is_empty_block(c, &cp, &cb)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline::text_measurer::FixedTextMeasurer;

    fn create_block_box(w: Option<i32>, h: Option<i32>) -> LayoutBox {
        let style = Arc::new(ComputedStyle::initial());
        let mut b = (*style.box_style).clone();
        b.display = Display::Block;
        b.width = w.map(LayoutUnit::from_px);
        b.height = h.map(LayoutUnit::from_px);
        LayoutBox::new(
            NodeId::new(),
            LayoutBoxKind::Block,
            Arc::new(ComputedStyle {
                box_style: Arc::new(b),
                ..(*style).clone()
            }),
        )
    }

    #[test]
    fn test_bfc_width_auto_stretch() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        let mut node = create_block_box(None, Some(100)); // width: auto
        let mut s = (*node.style.surrounding).clone();
        s.padding.left = LayoutUnit::from_px(10);
        s.padding.right = LayoutUnit::from_px(15);
        s.border_width.left = LayoutUnit::from_px(2);
        s.border_width.right = LayoutUnit::from_px(3);
        node.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s),
            ..(*node.style).clone()
        });

        // Contêiner de 500px:
        // content_width = 500 - (10 + 15 + 2 + 3) = 470px
        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(500),
            LayoutUnit::from_px(600),
        ));

        let res = bfc.layout(&node, &space, &measurer);

        assert_eq!(res.content_width(), LayoutUnit::from_px(470));
        assert_eq!(res.width(), LayoutUnit::from_px(500));
        assert_eq!(res.height(), LayoutUnit::from_px(100));
    }

    #[test]
    fn test_bfc_box_sizing_border_box() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        let mut node = create_block_box(Some(300), Some(200));
        let mut b = (*node.style.box_style).clone();
        b.box_sizing = BoxSizing::BorderBox;
        let mut s = (*node.style.surrounding).clone();
        s.padding.left = LayoutUnit::from_px(20);
        s.padding.right = LayoutUnit::from_px(20);
        node.style = Arc::new(ComputedStyle {
            box_style: Arc::new(b),
            surrounding: Arc::new(s),
            ..(*node.style).clone()
        });

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(800),
            LayoutUnit::from_px(600),
        ));

        let res = bfc.layout(&node, &space, &measurer);

        // Border-box: width é 300px, content_width = 300 - 40 = 260px
        assert_eq!(res.width(), LayoutUnit::from_px(300));
        assert_eq!(res.content_width(), LayoutUnit::from_px(260));
    }

    #[test]
    fn test_bfc_margin_auto_centering() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        let mut node = create_block_box(Some(200), Some(100));
        // margin-left: auto, margin-right: auto
        let mut s = (*node.style.surrounding).clone();
        s.margin.left = None;
        s.margin.right = None;
        node.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s),
            ..(*node.style).clone()
        });

        // Contêiner de 600px, caixa de 200px -> 400px sobra -> 200px cada margem
        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(600),
            LayoutUnit::from_px(600),
        ));

        let res = bfc.layout(&node, &space, &measurer);

        assert_eq!(res.border_box.inline_start, LayoutUnit::from_px(200));
        assert_eq!(res.margins.inline_start, LayoutUnit::from_px(200));
        assert_eq!(res.margins.inline_end, LayoutUnit::from_px(200));
    }

    #[test]
    fn test_bfc_sibling_margin_collapsing() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        let mut parent = create_block_box(Some(400), None);

        // Filho 1: 50px de altura, margin-bottom = 30px
        let mut child1 = create_block_box(Some(200), Some(50));
        let mut s1 = (*child1.style.surrounding).clone();
        s1.margin.bottom = Some(LayoutUnit::from_px(30));
        child1.style = Arc::new(ComputedStyle { surrounding: Arc::new(s1), ..(*child1.style).clone() });

        // Filho 2: 50px de altura, margin-top = 20px (menor), margin-bottom = 10px
        let mut child2 = create_block_box(Some(200), Some(50));
        let mut s2 = (*child2.style.surrounding).clone();
        s2.margin.top = Some(LayoutUnit::from_px(20));
        s2.margin.bottom = Some(LayoutUnit::from_px(10));
        child2.style = Arc::new(ComputedStyle { surrounding: Arc::new(s2), ..(*child2.style).clone() });

        parent.add_child(child1);
        parent.add_child(child2);

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(400),
            LayoutUnit::from_px(800),
        ));

        let res = bfc.layout(&parent, &space, &measurer);

        assert_eq!(res.children.len(), 2);
        // Filho 1 começa em y = 0, termina em y = 50px
        assert_eq!(res.children[0].border_box.block_start, LayoutUnit::ZERO);
        assert_eq!(res.children[0].height(), LayoutUnit::from_px(50));

        // Filho 2 deve começar em 50px + max(30, 20) = 50 + 30 = 80px!
        assert_eq!(res.children[1].border_box.block_start, LayoutUnit::from_px(80));
    }

    #[test]
    fn test_bfc_empty_block_margin_collapsing() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        let mut parent = create_block_box(Some(400), None);

        // Filho 1: 50px altura, margin-bottom = 20px
        let mut child1 = create_block_box(Some(200), Some(50));
        let mut s1 = (*child1.style.surrounding).clone();
        s1.margin.bottom = Some(LayoutUnit::from_px(20));
        child1.style = Arc::new(ComputedStyle { surrounding: Arc::new(s1), ..(*child1.style).clone() });

        // Filho 2 (vazio): height = auto, sem filhos, margin-top = 30px, margin-bottom = 40px
        let mut child2 = create_block_box(None, None);
        let mut s2 = (*child2.style.surrounding).clone();
        s2.margin.top = Some(LayoutUnit::from_px(30));
        s2.margin.bottom = Some(LayoutUnit::from_px(40));
        child2.style = Arc::new(ComputedStyle { surrounding: Arc::new(s2), ..(*child2.style).clone() });

        // Filho 3: 50px altura, margin-top = 10px
        let mut child3 = create_block_box(Some(200), Some(50));
        let mut s3 = (*child3.style.surrounding).clone();
        s3.margin.top = Some(LayoutUnit::from_px(10));
        child3.style = Arc::new(ComputedStyle { surrounding: Arc::new(s3), ..(*child3.style).clone() });

        parent.add_child(child1);
        parent.add_child(child2);
        parent.add_child(child3);

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(400),
            LayoutUnit::from_px(800),
        ));

        let res = bfc.layout(&parent, &space, &measurer);

        // O colapso entre filho 1 (mb=20), filho 2 (mt=30, mb=40) e filho 3 (mt=10) é max(20, 30, 40, 10) = 40px
        // Logo, filho 3 deve começar em 50 + 40 = 90px!
        assert_eq!(res.children[2].border_box.block_start, LayoutUnit::from_px(90));
    }

    #[test]
    fn test_bfc_independent_bfc_containment() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        // Pai com overflow: hidden estabelece novo BFC independente
        let mut parent = create_block_box(Some(400), None);
        let mut b = (*parent.style.box_style).clone();
        b.overflow_x = Overflow::Hidden;
        b.overflow_y = Overflow::Hidden;
        parent.style = Arc::new(ComputedStyle { box_style: Arc::new(b), ..(*parent.style).clone() });

        // Primeiro filho com margin-top = 30px
        let mut child = create_block_box(Some(200), Some(50));
        let mut s = (*child.style.surrounding).clone();
        s.margin.top = Some(LayoutUnit::from_px(30));
        child.style = Arc::new(ComputedStyle { surrounding: Arc::new(s), ..(*child.style).clone() });
        parent.add_child(child);

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(400),
            LayoutUnit::from_px(800),
        ));

        let res = bfc.layout(&parent, &space, &measurer);

        // Como o pai é um BFC independente, a margem do primeiro filho NÃO vaza para o pai!
        // O primeiro filho começa em y = 30px
        assert_eq!(res.children[0].border_box.block_start, LayoutUnit::from_px(30));
    }
}
