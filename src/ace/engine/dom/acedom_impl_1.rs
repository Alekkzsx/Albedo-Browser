use super::*;
use std::fmt::Write;
use crate::ace::html::{parse_fragment, HtmlDocument, HtmlNode, is_void_element};


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

    /// TODO: add docs
    pub fn from_html(html: &str) -> Self {
        // Sempre usa ACE-HTML parser proprietário (feature flag removida)
        let parsed = build_document_with_errors(html);
        Self::from_html_document(&parsed.document)
    }

    /// TODO: add docs
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

    /// TODO: add docs
    pub fn query_selector(&self, selector: &str) -> Option<usize> {
        self.query_selector_all(selector).into_iter().next()
    }

    /// TODO: add docs
    pub fn query_selector_all(&self, selector: &str) -> Vec<usize> {
        let selector = selector.trim();
        if selector.is_empty() {
            return Vec::new();
        }

        let mut matches = Vec::new();
        self.collect_selector_matches(self.root, selector, &mut matches);
        matches
    }

    /// TODO: add docs
    pub fn to_html(&self) -> String {
        self.serialize_subtree_html(self.root)
    }

pub(crate) fn collect_selector_matches(&self, node_idx: usize, selector: &str, matches: &mut Vec<usize>) {
        if let Some(node) = self.get_node(node_idx) {
            if self.node_matches_selector(node, selector) {
                matches.push(node_idx);
            }

            for &child_idx in &node.children {
                self.collect_selector_matches(child_idx, selector, matches);
            }
        }
    }

pub(crate) fn node_matches_selector(&self, node: &AceNode, selector: &str) -> bool {
        let AceNodeType::Element(element) = &node.node_type else {
            return false;
        };

        if let Some(id) = selector.strip_prefix('#') {
            return element.attributes.get("id").is_some_and(|value| value == id);
        }

        if let Some(class_name) = selector.strip_prefix('.') {
            return element
                .attributes
                .get("class")
                .is_some_and(|value| value.split_ascii_whitespace().any(|class| class == class_name));
        }

        element.tag.eq_ignore_ascii_case(selector)
    }
}
