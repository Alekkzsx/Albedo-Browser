//! # Box Tree Construction (`BoxTree` and `BoxTreeBuilder`)
//!
//! Transforms the semantic and styled `RenderTree` from `ace_style` together with
//! the DOM `Document` from `ace_dom` into a complete `BoxTree` of `LayoutBox` nodes,
//! applying normative CSS 2.2 anonymous box wrapping.

use crate::tree::anonymous::wrap_anonymous_blocks;
use crate::tree::layout_box::{LayoutBox, LayoutBoxKind};
use ace_dom::Document;
use ace_style::engine::{RenderNodeKind, RenderTree, RenderTreeNode};
use ace_style::values::enums::Display;
use smol_str::SmolStr;
use std::sync::Arc;

/// A árvore de caixas de layout (Box Tree), representando a estrutura semântica
/// de caixas antes da resolução de geometria e quebra de linhas.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxTree {
    /// A raiz da árvore de caixas (normalmente o elemento HTML/raiz visível).
    pub root: Option<LayoutBox>,
}

impl BoxTree {
    /// Cria uma nova `BoxTree` com a raiz fornecida.
    #[inline]
    pub fn new(root: Option<LayoutBox>) -> Self {
        Self { root }
    }

    /// Cria uma `BoxTree` vazia.
    #[inline]
    pub fn empty() -> Self {
        Self { root: None }
    }

    /// Retorna referência imutável à raiz.
    #[inline]
    pub fn root(&self) -> Option<&LayoutBox> {
        self.root.as_ref()
    }

    /// Retorna referência mutável à raiz.
    #[inline]
    pub fn root_mut(&mut self) -> Option<&mut LayoutBox> {
        self.root.as_mut()
    }

    /// Retorna `true` se a árvore de caixas estiver vazia.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Retorna o número total de caixas de layout presentes na árvore.
    pub fn box_count(&self) -> usize {
        match &self.root {
            Some(r) => 1 + r.count_descendants(),
            None => 0,
        }
    }

    /// Produz um dump textual para depuração e inspeção em testes.
    pub fn dump(&self) -> String {
        match &self.root {
            Some(r) => r.dump_tree(0),
            None => "<Empty BoxTree>\n".to_string(),
        }
    }
}

/// Construtor responsável por transformar `RenderTree` e `Document` na `BoxTree`.
pub struct BoxTreeBuilder<'a> {
    document: &'a Document,
}

impl<'a> BoxTreeBuilder<'a> {
    /// Inicializa o construtor com o documento DOM de suporte.
    #[inline]
    pub fn new(document: &'a Document) -> Self {
        Self { document }
    }

    /// Constrói a `BoxTree` completa a partir da `RenderTree`.
    pub fn build(&self, render_tree: &RenderTree) -> BoxTree {
        let Some(ref render_root) = render_tree.root else {
            return BoxTree::empty();
        };

        let Some(mut root_box) = self.build_box_recursive(render_root) else {
            return BoxTree::empty();
        };

        // Aplica o algoritmo normativo CSS 2.2 §9.2.1.1 de caixas anônimas
        wrap_anonymous_blocks(&mut root_box);

        BoxTree::new(Some(root_box))
    }

    /// Converte recursivamente um `RenderTreeNode` em uma hierarquia de `LayoutBox`.
    fn build_box_recursive(&self, render_node: &RenderTreeNode) -> Option<LayoutBox> {
        // Nós com `display: none` não geram caixas de layout
        if render_node.style.display() == Display::None {
            return None;
        }

        let dom_node = self.document.get_node(render_node.node_id);

        // Identifica se o nó corresponde a um nó de texto
        let is_text = render_node.kind == RenderNodeKind::Text
            || dom_node.map(|n| n.is_text()).unwrap_or(false);

        if is_text {
            let text_str = if let Some(dom) = dom_node {
                dom.text_content().unwrap_or("")
            } else if let Some(ref pseudo_content) = render_node.style.content {
                pseudo_content.as_str()
            } else {
                ""
            };

            return Some(LayoutBox::new_text(
                render_node.node_id,
                Arc::clone(&render_node.style),
                SmolStr::new(text_str),
            ));
        }

        // Mapeia o tipo de caixa de layout a partir do estilo e tipo do RenderNode
        let kind = match render_node.kind {
            RenderNodeKind::AnonymousBefore | RenderNodeKind::AnonymousAfter => {
                LayoutBoxKind::Anonymous
            }
            RenderNodeKind::Text => LayoutBoxKind::TextRun,
            RenderNodeKind::Element => match render_node.style.display() {
                Display::Block => LayoutBoxKind::Block,
                Display::Inline => LayoutBoxKind::Inline,
                Display::InlineBlock => LayoutBoxKind::InlineBlock,
                Display::Flex | Display::InlineFlex => LayoutBoxKind::Flex,
                Display::Grid | Display::InlineGrid => LayoutBoxKind::Grid,
                Display::Table => LayoutBoxKind::Table,
                Display::TableRow => LayoutBoxKind::TableRow,
                Display::TableCell => LayoutBoxKind::TableCell,
                Display::ListItem => LayoutBoxKind::Block,
                Display::None => return None,
            },
        };

        let mut children = Vec::with_capacity(render_node.children.len());
        for child_rn in &render_node.children {
            if let Some(child_box) = self.build_box_recursive(child_rn) {
                children.push(child_box);
            }
        }

        let mut layout_box =
            LayoutBox::new(render_node.node_id, kind, Arc::clone(&render_node.style));
        layout_box.children = children;

        Some(layout_box)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ace_dom::Document;
    use ace_style::computed::ComputedStyle;
    use ace_style::engine::RenderTreeNode;
    use rustc_hash::FxHashMap;

    fn node_id(val: u64) -> ace_core::id::NodeId {
        ace_core::id::NodeId::from_raw(val).unwrap()
    }

    #[test]
    fn test_box_tree_empty() {
        let doc = Document::new(None);
        let builder = BoxTreeBuilder::new(&doc);
        let render_tree = RenderTree::default();

        let box_tree = builder.build(&render_tree);
        assert!(box_tree.is_empty());
        assert_eq!(box_tree.box_count(), 0);
    }

    #[test]
    fn test_box_tree_builder_single_element() {
        let doc = Document::new(None);
        let builder = BoxTreeBuilder::new(&doc);

        let style = Arc::new(ComputedStyle::initial());
        let root_rn = RenderTreeNode::new_element(node_id(1), Arc::clone(&style), Vec::new());

        let mut node_styles = FxHashMap::default();
        node_styles.insert(node_id(1), Arc::clone(&style));

        let render_tree = RenderTree {
            root: Some(root_rn),
            node_styles,
        };

        let box_tree = builder.build(&render_tree);
        assert!(!box_tree.is_empty());
        assert_eq!(box_tree.box_count(), 1);
        assert_eq!(box_tree.root().unwrap().id, node_id(1));
    }

    #[test]
    fn test_box_tree_builder_with_mixed_children_wrapping() {
        let doc = Document::new(None);
        let builder = BoxTreeBuilder::new(&doc);

        let mut style_block_raw = ComputedStyle::initial();
        let mut box_block = (*style_block_raw.box_style).clone();
        box_block.display = Display::Block;
        style_block_raw.box_style = Arc::new(box_block);
        let style_block = Arc::new(style_block_raw);

        let mut style_inline_raw = ComputedStyle::initial();
        let mut box_inline = (*style_inline_raw.box_style).clone();
        box_inline.display = Display::Inline;
        style_inline_raw.box_style = Arc::new(box_inline);
        let style_inline = Arc::new(style_inline_raw);

        let inline_rn1 =
            RenderTreeNode::new_element(node_id(2), Arc::clone(&style_inline), Vec::new());
        let block_rn =
            RenderTreeNode::new_element(node_id(3), Arc::clone(&style_block), Vec::new());
        let inline_rn2 =
            RenderTreeNode::new_element(node_id(4), Arc::clone(&style_inline), Vec::new());

        let root_rn = RenderTreeNode::new_element(
            node_id(1),
            Arc::clone(&style_block),
            vec![inline_rn1, block_rn, inline_rn2],
        );

        let mut node_styles = FxHashMap::default();
        node_styles.insert(node_id(1), Arc::clone(&style_block));
        node_styles.insert(node_id(2), Arc::clone(&style_inline));
        node_styles.insert(node_id(3), Arc::clone(&style_block));
        node_styles.insert(node_id(4), Arc::clone(&style_inline));

        let render_tree = RenderTree {
            root: Some(root_rn),
            node_styles,
        };

        let box_tree = builder.build(&render_tree);
        let root = box_tree.root().unwrap();

        // CSS 2.2 §9.2.1.1 wrapping deve resultar em:
        // [Anonymous([Inline]), Block, Anonymous([Inline])]
        assert_eq!(root.children.len(), 3);
        assert_eq!(root.children[0].kind, LayoutBoxKind::Anonymous);
        assert_eq!(root.children[1].kind, LayoutBoxKind::Block);
        assert_eq!(root.children[2].kind, LayoutBoxKind::Anonymous);

        let dump = box_tree.dump();
        assert!(dump.contains("Block (id=1)"));
        assert!(dump.contains("Anonymous"));
    }

    #[test]
    fn test_box_tree_builder_omits_display_none() {
        let doc = Document::new(None);
        let builder = BoxTreeBuilder::new(&doc);

        let mut style_none_raw = ComputedStyle::initial();
        let mut box_none = (*style_none_raw.box_style).clone();
        box_none.display = Display::None;
        style_none_raw.box_style = Arc::new(box_none);
        let style_none = Arc::new(style_none_raw);

        let root_rn = RenderTreeNode::new_element(node_id(1), Arc::clone(&style_none), Vec::new());
        let render_tree = RenderTree {
            root: Some(root_rn),
            node_styles: FxHashMap::default(),
        };

        let box_tree = builder.build(&render_tree);
        assert!(box_tree.is_empty());
    }
}
