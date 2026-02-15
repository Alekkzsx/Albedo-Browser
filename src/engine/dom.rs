use kuchiki::NodeRef;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct AceDOM {
    pub nodes: Vec<AceNode>,
    pub root: usize,
    pub head: Option<usize>,
    pub body: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct AceNode {
    pub node_type: AceNodeType,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub prev_sibling: Option<usize>,
    pub next_sibling: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AceNodeType {
    Element(AceElement),
    Text(String),
    Comment(String),
    Document,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AceElement {
    pub tag: String,
    pub attributes: HashMap<String, String>,
}

impl AceDOM {
    pub fn new(kuchiki_root: NodeRef) -> Self {
        let mut nodes = Vec::new();
        
        let root_idx = Self::convert_recursive(&kuchiki_root, &mut nodes, None);

        let mut dom = Self {
            nodes,
            root: root_idx,
            head: None,
            body: None,
        };

        dom.find_head_body();
        dom
    }

    pub fn get_node(&self, id: usize) -> Option<&AceNode> {
        self.nodes.get(id)
    }

    fn convert_recursive(
        kuchiki_node: &NodeRef,
        nodes: &mut Vec<AceNode>,
        parent_idx: Option<usize>,
    ) -> usize {
        let node_type = if let Some(el) = kuchiki_node.as_element() {
            let tag = el.name.local.to_string();
            let mut attributes = HashMap::new();
            for (curr_name, curr_val) in el.attributes.borrow().map.iter() {
                attributes.insert(curr_name.local.to_string(), curr_val.value.to_string());
            }

            AceNodeType::Element(AceElement { tag, attributes })
        } else if let Some(text) = kuchiki_node.as_text() {
            AceNodeType::Text(text.borrow().clone()) 
        } else if let Some(comment) = kuchiki_node.as_comment() {
            AceNodeType::Comment(comment.borrow().clone())
        } else {
            AceNodeType::Document
        };

        let current_idx = nodes.len();
        nodes.push(AceNode {
            node_type,
            parent: parent_idx,
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
        });

        let mut children_indices = Vec::new();
        for child in kuchiki_node.children() {
            let child_idx = Self::convert_recursive(&child, nodes, Some(current_idx));
            children_indices.push(child_idx);
        }

        if !children_indices.is_empty() {
             for i in 0..children_indices.len() {
                let curr = children_indices[i];
                let prev = if i > 0 { Some(children_indices[i-1]) } else { None };
                let next = if i < children_indices.len() - 1 { Some(children_indices[i+1]) } else { None };

                if let Some(node) = nodes.get_mut(curr) {
                    node.prev_sibling = prev;
                    node.next_sibling = next;
                }
             }
        }

        if let Some(node) = nodes.get_mut(current_idx) {
            node.children = children_indices;
        }

        current_idx
    }

    fn find_head_body(&mut self) {
        // Busca simples a partir da raiz
        // Assume que html é filho da raiz, e head/body são filhos de html
        if let Some(root_node) = self.nodes.get(self.root) {
             for &child_idx in &root_node.children {
                 if let Some(html_node) = self.nodes.get(child_idx) {
                     if let AceNodeType::Element(el) = &html_node.node_type {
                         if el.tag == "html" {
                             // html encontrado, procurar head e body nos filhos
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
        
        // Fallback: busca em profundidade se não achou na estrutura padrão
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
}
