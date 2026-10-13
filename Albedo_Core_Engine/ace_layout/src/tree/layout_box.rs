//! # Layout Box Data Structure (`LayoutBox`)
//!
//! Representation of semantic layout boxes in the Box Tree, bridging DOM and styles
//! to the formatting context algorithms.

use crate::tree::min_max::MinMaxSizes;
use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_style::computed::ComputedStyle;
use ace_style::values::enums::BoxSizing;
use smol_str::SmolStr;
use std::cell::Cell;
use std::sync::Arc;

/// Categorias normativas de caixas de layout no Box Tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutBoxKind {
    /// Caixa de contêiner de bloco (BFC).
    Block,
    /// Caixa inline tradicional participante de IFC.
    Inline,
    /// Elemento atômico inline-block: estabelece BFC internamente, comporta-se como inline externamente.
    InlineBlock,
    /// Contêiner Flexbox estabelecendo FFC.
    Flex,
    /// Contêiner Grid estabelecendo GFC.
    Grid,
    /// Contêiner de tabela estabelecendo TFC.
    Table,
    /// Linha de tabela (`<tr>`).
    TableRow,
    /// Célula de tabela (`<td>`, `<th>`): estabelece BFC independente internamente.
    TableCell,
    /// Caixa de bloco anônima sintetizada pelo motor (CSS 2.2 §9.2.1.1).
    Anonymous,
    /// Folha contendo sequência contígua de texto para formatação em linha.
    TextRun,
}

impl LayoutBoxKind {
    /// Retorna `true` se a caixa for de nível de bloco no fluxo normal.
    #[inline]
    pub fn is_block_level(&self) -> bool {
        matches!(
            self,
            Self::Block | Self::Flex | Self::Grid | Self::Table | Self::Anonymous
        )
    }

    /// Retorna `true` se a caixa for de nível inline participante de IFC.
    #[inline]
    pub fn is_inline_level(&self) -> bool {
        matches!(self, Self::Inline | Self::InlineBlock | Self::TextRun)
    }

    /// Retorna `true` se for uma folha de texto (`TextRun`).
    #[inline]
    pub fn is_text_run(&self) -> bool {
        matches!(self, Self::TextRun)
    }

    /// Retorna `true` se for uma caixa anônima sintetizada pelo motor.
    #[inline]
    pub fn is_anonymous(&self) -> bool {
        matches!(self, Self::Anonymous)
    }

    /// Retorna `true` se for elemento estrutural interno de tabela.
    #[inline]
    pub fn is_table_part(&self) -> bool {
        matches!(self, Self::TableRow | Self::TableCell)
    }

    /// Retorna `true` se a caixa estabelece um novo contexto de formatação independente.
    #[inline]
    pub fn establishes_independent_formatting_context(&self) -> bool {
        matches!(
            self,
            Self::InlineBlock | Self::Flex | Self::Grid | Self::Table | Self::TableCell
        )
    }
}

/// Nó da árvore de caixas de layout (Box Tree).
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutBox {
    /// Identificador do nó DOM de origem (ou ID do contêiner para caixas anônimas).
    pub id: NodeId,
    /// Categoria normativa de caixa de layout.
    pub kind: LayoutBoxKind,
    /// Estilo computado imutável associado.
    pub style: Arc<ComputedStyle>,
    /// Conteúdo textual caso a caixa seja um `TextRun`.
    pub text: Option<SmolStr>,
    /// Subcaixas filhas no Box Tree.
    pub children: Vec<LayoutBox>,
    /// Cache de medição intrínseca `MinMaxSizes`.
    pub min_max_cache: Cell<Option<MinMaxSizes>>,
}

impl LayoutBox {
    /// Cria uma nova caixa de layout com filhos vazios.
    pub fn new(id: NodeId, kind: LayoutBoxKind, style: Arc<ComputedStyle>) -> Self {
        Self {
            id,
            kind,
            style,
            text: None,
            children: Vec::new(),
            min_max_cache: Cell::new(None),
        }
    }

    /// Cria uma nova caixa de execução de texto (`TextRun`).
    pub fn new_text(id: NodeId, style: Arc<ComputedStyle>, text: SmolStr) -> Self {
        Self {
            id,
            kind: LayoutBoxKind::TextRun,
            style,
            text: Some(text),
            children: Vec::new(),
            min_max_cache: Cell::new(None),
        }
    }

    /// Cria uma caixa de bloco anônima sintetizada pelo motor.
    pub fn new_anonymous(id: NodeId, style: Arc<ComputedStyle>) -> Self {
        Self {
            id,
            kind: LayoutBoxKind::Anonymous,
            style,
            text: None,
            children: Vec::new(),
            min_max_cache: Cell::new(None),
        }
    }

    /// Cria uma caixa de bloco anônima sintetizada com filhos preexistentes.
    pub fn new_anonymous_with_children(
        id: NodeId,
        style: Arc<ComputedStyle>,
        children: Vec<LayoutBox>,
    ) -> Self {
        Self {
            id,
            kind: LayoutBoxKind::Anonymous,
            style,
            text: None,
            children,
            min_max_cache: Cell::new(None),
        }
    }

    /// Adiciona uma caixa filha ao final da lista de filhos.
    pub fn add_child(&mut self, child: LayoutBox) {
        self.children.push(child);
        self.clear_min_max_cache();
    }

    /// Retorna referência ao estilo computado.
    #[inline]
    pub fn style(&self) -> &Arc<ComputedStyle> {
        &self.style
    }

    /// Retorna a categoria normativa de caixa.
    #[inline]
    pub fn box_kind(&self) -> LayoutBoxKind {
        self.kind
    }

    /// Alias para `box_kind`.
    #[inline]
    pub fn kind(&self) -> LayoutBoxKind {
        self.kind
    }

    /// Retorna fatia imutável dos filhos da caixa.
    #[inline]
    pub fn children(&self) -> &[LayoutBox] {
        &self.children
    }

    /// Retorna fatia mutável dos filhos da caixa.
    #[inline]
    pub fn children_mut(&mut self) -> &mut Vec<LayoutBox> {
        self.clear_min_max_cache();
        &mut self.children
    }

    /// Retorna `true` se a caixa for de nível de bloco.
    #[inline]
    pub fn is_block_level(&self) -> bool {
        self.kind.is_block_level()
    }

    /// Retorna `true` se a caixa for de nível inline.
    #[inline]
    pub fn is_inline_level(&self) -> bool {
        self.kind.is_inline_level()
    }

    /// Retorna `true` se a caixa for uma folha de texto.
    #[inline]
    pub fn is_text_run(&self) -> bool {
        self.kind.is_text_run()
    }

    /// Retorna `true` se a caixa for anônima.
    #[inline]
    pub fn is_anonymous(&self) -> bool {
        self.kind.is_anonymous()
    }

    /// Retorna o texto se presente como `&str`.
    #[inline]
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Retorna o texto se presente como `&SmolStr`.
    #[inline]
    pub fn text_content(&self) -> Option<&SmolStr> {
        self.text.as_ref()
    }

    /// Obtém o valor em cache de `MinMaxSizes`, se existir.
    #[inline]
    pub fn min_max_cache(&self) -> Option<MinMaxSizes> {
        self.min_max_cache.get()
    }

    /// Salva o valor de `MinMaxSizes` no cache interno.
    #[inline]
    pub fn set_min_max_cache(&self, sizes: MinMaxSizes) {
        self.min_max_cache.set(Some(sizes));
    }

    /// Limpa o cache de medição intrínseca.
    #[inline]
    pub fn clear_min_max_cache(&self) {
        self.min_max_cache.set(None);
    }

    /// Conta o número total de descendentes (recursivo).
    pub fn count_descendants(&self) -> usize {
        let mut count = self.children.len();
        for child in &self.children {
            count += child.count_descendants();
        }
        count
    }

    /// Computa ou recupera do cache os tamanhos intrínsecos `MinMaxSizes` da caixa.
    pub fn compute_min_max_sizes(&self) -> MinMaxSizes {
        if let Some(cached) = self.min_max_cache.get() {
            return cached;
        }

        let horizontal_surrounding = self.horizontal_padding_and_border();
        let mut sizes = match self.kind {
            LayoutBoxKind::TextRun => {
                let text_str = self.text.as_deref().unwrap_or("");
                let font_size = self.style.font.size;
                // Estimativa tipográfica padrão: caractere médio ocupa ~50% do font-size
                let char_width = font_size.saturating_div(2).max(LayoutUnit::from_px(1));

                // min_content: largura da palavra mais longa (sem quebra)
                let max_word_len = text_str
                    .split_whitespace()
                    .map(|w| w.chars().count())
                    .max()
                    .unwrap_or(0);
                let min_w = char_width.saturating_mul(max_word_len as i32);

                // max_content: linha inteira contínua sem quebras
                let total_chars = text_str.chars().count();
                let max_w = char_width.saturating_mul(total_chars as i32);

                MinMaxSizes::new(min_w, max_w, false)
            }
            LayoutBoxKind::Inline => {
                // IFC: min_content é o maior min_content dos filhos; max_content é a soma
                let mut min_val = LayoutUnit::ZERO;
                let mut max_val = LayoutUnit::ZERO;
                let mut depends_pct = false;

                for child in &self.children {
                    let child_sizes = child.compute_min_max_sizes();
                    min_val = min_val.max(child_sizes.min_content);
                    max_val = max_val.saturating_add(child_sizes.max_content);
                    depends_pct |= child_sizes.depends_on_percentage;
                }

                MinMaxSizes::new(min_val, max_val, depends_pct) + horizontal_surrounding
            }
            LayoutBoxKind::Block | LayoutBoxKind::Anonymous | LayoutBoxKind::InlineBlock => {
                // BFC: min_content é max(filhos min); max_content é max(filhos max)
                let mut min_val = LayoutUnit::ZERO;
                let mut max_val = LayoutUnit::ZERO;
                let mut depends_pct = false;

                for child in &self.children {
                    let child_sizes = child.compute_min_max_sizes();
                    min_val = min_val.max(child_sizes.min_content);
                    max_val = max_val.max(child_sizes.max_content);
                    depends_pct |= child_sizes.depends_on_percentage;
                }

                MinMaxSizes::new(min_val, max_val, depends_pct) + horizontal_surrounding
            }
            LayoutBoxKind::Flex => {
                // Flex: assumindo row nowrap por padrão para tamanhos intrínsecos
                let mut min_val = LayoutUnit::ZERO;
                let mut max_val = LayoutUnit::ZERO;
                let mut depends_pct = false;

                for child in &self.children {
                    let child_sizes = child.compute_min_max_sizes();
                    min_val = min_val.saturating_add(child_sizes.min_content);
                    max_val = max_val.saturating_add(child_sizes.max_content);
                    depends_pct |= child_sizes.depends_on_percentage;
                }

                MinMaxSizes::new(min_val, max_val, depends_pct) + horizontal_surrounding
            }
            LayoutBoxKind::Grid | LayoutBoxKind::Table => {
                let mut min_val = LayoutUnit::ZERO;
                let mut max_val = LayoutUnit::ZERO;
                let mut depends_pct = false;

                for child in &self.children {
                    let child_sizes = child.compute_min_max_sizes();
                    min_val = min_val.max(child_sizes.min_content);
                    max_val = max_val.max(child_sizes.max_content);
                    depends_pct |= child_sizes.depends_on_percentage;
                }

                MinMaxSizes::new(min_val, max_val, depends_pct) + horizontal_surrounding
            }
            LayoutBoxKind::TableRow => {
                // Linha de tabela acumula células horizontalmente
                let mut min_val = LayoutUnit::ZERO;
                let mut max_val = LayoutUnit::ZERO;
                let mut depends_pct = false;

                for child in &self.children {
                    let child_sizes = child.compute_min_max_sizes();
                    min_val = min_val.saturating_add(child_sizes.min_content);
                    max_val = max_val.saturating_add(child_sizes.max_content);
                    depends_pct |= child_sizes.depends_on_percentage;
                }

                MinMaxSizes::new(min_val, max_val, depends_pct) + horizontal_surrounding
            }
            LayoutBoxKind::TableCell => {
                let mut min_val = LayoutUnit::ZERO;
                let mut max_val = LayoutUnit::ZERO;
                let mut depends_pct = false;

                for child in &self.children {
                    let child_sizes = child.compute_min_max_sizes();
                    min_val = min_val.max(child_sizes.min_content);
                    max_val = max_val.max(child_sizes.max_content);
                    depends_pct |= child_sizes.depends_on_percentage;
                }

                MinMaxSizes::new(min_val, max_val, depends_pct) + horizontal_surrounding
            }
        };

        // Aplicação de restrições explícitas de width / min-width / max-width do CSS
        if let Some(explicit_width) = self.style.width() {
            let effective = match self.style.box_sizing() {
                BoxSizing::BorderBox => explicit_width,
                BoxSizing::ContentBox => explicit_width.saturating_add(horizontal_surrounding),
            };
            sizes.min_content = effective;
            sizes.max_content = effective;
        }

        if let Some(min_w) = self.style.box_style.min_width {
            let effective_min = match self.style.box_sizing() {
                BoxSizing::BorderBox => min_w,
                BoxSizing::ContentBox => min_w.saturating_add(horizontal_surrounding),
            };
            sizes.min_content = sizes.min_content.max(effective_min);
            sizes.max_content = sizes.max_content.max(effective_min);
        }

        if let Some(max_w) = self.style.box_style.max_width {
            let effective_max = match self.style.box_sizing() {
                BoxSizing::BorderBox => max_w,
                BoxSizing::ContentBox => max_w.saturating_add(horizontal_surrounding),
            };
            sizes.min_content = sizes.min_content.min(effective_max);
            sizes.max_content = sizes.max_content.min(effective_max);
        }

        sizes.enforce_invariants();
        self.min_max_cache.set(Some(sizes));
        sizes
    }

    /// Soma de padding horizontal e border horizontal.
    #[inline]
    fn horizontal_padding_and_border(&self) -> LayoutUnit {
        self.style
            .surrounding
            .padding
            .left
            .saturating_add(self.style.surrounding.padding.right)
            .saturating_add(self.style.surrounding.border_width.left)
            .saturating_add(self.style.surrounding.border_width.right)
    }

    /// Gera uma representação textual hierárquica para depuração e testes.
    pub fn dump_tree(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let mut out = format!("{pad}{:?} (id={:?})", self.kind, self.id.raw());
        if let Some(ref text) = self.text {
            out.push_str(&format!(" {:?}", text.as_str()));
        }
        out.push('\n');
        for child in &self.children {
            out.push_str(&child.dump_tree(indent + 1));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node_id(val: u64) -> NodeId {
        NodeId::from_raw(val).unwrap()
    }

    #[test]
    fn test_layout_box_creation_and_hierarchy() {
        let style = Arc::new(ComputedStyle::initial());
        let mut root = LayoutBox::new(test_node_id(1), LayoutBoxKind::Block, Arc::clone(&style));
        assert_eq!(root.kind(), LayoutBoxKind::Block);
        assert!(root.is_block_level());
        assert!(!root.is_inline_level());

        let child1 = LayoutBox::new(test_node_id(2), LayoutBoxKind::Inline, Arc::clone(&style));
        let child2 = LayoutBox::new_text(
            test_node_id(3),
            Arc::clone(&style),
            SmolStr::new("hello world"),
        );

        root.add_child(child1);
        root.add_child(child2);

        assert_eq!(root.children().len(), 2);
        assert_eq!(root.count_descendants(), 2);
        assert_eq!(root.children()[1].text(), Some("hello world"));
    }

    #[test]
    fn test_layout_box_min_max_computation_and_cache() {
        let style = Arc::new(ComputedStyle::initial());
        let text_box = LayoutBox::new_text(
            test_node_id(1),
            Arc::clone(&style),
            SmolStr::new("abc defgh"),
        );

        let sizes = text_box.compute_min_max_sizes();
        assert!(sizes.min_content > LayoutUnit::ZERO);
        assert!(sizes.max_content >= sizes.min_content);
        assert_eq!(text_box.min_max_cache(), Some(sizes));

        // Subsequent call returns cached
        let cached = text_box.compute_min_max_sizes();
        assert_eq!(cached, sizes);
    }

    #[test]
    fn test_layout_box_kind_predicates() {
        assert!(LayoutBoxKind::Block.is_block_level());
        assert!(LayoutBoxKind::Anonymous.is_block_level());
        assert!(LayoutBoxKind::Flex.is_block_level());
        assert!(LayoutBoxKind::Grid.is_block_level());
        assert!(LayoutBoxKind::Table.is_block_level());

        assert!(LayoutBoxKind::Inline.is_inline_level());
        assert!(LayoutBoxKind::InlineBlock.is_inline_level());
        assert!(LayoutBoxKind::TextRun.is_inline_level());

        assert!(LayoutBoxKind::TextRun.is_text_run());
        assert!(!LayoutBoxKind::Block.is_text_run());

        assert!(LayoutBoxKind::Anonymous.is_anonymous());
        assert!(!LayoutBoxKind::Block.is_anonymous());

        assert!(LayoutBoxKind::TableRow.is_table_part());
        assert!(LayoutBoxKind::TableCell.is_table_part());

        assert!(LayoutBoxKind::InlineBlock.establishes_independent_formatting_context());
        assert!(LayoutBoxKind::Flex.establishes_independent_formatting_context());
        assert!(LayoutBoxKind::Grid.establishes_independent_formatting_context());
        assert!(LayoutBoxKind::TableCell.establishes_independent_formatting_context());
        assert!(!LayoutBoxKind::Block.establishes_independent_formatting_context());
    }

    #[test]
    fn test_layout_box_cache_invalidation() {
        let style = Arc::new(ComputedStyle::initial());
        let mut b = LayoutBox::new(test_node_id(1), LayoutBoxKind::Block, Arc::clone(&style));
        b.set_min_max_cache(MinMaxSizes::from_single(LayoutUnit::from_px(100)));
        assert!(b.min_max_cache().is_some());

        // add_child clears cache
        b.add_child(LayoutBox::new(
            test_node_id(2),
            LayoutBoxKind::Block,
            Arc::clone(&style),
        ));
        assert!(b.min_max_cache().is_none());

        b.set_min_max_cache(MinMaxSizes::from_single(LayoutUnit::from_px(100)));
        assert!(b.min_max_cache().is_some());

        // children_mut clears cache
        let _ = b.children_mut();
        assert!(b.min_max_cache().is_none());
    }

    #[test]
    fn test_layout_box_explicit_width_and_box_sizing() {
        let mut style = ComputedStyle::initial();
        let mut box_style = (*style.box_style).clone();
        box_style.width = Some(LayoutUnit::from_px(200));
        box_style.box_sizing = BoxSizing::BorderBox;
        style.box_style = Arc::new(box_style);

        let mut surrounding = (*style.surrounding).clone();
        surrounding.padding.left = LayoutUnit::from_px(20);
        surrounding.padding.right = LayoutUnit::from_px(20);
        style.surrounding = Arc::new(surrounding);

        let b = LayoutBox::new(test_node_id(1), LayoutBoxKind::Block, Arc::new(style));
        let sizes = b.compute_min_max_sizes();

        // BorderBox: explicit width remains 200px
        assert_eq!(sizes.min_content, LayoutUnit::from_px(200));
        assert_eq!(sizes.max_content, LayoutUnit::from_px(200));

        // Now ContentBox: explicit width expands by 40px padding = 240px
        let mut style_content = ComputedStyle::initial();
        let mut box_style_c = (*style_content.box_style).clone();
        box_style_c.width = Some(LayoutUnit::from_px(200));
        box_style_c.box_sizing = BoxSizing::ContentBox;
        style_content.box_style = Arc::new(box_style_c);

        let mut surrounding_c = (*style_content.surrounding).clone();
        surrounding_c.padding.left = LayoutUnit::from_px(20);
        surrounding_c.padding.right = LayoutUnit::from_px(20);
        style_content.surrounding = Arc::new(surrounding_c);

        let b_content =
            LayoutBox::new(test_node_id(2), LayoutBoxKind::Block, Arc::new(style_content));
        let sizes_c = b_content.compute_min_max_sizes();
        assert_eq!(sizes_c.min_content, LayoutUnit::from_px(240));
        assert_eq!(sizes_c.max_content, LayoutUnit::from_px(240));
    }

    #[test]
    fn test_layout_box_dump_tree() {
        let style = Arc::new(ComputedStyle::initial());
        let mut root = LayoutBox::new(test_node_id(1), LayoutBoxKind::Block, Arc::clone(&style));
        root.add_child(LayoutBox::new_text(
            test_node_id(2),
            Arc::clone(&style),
            SmolStr::new("hello"),
        ));

        let dump = root.dump_tree(0);
        assert!(dump.contains("Block (id=1)"));
        assert!(dump.contains("TextRun (id=2) \"hello\""));
    }
}
