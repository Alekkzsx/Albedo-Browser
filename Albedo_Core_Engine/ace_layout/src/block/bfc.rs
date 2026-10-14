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

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{Direction, LogicalRect, LogicalSides, LogicalSize};
use ace_style::values::enums::{BoxSizing, Clear, Display, Float, Overflow};

use crate::block::exclusion::{ClearType, Exclusion, ExclusionSpace, FloatType};
use crate::block::margin_strut::MarginStrut;
use crate::inline::ifc::InlineFormattingContext;
use crate::inline::line_box::LineBoxFragment;
use crate::inline::text_measurer::TextMeasurer;
use crate::tree::constraint_space::ConstraintSpace;
use crate::tree::layout_box::LayoutBox;

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
        if display == Display::InlineBlock
            || display == Display::InlineFlex
            || display == Display::InlineGrid
        {
            return true;
        }

        node.kind.establishes_independent_formatting_context()
    }

    /// Calcula a largura inline do conteúdo de um float (conforme CSS 2.2 §10.3.5 Shrink-to-Fit).
    fn compute_float_inline_size(
        child: &LayoutBox,
        containing_width: LayoutUnit,
        paddings: &LogicalSides<LayoutUnit>,
        borders: &LogicalSides<LayoutUnit>,
    ) -> LayoutUnit {
        let non_content_h = paddings
            .inline_start
            .saturating_add(paddings.inline_end)
            .saturating_add(borders.inline_start)
            .saturating_add(borders.inline_end);

        if let Some(w) = child.style.width() {
            return match child.style.box_sizing() {
                BoxSizing::BorderBox => w.saturating_sub(non_content_h).max(LayoutUnit::ZERO),
                BoxSizing::ContentBox => w,
            };
        }

        // width: auto (Shrink-to-Fit conforme CSS 2.2 §10.3.5)
        let min_max = child.compute_min_max_sizes();
        let available = containing_width.saturating_sub(non_content_h).max(LayoutUnit::ZERO);
        min_max.min_content.max(available.min(min_max.max_content))
    }

    /// Executa o layout em bloco completo para uma caixa e sua subárvore.
    pub fn layout(
        &self,
        node: &LayoutBox,
        space: &ConstraintSpace,
        measurer: &dyn TextMeasurer,
    ) -> BfcLayoutResult {
        let mut exclusion_space = ExclusionSpace::new();
        self.layout_with_exclusions(node, space, measurer, &mut exclusion_space)
    }

    /// Executa o layout em bloco completo integrando o espaço de exclusão (ExclusionSpace).
    pub fn layout_with_exclusions(
        &self,
        node: &LayoutBox,
        space: &ConstraintSpace,
        measurer: &dyn TextMeasurer,
        exclusion_space: &mut ExclusionSpace,
    ) -> BfcLayoutResult {
        let is_new_bfc = Self::establishes_independent_bfc(node) || space.is_new_fc;
        let mut local_exclusion_space = ExclusionSpace::new();
        let active_exclusion_space: &mut ExclusionSpace = if is_new_bfc {
            &mut local_exclusion_space
        } else {
            exclusion_space
        };

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
            Self::compute_inline_sizing(
                node,
                containing_width,
                &paddings,
                &borders,
                space.direction,
            );

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
            // Formata via InlineFormattingContext integrando o ExclusionSpace
            let ifc_space = ConstraintSpace::for_container(LogicalSize::new(
                inline_content_size,
                LayoutUnit::MAX,
            ))
            .with_direction(space.direction)
            .with_writing_mode(space.writing_mode);

            let ifc = InlineFormattingContext::new();
            let ifc_res = ifc.layout_children_with_exclusions(
                &node.children,
                &node.style,
                &ifc_space,
                measurer,
                active_exclusion_space,
            );

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
                // 1. Elemento flutuante de nível de bloco (CSS 2.2 §9.5.1)
                let child_float = child.style.box_style.float;
                if child_float.is_floating() {
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
                    let float_margin_left = child.style.margin_left().unwrap_or(LayoutUnit::ZERO);
                    let float_margin_right = child.style.margin_right().unwrap_or(LayoutUnit::ZERO);
                    let float_margin_top = child.style.margin_top().unwrap_or(LayoutUnit::ZERO);
                    let float_margin_bottom = child.style.margin_bottom().unwrap_or(LayoutUnit::ZERO);

                    let float_inline_content = Self::compute_float_inline_size(
                        child,
                        inline_content_size,
                        &child_paddings,
                        &child_borders,
                    );

                    let float_space = ConstraintSpace::for_container(LogicalSize::new(
                        float_inline_content,
                        LayoutUnit::MAX,
                    ))
                    .with_is_new_fc(true)
                    .with_direction(space.direction)
                    .with_writing_mode(space.writing_mode);

                    let mut float_res = self.layout(child, &float_space, measurer);

                    let float_margin_w = float_res
                        .border_box
                        .inline_size
                        .saturating_add(float_margin_left)
                        .saturating_add(float_margin_right);
                    let float_margin_h = float_res
                        .border_box
                        .block_size
                        .saturating_add(float_margin_top)
                        .saturating_add(float_margin_bottom);

                    let mut search_y = current_block_offset;
                    let float_clear = child.style.box_style.clear;
                    if float_clear != Clear::None {
                        let clear_type = match float_clear {
                            Clear::Left => ClearType::Left,
                            Clear::Right => ClearType::Right,
                            Clear::Both => ClearType::Both,
                            Clear::None => ClearType::None,
                        };
                        let clear_y = active_exclusion_space.clear_offset(clear_type);
                        search_y = search_y.max(clear_y);
                    }

                    let opp = active_exclusion_space.find_layout_opportunity(
                        search_y,
                        inline_content_size,
                        float_margin_w,
                        float_margin_h,
                    );

                    let float_kind = if child_float == Float::Left {
                        FloatType::Left
                    } else {
                        FloatType::Right
                    };

                    let (margin_x, margin_y) = match float_kind {
                        FloatType::Left => (opp.inline_start, opp.block_start),
                        FloatType::Right => (
                            opp.inline_end().saturating_sub(float_margin_w).max(opp.inline_start),
                            opp.block_start,
                        ),
                    };

                    let border_x = margin_x.saturating_add(float_margin_left);
                    let border_y = margin_y.saturating_add(float_margin_top);

                    float_res.margin_box =
                        LogicalRect::new(margin_x, margin_y, float_margin_w, float_margin_h);
                    float_res.border_box.inline_start = border_x;
                    float_res.border_box.block_start = border_y;
                    float_res.padding_box.inline_start =
                        border_x.saturating_add(child_borders.inline_start);
                    float_res.padding_box.block_start =
                        border_y.saturating_add(child_borders.block_start);
                    float_res.content_box.inline_start = float_res
                        .padding_box
                        .inline_start
                        .saturating_add(child_paddings.inline_start);
                    float_res.content_box.block_start = float_res
                        .padding_box
                        .block_start
                        .saturating_add(child_paddings.block_start);

                    float_res.margins.inline_start = float_margin_left;
                    float_res.margins.inline_end = float_margin_right;
                    float_res.margins.block_start = float_margin_top;
                    float_res.margins.block_end = float_margin_bottom;

                    active_exclusion_space.add_exclusion(Exclusion::new(
                        float_res.margin_box,
                        float_kind,
                        child.id,
                    ));

                    children_results.push(float_res);
                    continue;
                }

                let child_is_new_bfc = Self::establishes_independent_bfc(child);

                let child_margin_top = child.style.margin_top().unwrap_or(LayoutUnit::ZERO);
                let child_margin_bottom = child.style.margin_bottom().unwrap_or(LayoutUnit::ZERO);

                // 2. Resolução de Clearance (CSS 2.2 §9.5.2)
                let child_clear = child.style.box_style.clear;
                let mut has_clearance = false;
                if child_clear != Clear::None {
                    let clear_type = match child_clear {
                        Clear::Left => ClearType::Left,
                        Clear::Right => ClearType::Right,
                        Clear::Both => ClearType::Both,
                        Clear::None => ClearType::None,
                    };
                    let clear_y = active_exclusion_space.clear_offset(clear_type);
                    if clear_y > LayoutUnit::ZERO {
                        let hypothetical_y = current_block_offset
                            .saturating_add(running_strut.solve().max(child_margin_top));
                        if hypothetical_y < clear_y {
                            running_strut.reset();
                            current_block_offset = clear_y;
                            has_clearance = true;
                        }
                    }
                }

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
                    if has_clearance {
                        running_strut.reset();
                        running_strut.append(child_margin_bottom);

                        let mut child_res = self.layout_with_exclusions(
                            child,
                            &child_containing_space,
                            measurer,
                            active_exclusion_space,
                        );
                        child_res.border_box.block_start = current_block_offset;
                        child_res.border_box.block_size = LayoutUnit::ZERO;
                        child_res.margins.block_start = child_margin_top;
                        child_res.margins.block_end = child_margin_bottom;
                        children_results.push(child_res);
                    } else {
                        // Bloco vazio: colapsa margin-top e margin-bottom no strut em O(1) sem backtracking
                        running_strut.append(child_margin_top);
                        running_strut.append(child_margin_bottom);

                        let mut child_res = self.layout_with_exclusions(
                            child,
                            &child_containing_space,
                            measurer,
                            active_exclusion_space,
                        );
                        child_res.border_box.block_start = current_block_offset;
                        child_res.border_box.block_size = LayoutUnit::ZERO;
                        child_res.margins.block_start = child_margin_top;
                        child_res.margins.block_end = child_margin_bottom;
                        children_results.push(child_res);
                    }
                    continue;
                }

                if child_is_new_bfc {
                    let child_space = child_containing_space.clone().with_is_new_fc(true);
                    if has_clearance {
                        let mut child_res = self.layout(child, &child_space, measurer);
                        child_res.border_box.block_start = current_block_offset;
                        child_res.margins.block_start = child_margin_top;

                        current_block_offset =
                            current_block_offset.saturating_add(child_res.border_box.block_size);
                        running_strut.append(child_margin_bottom);

                        children_results.push(child_res);
                    } else {
                        // Novo BFC independente: não colapsa com o strut interno
                        running_strut.append(child_margin_top);
                        let collapsed_top = running_strut.solve();
                        running_strut.reset();

                        current_block_offset = current_block_offset.saturating_add(collapsed_top);

                        let mut child_res = self.layout(child, &child_space, measurer);
                        child_res.border_box.block_start = current_block_offset;
                        child_res.margins.block_start = collapsed_top;

                        current_block_offset =
                            current_block_offset.saturating_add(child_res.border_box.block_size);
                        running_strut.append(child_margin_bottom);

                        children_results.push(child_res);
                    }
                    continue;
                }

                // Filho em fluxo normal
                if has_clearance {
                    let mut child_res = self.layout_with_exclusions(
                        child,
                        &child_containing_space,
                        measurer,
                        active_exclusion_space,
                    );
                    child_res.border_box.block_start = current_block_offset;
                    child_res.margins.block_start = child_margin_top;

                    current_block_offset =
                        current_block_offset.saturating_add(child_res.border_box.block_size);

                    running_strut.reset();
                    running_strut.append(child_margin_bottom);
                    running_strut.append_strut(&child_res.end_margin_strut);

                    children_results.push(child_res);
                } else {
                    running_strut.append(child_margin_top);

                    // Colapso pai/primeiro filho
                    if child_idx == 0 && parent_can_collapse_first_child {
                        // Margem do primeiro filho vaza/colapsa com o pai (não adiciona offset)
                        let mut child_res = self.layout_with_exclusions(
                            child,
                            &child_containing_space,
                            measurer,
                            active_exclusion_space,
                        );
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

                        let mut child_res = self.layout_with_exclusions(
                            child,
                            &child_containing_space,
                            measurer,
                            active_exclusion_space,
                        );
                        child_res.border_box.block_start = current_block_offset;
                        child_res.margins.block_start = collapsed_top;

                        current_block_offset =
                            current_block_offset.saturating_add(child_res.border_box.block_size);

                        running_strut.append(child_margin_bottom);
                        running_strut.append_strut(&child_res.end_margin_strut);

                        children_results.push(child_res);
                    }
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

        // Expansão de altura em BFCs independentes para conter floats (CSS 2.2 §10.6.7)
        if is_new_bfc && !active_exclusion_space.is_empty() {
            let max_float_bottom = active_exclusion_space.clear_offset(ClearType::Both);
            content_block_size = content_block_size.max(max_float_bottom);
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
            padding_box
                .inline_start
                .saturating_add(paddings.inline_start),
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
    use crate::tree::layout_box::LayoutBoxKind;
    use ace_style::computed::ComputedStyle;
    use std::sync::Arc;

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
        child1.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s1),
            ..(*child1.style).clone()
        });

        // Filho 2: 50px de altura, margin-top = 20px (menor), margin-bottom = 10px
        let mut child2 = create_block_box(Some(200), Some(50));
        let mut s2 = (*child2.style.surrounding).clone();
        s2.margin.top = Some(LayoutUnit::from_px(20));
        s2.margin.bottom = Some(LayoutUnit::from_px(10));
        child2.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s2),
            ..(*child2.style).clone()
        });

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
        assert_eq!(
            res.children[1].border_box.block_start,
            LayoutUnit::from_px(80)
        );
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
        child1.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s1),
            ..(*child1.style).clone()
        });

        // Filho 2 (vazio): height = auto, sem filhos, margin-top = 30px, margin-bottom = 40px
        let mut child2 = create_block_box(None, None);
        let mut s2 = (*child2.style.surrounding).clone();
        s2.margin.top = Some(LayoutUnit::from_px(30));
        s2.margin.bottom = Some(LayoutUnit::from_px(40));
        child2.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s2),
            ..(*child2.style).clone()
        });

        // Filho 3: 50px altura, margin-top = 10px
        let mut child3 = create_block_box(Some(200), Some(50));
        let mut s3 = (*child3.style.surrounding).clone();
        s3.margin.top = Some(LayoutUnit::from_px(10));
        child3.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s3),
            ..(*child3.style).clone()
        });

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
        assert_eq!(
            res.children[2].border_box.block_start,
            LayoutUnit::from_px(90)
        );
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
        parent.style = Arc::new(ComputedStyle {
            box_style: Arc::new(b),
            ..(*parent.style).clone()
        });

        // Primeiro filho com margin-top = 30px
        let mut child = create_block_box(Some(200), Some(50));
        let mut s = (*child.style.surrounding).clone();
        s.margin.top = Some(LayoutUnit::from_px(30));
        child.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s),
            ..(*child.style).clone()
        });
        parent.add_child(child);

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(400),
            LayoutUnit::from_px(800),
        ));

        let res = bfc.layout(&parent, &space, &measurer);

        // Como o pai é um BFC independente, a margem do primeiro filho NÃO vaza para o pai!
        // O primeiro filho começa em y = 30px
        assert_eq!(
            res.children[0].border_box.block_start,
            LayoutUnit::from_px(30)
        );
    }

    #[test]
    fn test_bfc_parent_first_child_margin_collapsing() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        // Pai sem border-top e sem padding-top
        let mut parent = create_block_box(Some(400), None);

        // Primeiro filho com margin-top = 30px
        let mut child = create_block_box(Some(200), Some(50));
        let mut s = (*child.style.surrounding).clone();
        s.margin.top = Some(LayoutUnit::from_px(30));
        child.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s),
            ..(*child.style).clone()
        });
        parent.add_child(child);

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(400),
            LayoutUnit::from_px(800),
        ));

        let res = bfc.layout(&parent, &space, &measurer);

        // Primeiro filho colapsa com o pai: começa em y = 0
        assert_eq!(res.children[0].border_box.block_start, LayoutUnit::ZERO);
    }

    #[test]
    fn test_bfc_parent_last_child_margin_collapsing() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        // Pai com height: auto, border-bottom = 0, padding-bottom = 0
        let mut parent = create_block_box(Some(400), None);

        // Filho com 50px de altura e margin-bottom = 45px
        let mut child = create_block_box(Some(200), Some(50));
        let mut s = (*child.style.surrounding).clone();
        s.margin.bottom = Some(LayoutUnit::from_px(45));
        child.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s),
            ..(*child.style).clone()
        });
        parent.add_child(child);

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(400),
            LayoutUnit::from_px(800),
        ));

        let res = bfc.layout(&parent, &space, &measurer);

        // Altura do pai não inclui os 45px do filho
        assert_eq!(res.height(), LayoutUnit::from_px(50));
        // A margem inferior do filho vaza pelo end_margin_strut do pai
        assert_eq!(res.end_margin_strut.solve(), LayoutUnit::from_px(45));
    }

    #[test]
    fn test_bfc_negative_mixed_margin_collapsing() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        let mut parent = create_block_box(Some(400), None);

        // Sibling 1: 50px altura, margin-bottom = 30px
        let mut child1 = create_block_box(Some(200), Some(50));
        let mut s1 = (*child1.style.surrounding).clone();
        s1.margin.bottom = Some(LayoutUnit::from_px(30));
        child1.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s1),
            ..(*child1.style).clone()
        });

        // Sibling 2: 50px altura, margin-top = -10px
        let mut child2 = create_block_box(Some(200), Some(50));
        let mut s2 = (*child2.style.surrounding).clone();
        s2.margin.top = Some(LayoutUnit::from_px(-10));
        child2.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s2),
            ..(*child2.style).clone()
        });

        parent.add_child(child1);
        parent.add_child(child2);

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(400),
            LayoutUnit::from_px(800),
        ));

        let res = bfc.layout(&parent, &space, &measurer);

        // Colapso misto: 30 + (-10) = 20px
        // Sibling 2 começa em 50 + 20 = 70px
        assert_eq!(
            res.children[1].border_box.block_start,
            LayoutUnit::from_px(70)
        );
    }

    #[test]
    fn test_bfc_min_max_width_clamping() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        // width: auto em contêiner de 500px, mas max-width = 300px
        let mut node = create_block_box(None, Some(100));
        let mut b = (*node.style.box_style).clone();
        b.max_width = Some(LayoutUnit::from_px(300));
        node.style = Arc::new(ComputedStyle {
            box_style: Arc::new(b),
            ..(*node.style).clone()
        });

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(500),
            LayoutUnit::from_px(600),
        ));

        let res = bfc.layout(&node, &space, &measurer);
        assert_eq!(res.width(), LayoutUnit::from_px(300));
    }

    #[test]
    fn test_bfc_block_level_floats_left_and_right() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        let mut parent = create_block_box(Some(400), None);

        // Float Left (100x50)
        let mut float_l = create_block_box(Some(100), Some(50));
        let mut b_l = (*float_l.style.box_style).clone();
        b_l.float = Float::Left;
        float_l.style = Arc::new(ComputedStyle {
            box_style: Arc::new(b_l),
            ..(*float_l.style).clone()
        });

        // Float Right (120x60)
        let mut float_r = create_block_box(Some(120), Some(60));
        let mut b_r = (*float_r.style.box_style).clone();
        b_r.float = Float::Right;
        float_r.style = Arc::new(ComputedStyle {
            box_style: Arc::new(b_r),
            ..(*float_r.style).clone()
        });

        // Normal block
        let normal = create_block_box(Some(400), Some(30));

        parent.add_child(float_l);
        parent.add_child(float_r);
        parent.add_child(normal);

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(400),
            LayoutUnit::from_px(600),
        ));

        let res = bfc.layout(&parent, &space, &measurer);
        assert_eq!(res.children.len(), 3);

        // Float left na borda esquerda
        assert_eq!(res.children[0].border_box.inline_start, LayoutUnit::ZERO);
        assert_eq!(res.children[0].border_box.block_start, LayoutUnit::ZERO);
        assert_eq!(res.children[0].width(), LayoutUnit::from_px(100));

        // Float right na borda direita (400 - 120 = 280)
        assert_eq!(
            res.children[1].border_box.inline_start,
            LayoutUnit::from_px(280)
        );
        assert_eq!(res.children[1].border_box.block_start, LayoutUnit::ZERO);
        assert_eq!(res.children[1].width(), LayoutUnit::from_px(120));

        // Bloco normal no fluxo (y = 0, pois floats não consomem espaço no fluxo normal de blocos)
        assert_eq!(res.children[2].border_box.block_start, LayoutUnit::ZERO);
    }

    #[test]
    fn test_bfc_clearance_barrier_and_strut_reset() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        let mut parent = create_block_box(Some(400), None);

        // Float Left de 80px de altura
        let mut float_l = create_block_box(Some(100), Some(80));
        let mut b_l = (*float_l.style.box_style).clone();
        b_l.float = Float::Left;
        float_l.style = Arc::new(ComputedStyle {
            box_style: Arc::new(b_l),
            ..(*float_l.style).clone()
        });

        // Bloco 1: altura 40px, margin-bottom 20px
        let mut child1 = create_block_box(Some(300), Some(40));
        let mut s1 = (*child1.style.surrounding).clone();
        s1.margin.bottom = Some(LayoutUnit::from_px(20));
        child1.style = Arc::new(ComputedStyle {
            surrounding: Arc::new(s1),
            ..(*child1.style).clone()
        });

        // Bloco 2: clear: left, margin-top 10px, altura 30px
        let mut child2 = create_block_box(Some(300), Some(30));
        let mut b2 = (*child2.style.box_style).clone();
        b2.clear = Clear::Left;
        let mut s2 = (*child2.style.surrounding).clone();
        s2.margin.top = Some(LayoutUnit::from_px(10));
        child2.style = Arc::new(ComputedStyle {
            box_style: Arc::new(b2),
            surrounding: Arc::new(s2),
            ..(*child2.style).clone()
        });

        parent.add_child(float_l);
        parent.add_child(child1);
        parent.add_child(child2);

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(400),
            LayoutUnit::from_px(600),
        ));

        let res = bfc.layout(&parent, &space, &measurer);
        // Bloco 1 começa em y = 0
        assert_eq!(res.children[1].border_box.block_start, LayoutUnit::ZERO);
        // Bloco 2 com clear: left é empurrado para y = 80px (base do float de 80px)
        assert_eq!(
            res.children[2].border_box.block_start,
            LayoutUnit::from_px(80)
        );
    }

    #[test]
    fn test_bfc_independent_overflow_contains_floats() {
        let bfc = BlockFormattingContext::new();
        let measurer = FixedTextMeasurer::new();

        // Contêiner com overflow: hidden estabelece BFC independente
        let mut parent = create_block_box(Some(400), None);
        let mut p_b = (*parent.style.box_style).clone();
        p_b.overflow_x = Overflow::Hidden;
        p_b.overflow_y = Overflow::Hidden;
        parent.style = Arc::new(ComputedStyle {
            box_style: Arc::new(p_b),
            ..(*parent.style).clone()
        });

        // Float de 150px de altura
        let mut float_l = create_block_box(Some(100), Some(150));
        let mut b_l = (*float_l.style.box_style).clone();
        b_l.float = Float::Left;
        float_l.style = Arc::new(ComputedStyle {
            box_style: Arc::new(b_l),
            ..(*float_l.style).clone()
        });

        parent.add_child(float_l);

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(400),
            LayoutUnit::from_px(600),
        ));

        let res = bfc.layout(&parent, &space, &measurer);
        // Altura do pai expande para conter o float (150px)
        assert_eq!(res.height(), LayoutUnit::from_px(150));
    }
}
