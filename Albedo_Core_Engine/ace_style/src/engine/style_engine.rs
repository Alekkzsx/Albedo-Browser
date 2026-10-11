//! # Orquestrador do Motor de Estilo e RenderTree (`StyleEngine`)
//!
//! Executa a cascata de cima para baixo no DOM, gerencia o `AncestorFilter`, MPC,
//! invalidação cirúrgica, injeção de pseudo-elementos e produz a `RenderTree`.

use crate::cascade::sorter::CascadeSorter;
use crate::computed::resolver::resolve_computed_style;
use crate::computed::style::ComputedStyle;
use crate::engine::cache::{StyleSharingCache, StyleSharingKey};
use crate::engine::invalidation::{
    DOMMutation, InvalidationResult, InvalidationSet, RuleInvalidationSets, StyleDirtyFlags,
};
use crate::engine::mpc::{compute_rules_signature, MatchedPropertiesCache, MpcKey};
use crate::model::layer::LayerRegistry;
use crate::model::media::MediaContext;
use crate::model::stylesheet::{CSSRule, StyleSheet};
use crate::parser::declaration_parser::parse_declarations;
use crate::rule_tree::bucket::RuleBucketMap;
use crate::user_agent::default_style::default_user_agent_stylesheet;
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use ace_dom::query::bloom::AncestorFilter;
use ace_dom::query::selector::PseudoElement;
use ace_dom::tree::Document;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use smol_str::SmolStr;
use std::sync::Arc;

/// Tipo de nó na árvore de renderização.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderNodeKind {
    /// Nó de elemento real do DOM
    Element,
    /// Caixa anônima gerada via pseudo-elemento `::before`
    AnonymousBefore,
    /// Caixa anônima gerada via pseudo-elemento `::after`
    AnonymousAfter,
    /// Nó de texto puro
    Text,
}

/// Nó da árvore de renderização associado a um `Arc<ComputedStyle>` imutável.
#[derive(Debug, Clone)]
pub struct RenderTreeNode {
    pub node_id: NodeId,
    pub kind: RenderNodeKind,
    pub style: Arc<ComputedStyle>,
    pub children: Vec<RenderTreeNode>,
}

impl RenderTreeNode {
    pub fn new_element(
        node_id: NodeId,
        style: Arc<ComputedStyle>,
        children: Vec<RenderTreeNode>,
    ) -> Self {
        Self {
            node_id,
            kind: RenderNodeKind::Element,
            style,
            children,
        }
    }

    pub fn new_anonymous_before(node_id: NodeId, style: Arc<ComputedStyle>) -> Self {
        Self {
            node_id,
            kind: RenderNodeKind::AnonymousBefore,
            style,
            children: Vec::new(),
        }
    }

    pub fn new_anonymous_after(node_id: NodeId, style: Arc<ComputedStyle>) -> Self {
        Self {
            node_id,
            kind: RenderNodeKind::AnonymousAfter,
            style,
            children: Vec::new(),
        }
    }
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
    pub mpc: MatchedPropertiesCache,
    pub invalidation_sets: RuleInvalidationSets,
    pub dirty_flags: FxHashMap<NodeId, StyleDirtyFlags>,
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
            mpc: MatchedPropertiesCache::new(),
            invalidation_sets: RuleInvalidationSets::new(),
            dirty_flags: FxHashMap::default(),
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
        self.invalidation_sets.add_stylesheet(&sheet);
        self.stylesheets.push(sheet);
        self.cache.clear();
        self.mpc.clear();
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
        self.mpc.clear();
    }

    /// Invalida cirurgicamente os nós afetados por uma mutação no DOM.
    pub fn invalidate_mutation(
        &mut self,
        doc: &Document,
        node_id: NodeId,
        mutation: &DOMMutation,
    ) -> InvalidationResult {
        let mut affected = FxHashSet::default();
        affected.insert(node_id);
        self.dirty_flags
            .insert(node_id, StyleDirtyFlags::RECALC_SELF);

        let mut descendant_targets = InvalidationSet::new();
        let mut sibling_targets = InvalidationSet::new();

        match mutation {
            DOMMutation::ClassChanged {
                old_class,
                new_class,
            } => {
                if let Some(ref oc) = old_class {
                    if let Some(set) = self.invalidation_sets.descendant_class_sets.get(oc) {
                        merge_invalidation_sets(&mut descendant_targets, set);
                    }
                    if let Some(set) = self.invalidation_sets.sibling_class_sets.get(oc) {
                        merge_invalidation_sets(&mut sibling_targets, set);
                    }
                }
                if let Some(ref nc) = new_class {
                    if let Some(set) = self.invalidation_sets.descendant_class_sets.get(nc) {
                        merge_invalidation_sets(&mut descendant_targets, set);
                    }
                    if let Some(set) = self.invalidation_sets.sibling_class_sets.get(nc) {
                        merge_invalidation_sets(&mut sibling_targets, set);
                    }
                }
            }
            DOMMutation::IdChanged { old_id, new_id } => {
                if let Some(ref oi) = old_id {
                    if let Some(set) = self.invalidation_sets.descendant_id_sets.get(oi) {
                        merge_invalidation_sets(&mut descendant_targets, set);
                    }
                }
                if let Some(ref ni) = new_id {
                    if let Some(set) = self.invalidation_sets.descendant_id_sets.get(ni) {
                        merge_invalidation_sets(&mut descendant_targets, set);
                    }
                }
            }
            DOMMutation::AttributeChanged { name } => {
                if let Some(set) = self.invalidation_sets.descendant_attr_sets.get(name) {
                    merge_invalidation_sets(&mut descendant_targets, set);
                }
            }
        }

        // Invalidação cirúrgica de descendentes se houver alvos
        if !descendant_targets.is_empty() {
            for (desc_id, desc_node) in doc.descendants(node_id) {
                if desc_id == node_id {
                    continue;
                }
                if let Some(el) = desc_node.as_element() {
                    let mut matches_target = descendant_targets.invalidates_all;
                    if !matches_target {
                        for class in el.classes.as_slice() {
                            if descendant_targets.target_classes.contains(class) {
                                matches_target = true;
                                break;
                            }
                        }
                    }
                    if !matches_target {
                        if let Some(ref id) = el.id_attr {
                            if descendant_targets.target_ids.contains(id) {
                                matches_target = true;
                            }
                        }
                    }
                    if !matches_target {
                        let lower_tag = Atom::new(&el.tag_name.as_str().to_ascii_lowercase());
                        if descendant_targets.target_tags.contains(&lower_tag) {
                            matches_target = true;
                        }
                    }

                    if matches_target {
                        affected.insert(desc_id);
                        self.dirty_flags
                            .insert(desc_id, StyleDirtyFlags::RECALC_SELF);
                    }
                }
            }
        }

        // Invalidação cirúrgica de irmãos se houver alvos
        if !sibling_targets.is_empty() {
            let mut curr = doc.get_node(node_id).and_then(|n| n.next_sibling);
            while let Some(sib_id) = curr {
                if let Some(sib_node) = doc.get_node(sib_id) {
                    if let Some(el) = sib_node.as_element() {
                        let mut matches_target = sibling_targets.invalidates_all;
                        if !matches_target {
                            for class in el.classes.as_slice() {
                                if sibling_targets.target_classes.contains(class) {
                                    matches_target = true;
                                    break;
                                }
                            }
                        }
                        if matches_target {
                            affected.insert(sib_id);
                            self.dirty_flags
                                .insert(sib_id, StyleDirtyFlags::RECALC_SELF);
                        }
                    }
                    curr = sib_node.next_sibling;
                } else {
                    break;
                }
            }
        }

        InvalidationResult {
            invalidated_count: affected.len(),
            affected_nodes: affected,
        }
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

    /// Computa os estilos utilizando paralelismo multithread com Rayon e thread-local caches.
    pub fn compute_parallel(&mut self, doc: &Document) -> RenderTree {
        let root_id = doc.root();
        let Some(root_node_data) = doc.get_node(root_id) else {
            return RenderTree::default();
        };

        // Computa o estilo do nó raiz na thread principal
        let mut sorter = CascadeSorter::new();
        let filter = AncestorFilter::new();
        let root_style =
            self.compute_single_element_style(doc, root_id, None, Some(&filter), &mut sorter);

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

        // Paraleliza a resolução das sub-árvores com Rayon utilizando thread-local caches
        type SubtreeResolution = (Option<RenderTreeNode>, Vec<(NodeId, Arc<ComputedStyle>)>);

        let stylesheets = &self.stylesheets;
        let bucket_map = &self.bucket_map;
        let layer_registry = &self.layer_registry;
        let viewport_size = (
            self.media_context.viewport_width,
            self.media_context.viewport_height,
        );

        let subtrees: Vec<SubtreeResolution> = child_ids
            .into_par_iter()
            .map_init(
                || {
                    (
                        CascadeSorter::new(),
                        AncestorFilter::new(),
                        StyleSharingCache::new(),
                    )
                },
                |(local_sorter, local_filter, local_cache), cid| {
                    local_sorter.clear();
                    local_filter.clear();
                    if let Some(el) = root_node_data.as_element() {
                        local_filter.push_element(el);
                    }

                    let mut local_styles = Vec::new();
                    let tree_node = compute_subtree_pure(
                        doc,
                        cid,
                        Some(&root_style),
                        local_filter,
                        local_sorter,
                        local_cache,
                        stylesheets,
                        bucket_map,
                        layer_registry,
                        viewport_size,
                        &mut local_styles,
                    );
                    (tree_node, local_styles)
                },
            )
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
            kind: RenderNodeKind::Element,
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
            // 1. Verifica o cache de compartilhamento de estilo entre irmãos idênticos
            let sharing_key = StyleSharingKey {
                parent_ptr: node.parent.map(|p| p.raw() as usize).unwrap_or(0),
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
                    parent_style,
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

        let viewport_size = (
            self.media_context.viewport_width,
            self.media_context.viewport_height,
        );

        // 2. Avalia Pseudo-Elemento ::before
        let before_rules = self.bucket_map.collect_matching_pseudo_rules(
            doc,
            node_id,
            PseudoElement::Before,
            &self.stylesheets,
        );
        let before_node = if !before_rules.is_empty() {
            sorter.resolve_cascade(&before_rules, &[], &self.layer_registry);
            let before_comp = resolve_computed_style(
                sorter.winning_declarations(),
                Some(&computed_style),
                16.0,
                viewport_size,
            );
            if before_comp.content.is_some() && !before_comp.is_display_none() {
                Some(RenderTreeNode::new_anonymous_before(
                    node_id,
                    Arc::new(before_comp),
                ))
            } else {
                None
            }
        } else {
            None
        };

        // 3. Processa os nós filhos DOM
        let mut dom_children = Vec::new();
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
                dom_children.push(child_render_node);
            }

            if let Some(child_node) = doc.get_node(child_id) {
                curr_child = child_node.next_sibling;
            } else {
                break;
            }
        }

        // 4. Avalia Pseudo-Elemento ::after
        let after_rules = self.bucket_map.collect_matching_pseudo_rules(
            doc,
            node_id,
            PseudoElement::After,
            &self.stylesheets,
        );
        let after_node = if !after_rules.is_empty() {
            sorter.resolve_cascade(&after_rules, &[], &self.layer_registry);
            let after_comp = resolve_computed_style(
                sorter.winning_declarations(),
                Some(&computed_style),
                16.0,
                viewport_size,
            );
            if after_comp.content.is_some() && !after_comp.is_display_none() {
                Some(RenderTreeNode::new_anonymous_after(
                    node_id,
                    Arc::new(after_comp),
                ))
            } else {
                None
            }
        } else {
            None
        };

        // Desempilha elemento do AncestorFilter
        if is_element {
            if let Some(el) = node.as_element() {
                ancestor_filter.pop_element(el);
            }
        }

        // Ordem de pintura CSS: ::before -> filhos DOM -> ::after
        let mut all_children = Vec::with_capacity(dom_children.len() + 2);
        if let Some(b) = before_node {
            all_children.push(b);
        }
        all_children.extend(dom_children);
        if let Some(a) = after_node {
            all_children.push(a);
        }

        Some(RenderTreeNode {
            node_id,
            kind: RenderNodeKind::Element,
            style: computed_style,
            children: all_children,
        })
    }

    fn compute_single_element_style(
        &mut self,
        doc: &Document,
        node_id: NodeId,
        parent_style: Option<&Arc<ComputedStyle>>,
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

        // Consulta o Matched Properties Cache (MPC)
        let rules_hash = compute_rules_signature(&matched_rules, &inline_decls);
        let parent_ptr = parent_style.map(|p| Arc::as_ptr(p) as usize).unwrap_or(0);
        let mpc_key = MpcKey {
            rules_hash,
            parent_ptr,
        };

        if let Some(cached) = self.mpc.get(&mpc_key) {
            return cached;
        }

        sorter.resolve_cascade(&matched_rules, &inline_decls, &self.layer_registry);

        let root_font_size = 16.0;
        let viewport_size = (
            self.media_context.viewport_width,
            self.media_context.viewport_height,
        );

        let computed = resolve_computed_style(
            sorter.winning_declarations(),
            parent_style.map(|p| p.as_ref()),
            root_font_size,
            viewport_size,
        );

        let style_arc = Arc::new(computed);
        self.mpc.insert(mpc_key, Arc::clone(&style_arc));
        style_arc
    }
}

/// Mescla um conjunto de invalidação de origem em um conjunto de destino.
fn merge_invalidation_sets(target: &mut InvalidationSet, source: &InvalidationSet) {
    if source.invalidates_all {
        target.invalidates_all = true;
    }
    for c in &source.target_classes {
        target.target_classes.insert(c.clone());
    }
    for id in &source.target_ids {
        target.target_ids.insert(id.clone());
    }
    for t in &source.target_tags {
        target.target_tags.insert(t.clone());
    }
    for a in &source.target_attributes {
        target.target_attributes.insert(a.clone());
    }
}

/// Função pura para resolução de sub-árvore em multithreading sem mutação compartilhada.
#[allow(clippy::too_many_arguments)]
fn compute_subtree_pure(
    doc: &Document,
    node_id: NodeId,
    parent_style: Option<&Arc<ComputedStyle>>,
    ancestor_filter: &mut AncestorFilter,
    sorter: &mut CascadeSorter,
    local_cache: &mut StyleSharingCache,
    stylesheets: &[StyleSheet],
    bucket_map: &RuleBucketMap,
    layer_registry: &LayerRegistry,
    viewport_size: (f32, f32),
    styles_out: &mut Vec<(NodeId, Arc<ComputedStyle>)>,
) -> Option<RenderTreeNode> {
    let node = doc.get_node(node_id)?;

    let computed_style = if let Some(el) = node.as_element() {
        let sharing_key = StyleSharingKey {
            parent_ptr: node.parent.map(|p| p.raw() as usize).unwrap_or(0),
            tag_name: el.tag_name.clone(),
            id_attr: el.id_attr.clone(),
            classes: el.classes.as_slice().to_vec(),
            inline_style: el.get_attribute("style").map(SmolStr::new),
        };

        if let Some(cached) = local_cache.try_get(&sharing_key) {
            cached
        } else {
            let matched =
                bucket_map.collect_matching_rules(doc, node_id, stylesheets, Some(ancestor_filter));
            let inline_decls = node
                .as_element()
                .and_then(|e| e.get_attribute("style"))
                .map(parse_declarations)
                .unwrap_or_default();

            sorter.resolve_cascade(&matched, &inline_decls, layer_registry);
            let resolved = resolve_computed_style(
                sorter.winning_declarations(),
                parent_style.map(|p| p.as_ref()),
                16.0,
                viewport_size,
            );
            let style_arc = Arc::new(resolved);
            local_cache.insert(sharing_key, Arc::clone(&style_arc));
            style_arc
        }
    } else if let Some(parent) = parent_style {
        parent.clone()
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

    // Avalia Pseudo-Elemento ::before
    let before_rules =
        bucket_map.collect_matching_pseudo_rules(doc, node_id, PseudoElement::Before, stylesheets);
    let before_node = if !before_rules.is_empty() {
        sorter.resolve_cascade(&before_rules, &[], layer_registry);
        let before_comp = resolve_computed_style(
            sorter.winning_declarations(),
            Some(&computed_style),
            16.0,
            viewport_size,
        );
        if before_comp.content.is_some() && !before_comp.is_display_none() {
            Some(RenderTreeNode::new_anonymous_before(
                node_id,
                Arc::new(before_comp),
            ))
        } else {
            None
        }
    } else {
        None
    };

    let mut dom_children = Vec::new();
    let mut curr_child = node.first_child;
    while let Some(child_id) = curr_child {
        if let Some(crn) = compute_subtree_pure(
            doc,
            child_id,
            Some(&computed_style),
            ancestor_filter,
            sorter,
            local_cache,
            stylesheets,
            bucket_map,
            layer_registry,
            viewport_size,
            styles_out,
        ) {
            dom_children.push(crn);
        }

        if let Some(child_node) = doc.get_node(child_id) {
            curr_child = child_node.next_sibling;
        } else {
            break;
        }
    }

    // Avalia Pseudo-Elemento ::after
    let after_rules =
        bucket_map.collect_matching_pseudo_rules(doc, node_id, PseudoElement::After, stylesheets);
    let after_node = if !after_rules.is_empty() {
        sorter.resolve_cascade(&after_rules, &[], layer_registry);
        let after_comp = resolve_computed_style(
            sorter.winning_declarations(),
            Some(&computed_style),
            16.0,
            viewport_size,
        );
        if after_comp.content.is_some() && !after_comp.is_display_none() {
            Some(RenderTreeNode::new_anonymous_after(
                node_id,
                Arc::new(after_comp),
            ))
        } else {
            None
        }
    } else {
        None
    };

    if let Some(el) = node.as_element() {
        ancestor_filter.pop_element(el);
    }

    let mut all_children = Vec::with_capacity(dom_children.len() + 2);
    if let Some(b) = before_node {
        all_children.push(b);
    }
    all_children.extend(dom_children);
    if let Some(a) = after_node {
        all_children.push(a);
    }

    Some(RenderTreeNode {
        node_id,
        kind: RenderNodeKind::Element,
        style: computed_style,
        children: all_children,
    })
}
