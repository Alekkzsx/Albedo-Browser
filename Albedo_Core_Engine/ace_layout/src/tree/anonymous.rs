//! # Anonymous Box Generation (CSS 2.2 §9.2.1.1 & §17.2.1)
//!
//! Implements the anonymous block box wrapping algorithm:
//! When a block container box contains a mixture of block-level and inline-level children,
//! contiguous inline-level runs are wrapped into anonymous block boxes.
//! Collapsible whitespace between adjacent block boxes is discarded.

use crate::tree::layout_box::{LayoutBox, LayoutBoxKind};
use ace_style::computed::{inherit_from_parent, ComputedStyle};
use ace_style::values::enums::{Display, WhiteSpace};
use std::sync::Arc;

/// Aplica o algoritmo recursivo de inserção de caixas anônimas conforme CSS 2.2 §9.2.1.1.
///
/// Se uma caixa contêiner de bloco contiver uma mistura de filhos de nível de bloco
/// e filhos de nível inline, sequências contíguas de nós de nível inline são
/// agrupadas em uma nova caixa `LayoutBoxKind::Anonymous`.
pub fn wrap_anonymous_blocks(layout_box: &mut LayoutBox) {
    // 1. Processa recursivamente todos os filhos primeiro (bottom-up)
    for child in &mut layout_box.children {
        wrap_anonymous_blocks(child);
    }

    // 2. Verifica se a caixa corrente é um contêiner de bloco
    if !is_block_container(layout_box.kind) {
        return;
    }

    // 3. Verifica se os filhos contêm uma mistura de block-level e inline-level
    let has_block_child = layout_box.children.iter().any(|c| c.is_block_level());
    let has_inline_child = layout_box.children.iter().any(|c| c.is_inline_level());

    if !has_block_child || !has_inline_child {
        // Homogêneo: ou todos são bloco (BFC) ou todos são inline (IFC) ou vazio
        return;
    }

    // 4. Mistura detectada: agrupa runs contíguos de filhos inline em caixas anônimas de bloco
    let old_children = std::mem::take(&mut layout_box.children);
    let mut new_children = Vec::with_capacity(old_children.len());
    let mut inline_run: Vec<LayoutBox> = Vec::new();

    let parent_style = Arc::clone(&layout_box.style);
    let parent_id = layout_box.id;

    let flush_run = |run: &mut Vec<LayoutBox>, out: &mut Vec<LayoutBox>| {
        if run.is_empty() {
            return;
        }

        // Verifica se a sequência inteira consiste unicamente em espaços colapsáveis
        if is_pure_collapsible_whitespace_run(run, &parent_style) {
            run.clear();
            return;
        }

        // Sintetiza estilo para a caixa anônima herdando do pai
        let anon_style = create_anonymous_block_style(&parent_style);
        let items = std::mem::take(run);
        let anon_box = LayoutBox::new_anonymous_with_children(parent_id, anon_style, items);
        out.push(anon_box);
    };

    for child in old_children {
        if child.is_inline_level() {
            inline_run.push(child);
        } else {
            // Encontrou uma caixa de nível de bloco: fecha o run inline anterior
            flush_run(&mut inline_run, &mut new_children);
            new_children.push(child);
        }
    }

    // Fecha o último run inline, se houver
    flush_run(&mut inline_run, &mut new_children);

    layout_box.children = new_children;
    layout_box.clear_min_max_cache();
}

/// Retorna `true` se o tipo de caixa atua como contêiner de bloco segundo CSS 2.2.
#[inline]
fn is_block_container(kind: LayoutBoxKind) -> bool {
    matches!(
        kind,
        LayoutBoxKind::Block
            | LayoutBoxKind::Anonymous
            | LayoutBoxKind::TableCell
            | LayoutBoxKind::InlineBlock
    )
}

/// Cria um `ComputedStyle` para uma caixa anônima de bloco herdando do contêiner pai.
fn create_anonymous_block_style(parent_style: &ComputedStyle) -> Arc<ComputedStyle> {
    let mut anon_style = ComputedStyle::initial();
    inherit_from_parent(&mut anon_style, parent_style);

    // Ajusta o box model para bloco (`display: block`) com margens/bordas/padding zeradas
    let mut box_style = (*anon_style.box_style).clone();
    box_style.display = Display::Block;
    anon_style.box_style = Arc::new(box_style);

    Arc::new(anon_style)
}

/// Verifica se a sequência consiste exclusivamente em nós de texto com espaços em branco colapsáveis.
fn is_pure_collapsible_whitespace_run(run: &[LayoutBox], parent_style: &ComputedStyle) -> bool {
    let white_space = parent_style.text.white_space;
    let is_collapsible = matches!(white_space, WhiteSpace::Normal | WhiteSpace::NoWrap);

    if !is_collapsible {
        return false;
    }

    run.iter().all(|child| {
        if child.kind == LayoutBoxKind::TextRun {
            if let Some(text) = child.text() {
                text.chars().all(|c| c.is_whitespace())
            } else {
                true
            }
        } else {
            false
        }
    })
}

/// Normalização de estrutura de tabelas conforme CSS 2.2 §17.2.1.
///
/// Encapsula células soltas em linhas de tabela anônimas e elementos não-linha
/// dentro de tabelas em células/linhas anônimas.
pub fn normalize_table_structure(layout_box: &mut LayoutBox) {
    for child in &mut layout_box.children {
        normalize_table_structure(child);
    }

    if layout_box.kind == LayoutBoxKind::Table {
        let old_children = std::mem::take(&mut layout_box.children);
        let mut new_children = Vec::with_capacity(old_children.len());

        let mut loose_cells: Vec<LayoutBox> = Vec::new();
        let parent_id = layout_box.id;
        let parent_style = Arc::clone(&layout_box.style);

        let flush_cells = |cells: &mut Vec<LayoutBox>, out: &mut Vec<LayoutBox>| {
            if cells.is_empty() {
                return;
            }
            let mut row_style = (*parent_style).clone();
            let mut row_box_style = (*row_style.box_style).clone();
            row_box_style.display = Display::TableRow;
            row_style.box_style = Arc::new(row_box_style);

            let row_box = LayoutBox {
                id: parent_id,
                kind: LayoutBoxKind::TableRow,
                style: Arc::new(row_style),
                text: None,
                children: std::mem::take(cells),
                min_max_cache: std::cell::Cell::new(None),
            };
            out.push(row_box);
        };

        for child in old_children {
            if child.kind == LayoutBoxKind::TableCell {
                loose_cells.push(child);
            } else {
                flush_cells(&mut loose_cells, &mut new_children);
                new_children.push(child);
            }
        }
        flush_cells(&mut loose_cells, &mut new_children);

        layout_box.children = new_children;
        layout_box.clear_min_max_cache();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol_str::SmolStr;

    fn node_id(val: u64) -> ace_core::id::NodeId {
        ace_core::id::NodeId::from_raw(val).unwrap()
    }

    #[test]
    fn test_anonymous_wrapping_mixed_children() {
        let style = Arc::new(ComputedStyle::initial());
        let mut root = LayoutBox::new(node_id(1), LayoutBoxKind::Block, Arc::clone(&style));

        let inline1 = LayoutBox::new_text(node_id(2), Arc::clone(&style), SmolStr::new("hello"));
        let block1 = LayoutBox::new(node_id(3), LayoutBoxKind::Block, Arc::clone(&style));
        let inline2 = LayoutBox::new_text(node_id(4), Arc::clone(&style), SmolStr::new("world"));

        root.add_child(inline1);
        root.add_child(block1);
        root.add_child(inline2);

        wrap_anonymous_blocks(&mut root);

        // Deve resultar em: [Anonymous([inline1]), block1, Anonymous([inline2])]
        assert_eq!(root.children.len(), 3);
        assert_eq!(root.children[0].kind, LayoutBoxKind::Anonymous);
        assert_eq!(root.children[0].children.len(), 1);
        assert_eq!(root.children[0].children[0].text(), Some("hello"));

        assert_eq!(root.children[1].kind, LayoutBoxKind::Block);

        assert_eq!(root.children[2].kind, LayoutBoxKind::Anonymous);
        assert_eq!(root.children[2].children.len(), 1);
        assert_eq!(root.children[2].children[0].text(), Some("world"));
    }

    #[test]
    fn test_anonymous_wrapping_pure_inline_unchanged() {
        let style = Arc::new(ComputedStyle::initial());
        let mut root = LayoutBox::new(node_id(1), LayoutBoxKind::Block, Arc::clone(&style));

        let inline1 = LayoutBox::new_text(node_id(2), Arc::clone(&style), SmolStr::new("foo"));
        let inline2 = LayoutBox::new_text(node_id(3), Arc::clone(&style), SmolStr::new("bar"));

        root.add_child(inline1);
        root.add_child(inline2);

        wrap_anonymous_blocks(&mut root);

        // Filhos puramente inline permanecem inalterados (IFC)
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].kind, LayoutBoxKind::TextRun);
        assert_eq!(root.children[1].kind, LayoutBoxKind::TextRun);
    }

    #[test]
    fn test_anonymous_wrapping_pure_block_unchanged() {
        let style = Arc::new(ComputedStyle::initial());
        let mut root = LayoutBox::new(node_id(1), LayoutBoxKind::Block, Arc::clone(&style));

        let b1 = LayoutBox::new(node_id(2), LayoutBoxKind::Block, Arc::clone(&style));
        let b2 = LayoutBox::new(node_id(3), LayoutBoxKind::Block, Arc::clone(&style));

        root.add_child(b1);
        root.add_child(b2);

        wrap_anonymous_blocks(&mut root);

        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].kind, LayoutBoxKind::Block);
        assert_eq!(root.children[1].kind, LayoutBoxKind::Block);
    }

    #[test]
    fn test_anonymous_wrapping_collapsible_whitespace_discarded() {
        let style = Arc::new(ComputedStyle::initial());
        let mut root = LayoutBox::new(node_id(1), LayoutBoxKind::Block, Arc::clone(&style));

        let b1 = LayoutBox::new(node_id(2), LayoutBoxKind::Block, Arc::clone(&style));
        let ws = LayoutBox::new_text(node_id(3), Arc::clone(&style), SmolStr::new("   \n\t  "));
        let b2 = LayoutBox::new(node_id(4), LayoutBoxKind::Block, Arc::clone(&style));

        root.add_child(b1);
        root.add_child(ws);
        root.add_child(b2);

        wrap_anonymous_blocks(&mut root);

        // O espaço colapsável entre blocos deve ser descartado
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].kind, LayoutBoxKind::Block);
        assert_eq!(root.children[1].kind, LayoutBoxKind::Block);
    }

    #[test]
    fn test_table_structure_normalization() {
        let style = Arc::new(ComputedStyle::initial());
        let mut table = LayoutBox::new(node_id(1), LayoutBoxKind::Table, Arc::clone(&style));

        let cell1 = LayoutBox::new(node_id(2), LayoutBoxKind::TableCell, Arc::clone(&style));
        let cell2 = LayoutBox::new(node_id(3), LayoutBoxKind::TableCell, Arc::clone(&style));

        table.add_child(cell1);
        table.add_child(cell2);

        normalize_table_structure(&mut table);

        assert_eq!(table.children.len(), 1);
        assert_eq!(table.children[0].kind, LayoutBoxKind::TableRow);
        assert_eq!(table.children[0].children.len(), 2);
    }

    #[test]
    fn test_anonymous_wrapping_nested_blocks() {
        let style = Arc::new(ComputedStyle::initial());
        let mut root = LayoutBox::new(node_id(1), LayoutBoxKind::Block, Arc::clone(&style));

        // Filho de bloco que tem mistura interna
        let mut nested_block = LayoutBox::new(node_id(2), LayoutBoxKind::Block, Arc::clone(&style));
        nested_block.add_child(LayoutBox::new_text(
            node_id(3),
            Arc::clone(&style),
            SmolStr::new("inner inline"),
        ));
        nested_block.add_child(LayoutBox::new(
            node_id(4),
            LayoutBoxKind::Block,
            Arc::clone(&style),
        ));

        // Raiz também tem mistura
        root.add_child(LayoutBox::new_text(
            node_id(5),
            Arc::clone(&style),
            SmolStr::new("root inline"),
        ));
        root.add_child(nested_block);

        wrap_anonymous_blocks(&mut root);

        // Raiz: [Anon([root inline]), nested_block]
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].kind, LayoutBoxKind::Anonymous);
        assert_eq!(root.children[1].kind, LayoutBoxKind::Block);

        // Nested block: [Anon([inner inline]), block(4)]
        let inner = &root.children[1];
        assert_eq!(inner.children.len(), 2);
        assert_eq!(inner.children[0].kind, LayoutBoxKind::Anonymous);
        assert_eq!(inner.children[1].kind, LayoutBoxKind::Block);
    }

    #[test]
    fn test_anonymous_wrapping_preserved_whitespace() {
        let mut style_pre = ComputedStyle::initial();
        let mut text_style = (*style_pre.text).clone();
        text_style.white_space = WhiteSpace::Pre;
        style_pre.text = Arc::new(text_style);
        let style_pre = Arc::new(style_pre);

        let mut root = LayoutBox::new(node_id(1), LayoutBoxKind::Block, Arc::clone(&style_pre));
        root.add_child(LayoutBox::new(
            node_id(2),
            LayoutBoxKind::Block,
            Arc::clone(&style_pre),
        ));
        root.add_child(LayoutBox::new_text(
            node_id(3),
            Arc::clone(&style_pre),
            SmolStr::new("   \n   "),
        ));
        root.add_child(LayoutBox::new(
            node_id(4),
            LayoutBoxKind::Block,
            Arc::clone(&style_pre),
        ));

        wrap_anonymous_blocks(&mut root);

        // white-space: pre NÃO é descartado: gera caixa anônima
        assert_eq!(root.children.len(), 3);
        assert_eq!(root.children[0].kind, LayoutBoxKind::Block);
        assert_eq!(root.children[1].kind, LayoutBoxKind::Anonymous);
        assert_eq!(root.children[2].kind, LayoutBoxKind::Block);
    }

    #[test]
    fn test_anonymous_wrapping_multiple_contiguous_inlines() {
        let style = Arc::new(ComputedStyle::initial());
        let mut root = LayoutBox::new(node_id(1), LayoutBoxKind::Block, Arc::clone(&style));

        // Run inicial de 3 inlines
        root.add_child(LayoutBox::new_text(node_id(2), Arc::clone(&style), SmolStr::new("a")));
        root.add_child(LayoutBox::new(node_id(3), LayoutBoxKind::Inline, Arc::clone(&style)));
        root.add_child(LayoutBox::new_text(node_id(4), Arc::clone(&style), SmolStr::new("b")));

        // Bloco intermediário
        root.add_child(LayoutBox::new(node_id(5), LayoutBoxKind::Block, Arc::clone(&style)));

        // Run final de 2 inlines
        root.add_child(LayoutBox::new_text(node_id(6), Arc::clone(&style), SmolStr::new("c")));
        root.add_child(LayoutBox::new_text(node_id(7), Arc::clone(&style), SmolStr::new("d")));

        wrap_anonymous_blocks(&mut root);

        assert_eq!(root.children.len(), 3);
        assert_eq!(root.children[0].kind, LayoutBoxKind::Anonymous);
        assert_eq!(root.children[0].children.len(), 3);

        assert_eq!(root.children[1].kind, LayoutBoxKind::Block);

        assert_eq!(root.children[2].kind, LayoutBoxKind::Anonymous);
        assert_eq!(root.children[2].children.len(), 2);
    }
}
