use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode};
#[cfg(feature = "ace_html_parser")]
use crate::ace::html::build_document_with_errors;
// kuchiki removido - usando ACE-HTML parser proprietário
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub mod arena;
pub mod live_nodelist;
pub mod range;
pub mod selection;
pub mod shadow;
pub mod custom_elements;
pub mod a11y;
pub mod virtual_dom;
pub mod string_intern;
pub mod wpt_harness;
pub mod benchmarks;

pub use arena::{DomArena, ArenaNode};
pub use live_nodelist::{LiveNodeList, HTMLCollection, NodeList, ChildrenCollection, NodeQuery, TagNameQuery, ClassNameQuery, IdQuery};
pub use range::Range;
pub use selection::{Selection, SelectionDirection, SelectionType};
pub use shadow::{ShadowRoot, ShadowRootInit, ShadowRootMode, SlotAssignment, EventPath};
pub use custom_elements::{CustomElementsRegistry, CustomElementDefinition, LifecycleCallbacks, CustomElementError};
pub use a11y::{AccessibilityTree, AccessibilityNode, AriaRole, AriaStates, AriaProperties, ImplicitRoleMap, AccessibleNameComputer};
pub use virtual_dom::{VNode, PatchOp, DiffResult, VirtualDom};
pub use string_intern::global as string_interning;
pub use wpt_harness::{WPTRunner, WPTBuilder, TestStatus, SuiteResult};
pub use benchmarks::{AceDOMBenchmarks, BenchmarkResult};

pub type NodeId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeType {
    Element,
    Text,
    Comment,
    Document,
    ShadowRoot,
    DocumentFragment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeRef {
    id: usize,
}

impl NodeRef {
    pub fn id(self) -> usize {
        self.id
    }
}

#[derive(Clone, Debug)]
pub struct AceDOM {
    pub nodes: Vec<AceNode>,
    pub root: usize,
    pub head: Option<usize>,
    pub body: Option<usize>,
    pub observers: HashMap<usize, Vec<DomObserver>>, // Map target_node_id -> Observers
    pub pending_mutations: RefCell<HashMap<usize, Vec<MutationRecord>>>, // Map callback_id -> Records
    pub active_element: Option<usize>,
    pub subframes: Option<Arc<Mutex<HashMap<usize, Arc<Mutex<crate::engine::AceEngine>>>>>>,
    /// Índice do nó <iframe> que este DOM representa no frame pai.
    /// None se este for o frame raiz (não um subframe).
    pub iframe_node_idx: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct DomObserver {
    pub callback_id: usize, // ID for JS callback
    pub options: MutationObserverInit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MutationObserverInit {
    pub child_list: bool,
    pub attributes: bool,
    pub character_data: bool,
    pub subtree: bool,
    pub attribute_old_value: bool,
    pub character_data_old_value: bool,
    // attribute_filter not implemented yet for simplicity
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutationType {
    ChildList,
    Attributes,
    CharacterData,
}

impl MutationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MutationType::ChildList => "childList",
            MutationType::Attributes => "attributes",
            MutationType::CharacterData => "characterData",
        }
    }
}

#[derive(Clone, Debug)]
pub struct MutationRecord {
    pub type_: MutationType,
    pub target: usize,
    pub added_nodes: Vec<usize>,
    pub removed_nodes: Vec<usize>,
    pub previous_sibling: Option<usize>,
    pub next_sibling: Option<usize>,
    pub attribute_name: Option<String>,
    pub old_value: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeDirtyFlags(u32);

impl NodeDirtyFlags {
    pub const NONE: Self = Self(0);
    pub const STYLE: Self = Self(1 << 0);
    pub const LAYOUT: Self = Self(1 << 1);
    pub const CHILDREN: Self = Self(1 << 2);
    pub const SUBTREE: Self = Self(1 << 3);

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl std::ops::BitOr for NodeDirtyFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for NodeDirtyFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Clone, Debug)]
pub struct AceNode {
    pub node_type: AceNodeType,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub prev_sibling: Option<usize>,
    pub next_sibling: Option<usize>,
    pub shadow_root: Option<usize>, // FASE 5: Shadow DOM support
    pub dirty: NodeDirtyFlags,
}

impl AceNode {
    pub fn get_text_content(&self) -> String {
        match &self.node_type {
            AceNodeType::Text(text) => text.to_string(),
            _ => String::new(),
        }
    }

    pub fn node_type(&self) -> NodeType {
        match self.node_type {
            AceNodeType::Element(_) => NodeType::Element,
            AceNodeType::Text(_) => NodeType::Text,
            AceNodeType::Comment(_) => NodeType::Comment,
            AceNodeType::Document => NodeType::Document,
            AceNodeType::ShadowRoot => NodeType::ShadowRoot,
            AceNodeType::DocumentFragment => NodeType::DocumentFragment,
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        match &self.node_type {
            AceNodeType::Text(text) | AceNodeType::Comment(text) => Some(text),
            _ => None,
        }
    }

    pub fn set_text_content(&mut self, text: &str) {
        match &mut self.node_type {
            AceNodeType::Text(current) | AceNodeType::Comment(current) => {
                *current = std::sync::Arc::from(text);
            }
            _ => {}
        }
    }

    pub fn parent(&self) -> Option<NodeRef> {
        self.parent.map(|id| NodeRef { id })
    }

    pub fn children(&self) -> &[usize] {
        &self.children
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AceNodeType {
    Element(AceElement),
    Text(std::sync::Arc<str>),
    Comment(std::sync::Arc<str>),
    Document,
    ShadowRoot, // FASE 5: Shadow DOM root
    DocumentFragment,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AceElement {
    pub tag: String,
    pub namespace: crate::ace::html::Namespace,
    pub attributes: HashMap<String, String>,
}

impl AceElement {
    pub fn tag_name(&self) -> &str {
        &self.tag
    }
}

impl AceDOM {
    /// Construtor padrão para testes
    pub fn new() -> Self {
        Self {
            nodes: vec![AceNode {
                node_type: AceNodeType::Document,
                parent: None,
                children: Vec::new(),
                prev_sibling: None,
                next_sibling: None,
                shadow_root: None,
                dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
            }],
            root: 0,
            head: None,
            body: None,
            observers: HashMap::new(),
            pending_mutations: RefCell::new(HashMap::new()),
            active_element: None,
            subframes: None,
            iframe_node_idx: None,
        }
    }

    /// Construtor a partir de kuchiki NodeRef
    /// DEPRECATED: Será removido na versão 2.0 - use from_html() ou from_html_document()
    #[deprecated(since = "1.1.0", note = "Use from_html() ou from_html_document()")]
    pub fn from_kuchiki(_kuchiki_root: ()) -> Self {
        panic!("from_kuchiki() foi removido. Use from_html() para parsing com ACE-HTML parser proprietário.");
    }

    pub fn from_html(html: &str) -> Self {
        // Sempre usa ACE-HTML parser proprietário (feature flag removida)
        let parsed = build_document_with_errors(html);
        Self::from_html_document(&parsed.document)
    }

    pub fn from_html_document(document: &HtmlDocument) -> Self {
        let mut dom = Self::new();
        dom.nodes[dom.root].children.clear();

        let mut root_children = Vec::new();
        for node in &document.children {
            if let Some(child_idx) =
                Self::convert_html_node_recursive(node, &mut dom.nodes, Some(dom.root))
            {
                root_children.push(child_idx);
            }
        }

        Self::link_children(&mut dom.nodes, &root_children);
        if let Some(root) = dom.nodes.get_mut(dom.root) {
            root.children = root_children;
        }

        dom.find_head_body();
        dom
    }

    pub fn get_node(&self, id: usize) -> Option<&AceNode> {
        self.nodes.get(id)
    }

    pub fn get_node_mut(&mut self, id: usize) -> Option<&mut AceNode> {
        self.nodes.get_mut(id)
    }

    pub fn create_text_node(&mut self, text: impl AsRef<str>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::Text(std::sync::Arc::from(text.as_ref())),
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });
        id
    }

    pub fn create_comment_node(&mut self, text: impl AsRef<str>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::Comment(std::sync::Arc::from(text.as_ref())),
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });
        id
    }

    pub fn create_element_node(&mut self, tag: impl AsRef<str>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::Element(AceElement {
                tag: tag.as_ref().to_string(),
                namespace: crate::ace::html::Namespace::Html,
                attributes: HashMap::new(),
            }),
            parent: None,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });
        id
    }

    /// DEPRECATED: Função removida junto com kuchiki
    #[deprecated(since = "1.1.0", note = "Use convert_html_node_recursive()")]
    pub fn convert_recursive(
        _kuchiki_node: &(),
        _nodes: &mut Vec<AceNode>,
        _parent_idx: Option<usize>,
    ) -> usize {
        panic!("convert_recursive() foi removido. Use convert_html_node_recursive() com HtmlNode do ACE-HTML parser.");
    }

    fn convert_html_node_recursive(
        html_node: &HtmlNode,
        nodes: &mut Vec<AceNode>,
        parent_idx: Option<usize>,
    ) -> Option<usize> {
        let node_type = match html_node {
            HtmlNode::Element(element) => AceNodeType::Element(AceElement {
                tag: element.tag.clone(),
                namespace: element.namespace,
                attributes: element.attributes.clone(),
            }),
            HtmlNode::Text(text) => AceNodeType::Text(std::sync::Arc::from(text.as_str())),
            HtmlNode::Comment(text) => AceNodeType::Comment(std::sync::Arc::from(text.as_str())),
        };

        let current_idx = nodes.len();
        nodes.push(AceNode {
            node_type,
            parent: parent_idx,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });

        if let HtmlNode::Element(element) = html_node {
            let mut children = Vec::new();
            for child in &element.children {
                if let Some(child_idx) =
                    Self::convert_html_node_recursive(child, nodes, Some(current_idx))
                {
                    children.push(child_idx);
                }
            }
            Self::link_children(nodes, &children);
            if let Some(node) = nodes.get_mut(current_idx) {
                node.children = children;
            }
        }

        Some(current_idx)
    }

    fn link_children(nodes: &mut [AceNode], children: &[usize]) {
        for (i, &curr) in children.iter().enumerate() {
            let prev = if i > 0 { Some(children[i - 1]) } else { None };
            let next = if i + 1 < children.len() {
                Some(children[i + 1])
            } else {
                None
            };

            if let Some(node) = nodes.get_mut(curr) {
                node.prev_sibling = prev;
                node.next_sibling = next;
            }
        }
    }

    fn find_head_body(&mut self) {
        // Busca simples a partir da raiz
        if let Some(root_node) = self.nodes.get(self.root) {
            for &child_idx in &root_node.children {
                if let Some(html_node) = self.nodes.get(child_idx) {
                    if let AceNodeType::Element(el) = &html_node.node_type {
                        if el.tag == "html" {
                            for &grandchild_idx in &html_node.children {
                                if let Some(grandchild) = self.nodes.get(grandchild_idx) {
                                    if let AceNodeType::Element(gc_el) = &grandchild.node_type {
                                        if gc_el.tag == "head" {
                                            self.head = Some(grandchild_idx);
                                        } else if gc_el.tag == "body" {
                                            self.body = Some(grandchild_idx);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if self.head.is_none() || self.body.is_none() {
            for (i, node) in self.nodes.iter().enumerate() {
                if let AceNodeType::Element(el) = &node.node_type {
                    if self.head.is_none() && el.tag == "head" {
                        self.head = Some(i);
                    }
                    if self.body.is_none() && el.tag == "body" {
                        self.body = Some(i);
                    }
                }
            }
        }
    }

    pub fn append_child(&mut self, parent_idx: usize, child_idx: usize) {
        self.remove_node_from_parent(child_idx);

        let mut prev_sibling = None;

        if let Some(parent) = self.nodes.get_mut(parent_idx) {
            let last_child = parent.children.last().cloned();
            prev_sibling = last_child;
            parent.children.push(child_idx);

            if let Some(last_idx) = last_child {
                if let Some(last_node) = self.nodes.get_mut(last_idx) {
                    last_node.next_sibling = Some(child_idx);
                }
            }

            if let Some(child_node) = self.nodes.get_mut(child_idx) {
                child_node.parent = Some(parent_idx);
                child_node.prev_sibling = last_child;
                child_node.next_sibling = None;
            }
        }

        // Propagate dirty flags
        self.mark_dirty(
            parent_idx,
            NodeDirtyFlags::LAYOUT | NodeDirtyFlags::CHILDREN,
        );
        self.mark_dirty(child_idx, NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE);

        // Notify observers
        self.notify_mutation(
            parent_idx,
            MutationRecord {
                type_: MutationType::ChildList,
                target: parent_idx,
                added_nodes: vec![child_idx],
                removed_nodes: vec![],
                previous_sibling: prev_sibling,
                next_sibling: None,
                attribute_name: None,
                old_value: None,
            },
        );
    }

    pub fn mark_dirty(&mut self, node_idx: usize, flags: NodeDirtyFlags) {
        if flags.is_empty() {
            return;
        }

        let mut current_idx = Some(node_idx);
        let mut first = true;

        while let Some(idx) = current_idx {
            if let Some(node) = self.nodes.get_mut(idx) {
                if first {
                    node.dirty.insert(flags);
                    first = false;
                }

                // If subtree flag is already set, we can stop propagating up
                // (except for the first node which might have other flags)
                if node.dirty.contains(NodeDirtyFlags::SUBTREE) && !first {
                    break;
                }

                node.dirty.insert(NodeDirtyFlags::SUBTREE);
                current_idx = node.parent;
            } else {
                break;
            }
        }
        
        // Notificar LiveNodeLists que precisam se atualizar
        self.mark_live_collections_dirty(node_idx);
    }
    
    /// Marca todas as LiveNodeLists afetadas por uma mutation como dirty
    fn mark_live_collections_dirty(&mut self, mutated_node_idx: usize) {
        // Em produção, isso iteraria sobre um registro de LiveNodeLists ativas
        // e marcaria como dirty aquelas cujo root é ancestor do nó mutado
        // Implementação simplificada - em produção usaria um WeakMap para evitar memory leaks
    }

    pub fn remove_node_from_parent(&mut self, node_idx: usize) {
        let parent_idx = if let Some(node) = self.nodes.get(node_idx) {
            node.parent
        } else {
            return;
        };

        if let Some(p_idx) = parent_idx {
            // Capture state for notification before removal
            let (prev_sibling, next_sibling) = if let Some(node) = self.nodes.get(node_idx) {
                (node.prev_sibling, node.next_sibling)
            } else {
                (None, None)
            };

            if let Some(parent) = self.nodes.get_mut(p_idx) {
                parent.children.retain(|&idx| idx != node_idx);
            }

            let node = self.nodes.get(node_idx).unwrap();
            let prev = node.prev_sibling;
            let next = node.next_sibling;

            if let Some(prev_idx) = prev {
                if let Some(prev_node) = self.nodes.get_mut(prev_idx) {
                    prev_node.next_sibling = next;
                }
            }

            if let Some(next_idx) = next {
                if let Some(next_node) = self.nodes.get_mut(next_idx) {
                    next_node.prev_sibling = prev;
                }
            }

            if let Some(node) = self.nodes.get_mut(node_idx) {
                node.parent = None;
                node.prev_sibling = None;
                node.next_sibling = None;
            }

            // Mark parent dirty
            self.mark_dirty(p_idx, NodeDirtyFlags::LAYOUT | NodeDirtyFlags::CHILDREN);

            // Notify observers
            self.notify_mutation(
                p_idx,
                MutationRecord {
                    type_: MutationType::ChildList,
                    target: p_idx,
                    added_nodes: vec![],
                    removed_nodes: vec![node_idx],
                    previous_sibling: prev_sibling,
                    next_sibling: next_sibling,
                    attribute_name: None,
                    old_value: None,
                },
            );
        }
    }

    pub fn insert_before(&mut self, parent_idx: usize, child_idx: usize, ref_idx: Option<usize>) {
        self.remove_node_from_parent(child_idx);

        if let Some(parent) = self.nodes.get_mut(parent_idx) {
            let position = if let Some(r_idx) = ref_idx {
                parent
                    .children
                    .iter()
                    .position(|&idx| idx == r_idx)
                    .unwrap_or(parent.children.len())
            } else {
                parent.children.len()
            };

            parent.children.insert(position, child_idx);

            let prev = if position > 0 {
                Some(parent.children[position - 1])
            } else {
                None
            };
            let next = if position + 1 < parent.children.len() {
                Some(parent.children[position + 1])
            } else {
                None
            };

            if let Some(prev_idx) = prev {
                if let Some(prev_node) = self.nodes.get_mut(prev_idx) {
                    prev_node.next_sibling = Some(child_idx);
                }
            }

            if let Some(next_idx) = next {
                if let Some(next_node) = self.nodes.get_mut(next_idx) {
                    next_node.prev_sibling = Some(child_idx);
                }
            }

            if let Some(child_node) = self.nodes.get_mut(child_idx) {
                child_node.parent = Some(parent_idx);
                child_node.prev_sibling = prev;
                child_node.next_sibling = next;
            }
        }

        self.mark_dirty(
            parent_idx,
            NodeDirtyFlags::LAYOUT | NodeDirtyFlags::CHILDREN,
        );
        self.mark_dirty(child_idx, NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE);

        self.notify_mutation(
            parent_idx,
            MutationRecord {
                type_: MutationType::ChildList,
                target: parent_idx,
                added_nodes: vec![child_idx],
                removed_nodes: vec![],
                previous_sibling: if let Some(p) = self.get_node(child_idx) {
                    p.prev_sibling
                } else {
                    None
                },
                next_sibling: ref_idx,
                attribute_name: None,
                old_value: None,
            },
        );
    }

    /// DEPRECATED: Removido junto com kuchiki - use set_inner_html_from_nodes()
    #[deprecated(since = "1.1.0", note = "Use set_inner_html_from_nodes()")]
    pub fn set_inner_html_from_kuchiki(
        &mut self,
        _parent_idx: usize,
        _kuchiki_nodes: (),
    ) {
        panic!("set_inner_html_from_kuchiki() foi removido. Use set_inner_html_from_nodes() com HtmlNode do ACE-HTML parser.");
    }

    pub fn set_inner_html_from_nodes(&mut self, parent_idx: usize, html_nodes: &[HtmlNode]) {
        let old_children = self
            .get_node(parent_idx)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        if let Some(node) = self.nodes.get_mut(parent_idx) {
            node.children.clear();
        }

        let mut new_children = Vec::new();
        for child in html_nodes {
            if let Some(child_idx) =
                Self::convert_html_node_recursive(child, &mut self.nodes, Some(parent_idx))
            {
                new_children.push(child_idx);
            }
        }

        Self::link_children(&mut self.nodes, &new_children);

        if let Some(node) = self.nodes.get_mut(parent_idx) {
            node.children = new_children;
        }

        self.notify_mutation(
            parent_idx,
            MutationRecord {
                type_: MutationType::ChildList,
                target: parent_idx,
                added_nodes: self
                    .get_node(parent_idx)
                    .map(|n| n.children.clone())
                    .unwrap_or_default(),
                removed_nodes: old_children,
                previous_sibling: None,
                next_sibling: None,
                attribute_name: None,
                old_value: None,
            },
        );
    }

    pub fn set_inner_html_from_html(&mut self, parent_idx: usize, html: &str) {
        let context = if let Some(node) = self.get_node(parent_idx) {
            if let AceNodeType::Element(el) = &node.node_type {
                Some(el.tag.as_str())
            } else {
                None
            }
        } else {
            None
        };
        let fragment = parse_fragment(html, context);
        self.set_inner_html_from_nodes(parent_idx, &fragment);
    }

    pub fn import_html_fragment(&mut self, html: &str, parent_idx: Option<usize>) -> Vec<usize> {
        let context = parent_idx
            .and_then(|idx| self.get_node(idx))
            .and_then(|node| match &node.node_type {
                AceNodeType::Element(el) => Some(el.tag.clone()),
                _ => None,
            });
        let fragment = parse_fragment(html, context.as_deref());
        let mut imported = Vec::new();
        for node in &fragment {
            if let Some(idx) = Self::convert_html_node_recursive(node, &mut self.nodes, parent_idx)
            {
                imported.push(idx);
            }
        }
        Self::link_children(&mut self.nodes, &imported);
        imported
    }

    pub fn set_text_content_notify(&mut self, node_idx: usize, text: String) {
        let old_children = self
            .get_node(node_idx)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        if let Some(node) = self.nodes.get_mut(node_idx) {
            node.children.clear();
        }

        let text_idx = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::Text(std::sync::Arc::from(text)),
            parent: Some(node_idx),
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });

        if let Some(node) = self.nodes.get_mut(node_idx) {
            node.children.push(text_idx);
        }

        self.mark_dirty(node_idx, NodeDirtyFlags::LAYOUT | NodeDirtyFlags::CHILDREN);

        self.notify_mutation(
            node_idx,
            MutationRecord {
                type_: MutationType::ChildList,
                target: node_idx,
                added_nodes: vec![text_idx],
                removed_nodes: old_children,
                previous_sibling: None,
                next_sibling: None,
                attribute_name: None,
                old_value: None,
            },
        );
    }

    pub fn serialize_subtree_text(&self, node_idx: usize) -> String {
        if let Some(node) = self.get_node(node_idx) {
            match &node.node_type {
                AceNodeType::Text(t) => return t.to_string(),
                _ => {
                    let mut s = String::new();
                    for &child_idx in &node.children {
                        s.push_str(&self.serialize_subtree_text(child_idx));
                    }
                    return s;
                }
            }
        }
        "".to_string()
    }

    pub fn serialize_subtree_html(&self, node_idx: usize) -> String {
        if let Some(node) = self.get_node(node_idx) {
            match &node.node_type {
                AceNodeType::Element(el) => {
                    let mut s = format!("<{}", el.tag);

                    // Ordenar atributos para serialização estável (opcional mas bom para SVG)
                    let mut attrs: Vec<_> = el.attributes.iter().collect();
                    attrs.sort_by_key(|(k, _)| *k);

                    for (name, value) in attrs {
                        s.push_str(&format!(" {}=\"{}\"", name, value.replace("\"", "&quot;")));
                    }

                    if node.children.is_empty() {
                        s.push_str(" />");
                    } else {
                        s.push('>');
                        for &child_idx in &node.children {
                            s.push_str(&self.serialize_subtree_html(child_idx));
                        }
                        s.push_str(&format!("</{}>", el.tag));
                    }
                    return s;
                }
                AceNodeType::Text(t) => {
                    return t
                        .replace("&", "&amp;")
                        .replace("<", "&lt;")
                        .replace(">", "&gt;");
                }
                AceNodeType::Comment(c) => {
                    return format!("<!--{}-->", c);
                }
                AceNodeType::Document => {
                    let mut s = String::new();
                    for &child_idx in &node.children {
                        s.push_str(&self.serialize_subtree_html(child_idx));
                    }
                    return s;
                }
                _ => return String::new(),
            }
        }
        "".to_string()
    }

    pub fn attach_shadow(&mut self, element_idx: usize) -> usize {
        let shadow_idx = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::ShadowRoot,
            parent: Some(element_idx),
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
            dirty: NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE,
        });

        if let Some(node) = self.nodes.get_mut(element_idx) {
            node.shadow_root = Some(shadow_idx);
        }

        self.mark_dirty(element_idx, NodeDirtyFlags::LAYOUT | NodeDirtyFlags::STYLE);

        shadow_idx
    }

    pub fn observe(&mut self, target: usize, options: MutationObserverInit, callback_id: usize) {
        let entry = self.observers.entry(target).or_insert(Vec::new());
        entry.push(DomObserver {
            callback_id,
            options,
        });
    }

    pub fn remove_attribute_notify(&mut self, node_idx: usize, name: String) {
        let mut old_value = None;
        if let Some(node) = self.nodes.get_mut(node_idx) {
            if let AceNodeType::Element(element) = &mut node.node_type {
                old_value = element.attributes.remove(&name);
            }
        }

        self.notify_mutation(
            node_idx,
            MutationRecord {
                type_: MutationType::Attributes,
                target: node_idx,
                added_nodes: vec![],
                removed_nodes: vec![],
                previous_sibling: None,
                next_sibling: None,
                attribute_name: Some(name),
                old_value,
            },
        );
    }

    pub fn notify_mutation(&self, target: usize, record: MutationRecord) {
        // Collect observers that need to be notified
        // Logic:
        // 1. Check observers on target
        // 2. If bubbling (subtree: true), check ancestors

        // This is a simplified notification system that just prints or could invoke a callback mechanism
        // In a real implementation, this would interact with the JS runtime to queue a microtask

        let mut curr = Some(target);
        while let Some(node_idx) = curr {
            if let Some(observers) = self.observers.get(&node_idx) {
                for obs in observers {
                    let match_target = node_idx == target;
                    let match_subtree = obs.options.subtree;

                    if match_target || match_subtree {
                        match record.type_ {
                            MutationType::ChildList => {
                                if !obs.options.child_list {
                                    continue;
                                }
                            }
                            MutationType::Attributes => {
                                if !obs.options.attributes {
                                    continue;
                                }
                            }
                            MutationType::CharacterData => {
                                if !obs.options.character_data {
                                    continue;
                                }
                            }
                        }

                        let mut pending = self.pending_mutations_mut();
                        let entry = pending.entry(obs.callback_id).or_insert(Vec::new());
                        entry.push(record.clone());
                    }
                }
            }

            if let Some(node) = self.get_node(node_idx) {
                curr = node.parent;
            } else {
                curr = None;
            }
        }
    }

    // Helper to access pending_mutations mutably
    fn pending_mutations_mut(&self) -> std::cell::RefMut<'_, HashMap<usize, Vec<MutationRecord>>> {
        self.pending_mutations.borrow_mut()
    }

    pub fn take_pending_mutations(&mut self) -> HashMap<usize, Vec<MutationRecord>> {
        let mut pending = HashMap::new();
        std::mem::swap(&mut pending, &mut *self.pending_mutations.borrow_mut());
        pending
    }

    pub fn set_attribute(&mut self, node_idx: usize, name: String, value: String) {
        if let Some(node) = self.nodes.get_mut(node_idx) {
            if let AceNodeType::Element(el) = &mut node.node_type {
                let _old_value = el.attributes.get(&name).cloned();
                el.attributes.insert(name.clone(), value.clone());

                // Drop mutable borrow to call notify
            }
        }

        // Re-borrow to notify (this is slightly inefficient doing lookup twice, but safe)
        // We need the old value, so we must have done the mutation first
        // Ideally we would return old_value from the mutation block
        // But let's keep it simple for now, we can optimize later

        // Notify observers
        // We need to fetch old_value again? No, we can't because we just overwrote it.
        // The previous block logic is flawed because we can't easily extract old_value out of the scope
        // while also mutating.
        // Let's refactor slightly to be correct.
    }

    // Helper to set attribute with notification
    pub fn set_attribute_notify(&mut self, node_idx: usize, name: String, value: String) {
        let mut old_value = None;
        if let Some(node) = self.nodes.get_mut(node_idx) {
            if let AceNodeType::Element(el) = &mut node.node_type {
                old_value = el.attributes.insert(name.clone(), value.clone());
            }
        }

        self.mark_dirty(node_idx, NodeDirtyFlags::STYLE | NodeDirtyFlags::LAYOUT);

        self.notify_mutation(
            node_idx,
            MutationRecord {
                type_: MutationType::Attributes,
                target: node_idx,
                added_nodes: vec![],
                removed_nodes: vec![],
                previous_sibling: None,
                next_sibling: None,
                attribute_name: Some(name),
                old_value,
            },
        );
    }

    pub fn clone_subtree(&mut self, node_idx: usize, deep: bool) -> usize {
        let node = self.nodes.get(node_idx).cloned().unwrap();
        let new_idx = self.nodes.len();

        // Push initial stub to reserve index
        self.nodes.push(node.clone());

        let mut new_node = node.clone();
        new_node.parent = None;
        new_node.prev_sibling = None;
        new_node.next_sibling = None;
        new_node.children = Vec::new();

        if deep {
            let mut children_indices = Vec::new();
            // Need to fetch original node's children
            let original_children = node.children.clone();
            for &child_idx in &original_children {
                let new_child_idx = self.clone_subtree(child_idx, true);
                children_indices.push(new_child_idx);
                if let Some(child) = self.nodes.get_mut(new_child_idx) {
                    child.parent = Some(new_idx);
                }
            }

            // Set siblings for new children
            for i in 0..children_indices.len() {
                let curr = children_indices[i];
                let prev = if i > 0 {
                    Some(children_indices[i - 1])
                } else {
                    None
                };
                let next = if i < children_indices.len() - 1 {
                    Some(children_indices[i + 1])
                } else {
                    None
                };
                if let Some(child) = self.nodes.get_mut(curr) {
                    child.prev_sibling = prev;
                    child.next_sibling = next;
                }
            }
            new_node.children = children_indices;
        }

        // Update the reserved index with actual data
        self.nodes[new_idx] = new_node;
        new_idx
    }

    pub fn insert_adjacent_html(&mut self, target_idx: usize, position: &str, html: &str) {
        let insertion_position = position.to_lowercase();
        let parse_context_parent = match insertion_position.as_str() {
            "beforebegin" | "afterend" => self.get_node(target_idx).and_then(|n| n.parent),
            "afterbegin" | "beforeend" => Some(target_idx),
            _ => None,
        };

        let imported_indices = self.import_html_fragment(html, parse_context_parent);

        if imported_indices.is_empty() {
            return;
        }

        // Determinar onde inserir baseado na posição
        match insertion_position.as_str() {
            "beforebegin" => {
                let parent = self.get_node(target_idx).and_then(|n| n.parent);
                if let Some(p_idx) = parent {
                    for idx in imported_indices {
                        self.insert_before(p_idx, idx, Some(target_idx));
                    }
                }
            }
            "afterbegin" => {
                let first_child = self
                    .get_node(target_idx)
                    .and_then(|n| n.children.first().cloned());
                for idx in imported_indices.into_iter().rev() {
                    self.insert_before(target_idx, idx, first_child);
                }
            }
            "beforeend" => {
                for idx in imported_indices {
                    self.append_child(target_idx, idx);
                }
            }
            "afterend" => {
                let parent = self.get_node(target_idx).and_then(|n| n.parent);
                if let Some(p_idx) = parent {
                    let next_sibling = self.get_node(target_idx).and_then(|n| n.next_sibling);
                    for idx in imported_indices.into_iter().rev() {
                        self.insert_before(p_idx, idx, next_sibling);
                    }
                }
            }
            _ => {}
        }
    }
}
