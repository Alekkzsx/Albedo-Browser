//! # Orquestrador do Motor de Estilo e RenderTree (`StyleEngine`)
//!
//! Executa a cascata de cima para baixo no DOM, gerencia o `AncestorFilter` e produz a `RenderTree`.

use crate::cascade::sorter::CascadeSorter;
use crate::computed::resolver::resolve_computed_style;
use crate::computed::style::ComputedStyle;
use crate::engine::cache::{StyleSharingCache, StyleSharingKey};
use crate::model::layer::LayerRegistry;
use crate::model::media::MediaContext;
use crate::model::stylesheet::{CSSRule, StyleSheet};
use crate::parser::declaration_parser::parse_declarations;
use crate::rule_tree::bucket::RuleBucketMap;
use crate::user_agent::default_style::default_user_agent_stylesheet;
use ace_core::id::NodeId;
use ace_dom::query::bloom::AncestorFilter;
use ace_dom::tree::Document;
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::Arc;

/// Nó da árvore de renderização associado a um `Arc<ComputedStyle>` imutável.
#[derive(Debug, Clone)]
pub struct RenderTreeNode {
    pub node_id: NodeId,
    pub style: Arc<ComputedStyle>,
    pub children: Vec<RenderTreeNode>,
}

/// Árvore de renderização contendo apenas nós visíveis (`display != none`) com seus estilos finais.
#[derive(Debug, Clone, Default)]
pub struct RenderTree {
    pub root: Option<RenderTreeNode>,
    pub node_styles: FxHashMap<NodeId, Arc<ComputedStyle>>,
}

impl RenderTree {
    pub fn get_style(&self, id: NodeId) -> Option<&Arc<ComputedStyle>> {
        self.node_styles.get(&id)
    }
}

/// Motor principal de cálculo de estilo e cascata CSS.
pub struct StyleEngine {
    pub stylesheets: Vec<StyleSheet>,
    pub bucket_map: RuleBucketMap,
    pub layer_registry: LayerRegistry,
    pub media_context: MediaContext,
    pub cache: StyleSharingCache,
}

impl Default for StyleEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleEngine {
    /// Cria um novo motor de estilo inicializado com a folha padrão do User-Agent (WHATWG HTML §15).
    pub fn new() -> Self {
        let mut engine = Self {
            stylesheets: Vec::new(),
            bucket_map: RuleBucketMap::new(),
            layer_registry: LayerRegistry::new(),
            media_context: MediaContext::default(),
            cache: StyleSharingCache::new(),
        };

        // Adiciona a folha de estilo padrão do User-Agent
        let ua_sheet = default_user_agent_stylesheet();
        engine.add_stylesheet(ua_sheet);
        engine
    }

    /// Adiciona uma folha de estilo de autor ou externa e atualiza os baldes de busca e camadas.
    pub fn add_stylesheet(&mut self, sheet: StyleSheet) {
        let sheet_idx = self.stylesheets.len();
        self.register_sheet_layers(&sheet);
        self.bucket_map.add_stylesheet(sheet_idx, &sheet);
        self.stylesheets.push(sheet);
        self.cache.clear();
    }

    fn register_sheet_layers(&mut self, sheet: &StyleSheet) {
        for rule in &sheet.rules {
            match rule {
                CSSRule::LayerStatement(stmt) => {
                    self.layer_registry.register_statement(&stmt.names);
                }
                CSSRule::LayerBlock(block) => {
                    self.layer_registry.register_layer(&block.name);
                }
                _ => {}
            }
        }
    }

    /// Atualiza o contexto de mídia do dispositivo / tela.
    pub fn set_media_context(&mut self, ctx: MediaContext) {
        self.media_context = ctx;
        self.cache.clear();
    }

    /// Computa os estilos de toda a árvore DOM e gera a `RenderTree`.
    pub fn compute_styles(&mut self, doc: &Document) -> RenderTree {
        let mut render_tree = RenderTree::default();
        let root_id = doc.root();

        let mut ancestor_filter = AncestorFilter::new();
        let mut sorter = CascadeSorter::new();

        let root_node = self.compute_node_recursive(
            doc,
            root_id,
            None,
            &mut ancestor_filter,
            &mut sorter,
            &mut render_tree.node_styles,
        );

        render_tree.root = root_node;
        render_tree
    }

    /// Computa os estilos utilizando paralelismo multithread com Rayon em sub-árvores independentes.
    pub fn compute_parallel(&mut self, doc: &Document) -> RenderTree {
        let root_id = doc.root();
        let Some(root_node_data) = doc.get_node(root_id) else {
            return RenderTree::default();
        };

        // Computa o estilo do nó raiz na thread principal
        let mut sorter = CascadeSorter::new();
        let mut filter = AncestorFilter::new();
        let root_style = self.compute_single_element_style(doc, root_id, None, Some(&filter), &mut sorter);

        if root_style.is_display_none() {
            return RenderTree::default();
        }

        let mut render_tree = RenderTree::default();
        render_tree.node_styles.insert(root_id, root_style.clone());

        // Identifica os filhos diretos do nó raiz
        let mut child_ids = Vec::new();
        let mut curr_child = root_node_data.first_child;
        while let Some(cid) = curr_child {
            child_ids.push(cid);
            if let Some(cnode) = doc.get_node(cid) {
                curr_child = cnode.next_sibling;
            } else {
                break;
            }
        }

        // Paraleliza a resolução das sub-árvores com Rayon
        let stylesheets = &self.stylesheets;
        let bucket_map = &self.bucket_map;
        let layer_registry = &self.layer_registry;
        let viewport_size = (self.media_context.viewport_width, self.media_context.viewport_height);

        let subtrees: Vec<(Option<RenderTreeNode>, Vec<(NodeId, Arc<ComputedStyle>)>)> = child_ids
            .into_par_iter()
            .map(|cid| {
                let mut local_sorter = CascadeSorter::new();
                let mut local_filter = AncestorFilter::new();
                if let Some(el) = root_node_data.as_element() {
                    local_filter.push_element(el);
                }

                let mut local_styles = Vec::new();
                let tree_node = compute_subtree_pure(
                    doc,
                    cid,
                    Some(&root_style),
                    &mut local_filter,
                    &mut local_sorter,
                    stylesheets,
                    bucket_map,
                    layer_registry,
                    viewport_size,
                    &mut local_styles,
                );
                (tree_node, local_styles)
            })
            .collect();

        let mut children = Vec::new();
        for (subtree, styles) in subtrees {
            for (nid, sty) in styles {
                render_tree.node_styles.insert(nid, sty);
            }
            if let Some(rn) = subtree {
                children.push(rn);
            }
        }

        render_tree.root = Some(RenderTreeNode {
            node_id: root_id,
            style: root_style,
            children,
        });

        render_tree
    }

    fn compute_node_recursive(
        &mut self,
        doc: &Document,
        node_id: NodeId,
        parent_style: Option<&Arc<ComputedStyle>>,
        ancestor_filter: &mut AncestorFilter,
        sorter: &mut CascadeSorter,
        styles_map: &mut FxHashMap<NodeId, Arc<ComputedStyle>>,
    ) -> Option<RenderTreeNode> {
        let node = doc.get_node(node_id)?;

        let computed_style = if let Some(el) = node.as_element() {
            // Verifica o cache de compartilhamento de estilo entre irmãos idênticos
            let sharing_key = StyleSharingKey {
                parent_ptr: parent_style.map(|p| Arc::as_ptr(p) as usize).unwrap_or(0),
                tag_name: el.tag_name.clone(),
                id_attr: el.id_attr.clone(),
                classes: el.classes.as_slice().to_vec(),
                inline_style: el.get_attribute("style").map(SmolStr::new),
            };

            if let Some(cached) = self.cache.try_get(&sharing_key) {
                cached
            } else {
                let style = self.compute_single_element_style(
                    doc,
                    node_id,
                    parent_style.map(|s| s.as_ref()),
                    Some(ancestor_filter),
                    sorter,
                );
                self.cache.insert(sharing_key, style.clone());
                style
            }
        } else {
            // Nós de texto ou comentário: herdam o estilo do pai ou initial
            if let Some(parent) = parent_style {
                parent.clone()
            } else {
                Arc::new(ComputedStyle::initial())
            }
        };

        styles_map.insert(node_id, computed_style.clone());

        // Se display: none, este nó e todos os seus descendentes são excluídos da RenderTree
        if computed_style.is_display_none() {
            return None;
        }

        // Empilha elemento no AncestorFilter
        let is_element = node.is_element();
        if let Some(el) = node.as_element() {
            ancestor_filter.push_element(el);
        }

        // Processa os nós filhos
        let mut children = Vec::new();
        let mut curr_child = node.first_child;
        while let Some(child_id) = curr_child {
            if let Some(child_render_node) = self.compute_node_recursive(
                doc,
                child_id,
                Some(&computed_style),
                ancestor_filter,
                sorter,
                styles_map,
            ) {
                children.push(child_render_node);
            }

            if let Some(child_node) = doc.get_node(child_id) {
                curr_child = child_node.next_sibling;
            } else {
                break;
            }
        }

        // Desempilha elemento do AncestorFilter
        if is_element {
            if let Some(el) = node.as_element() {
                ancestor_filter.pop_element(el);
            }
        }

        Some(RenderTreeNode {
            node_id,
            style: computed_style,
            children,
        })
    }

    fn compute_single_element_style(
        &self,
        doc: &Document,
        node_id: NodeId,
        parent_style: Option<&ComputedStyle>,
        ancestor_filter: Option<&AncestorFilter>,
        sorter: &mut CascadeSorter,
    ) -> Arc<ComputedStyle> {
        let matched_rules = self.bucket_map.collect_matching_rules(
            doc,
            node_id,
            &self.stylesheets,
            ancestor_filter,
        );

        let inline_decls = if let Some(node) = doc.get_node(node_id) {
            if let Some(el) = node.as_element() {
                if let Some(style_attr) = el.get_attribute("style") {
                    parse_declarations(style_attr)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        sorter.resolve_cascade(&matched_rules, &inline_decls, &self.layer_registry);

        let root_font_size = 16.0;
        let viewport_size = (self.media_context.viewport_width, self.media_context.viewport_height);

        let computed = resolve_computed_style(
            sorter.winning_declarations(),
            parent_style,
            root_font_size,
            viewport_size,
        );

        Arc::new(computed)
    }
}

/// Função pura para resolução de sub-árvore em multithreading sem mutação compartilhada.
fn compute_subtree_pure(
    doc: &Document,
    node_id: NodeId,
    parent_style: Option<&ComputedStyle>,
    ancestor_filter: &mut AncestorFilter,
    sorter: &mut CascadeSorter,
    stylesheets: &[StyleSheet],
    bucket_map: &RuleBucketMap,
    layer_registry: &LayerRegistry,
    viewport_size: (f32, f32),
    styles_out: &mut Vec<(NodeId, Arc<ComputedStyle>)>,
) -> Option<RenderTreeNode> {
    let node = doc.get_node(node_id)?;

    let computed_style = if node.is_element() {
        let matched = bucket_map.collect_matching_rules(doc, node_id, stylesheets, Some(ancestor_filter));
        let inline_decls = node
            .as_element()
            .and_then(|el| el.get_attribute("style"))
            .map(parse_declarations)
            .unwrap_or_default();

        sorter.resolve_cascade(&matched, &inline_decls, layer_registry);
        let resolved = resolve_computed_style(
            sorter.winning_declarations(),
            parent_style,
            16.0,
            viewport_size,
        );
        Arc::new(resolved)
    } else if let Some(parent) = parent_style {
        Arc::new(parent.clone())
    } else {
        Arc::new(ComputedStyle::initial())
    };

    styles_out.push((node_id, computed_style.clone()));

    if computed_style.is_display_none() {
        return None;
    }

    if let Some(el) = node.as_element() {
        ancestor_filter.push_element(el);
    }

    let mut children = Vec::new();
    let mut curr_child = node.first_child;
    while let Some(child_id) = curr_child {
        if let Some(crn) = compute_subtree_pure(
            doc,
            child_id,
            Some(&computed_style),
            ancestor_filter,
            sorter,
            stylesheets,
            bucket_map,
            layer_registry,
            viewport_size,
            styles_out,
        ) {
            children.push(crn);
        }

        if let Some(child_node) = doc.get_node(child_id) {
            curr_child = child_node.next_sibling;
        } else {
            break;
        }
    }

    if let Some(el) = node.as_element() {
        ancestor_filter.pop_element(el);
    }

    Some(RenderTreeNode {
        node_id,
        style: computed_style,
        children,
    })
}
