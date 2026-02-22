use kuchiki::NodeRef;
use std::collections::HashMap;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

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

#[derive(Clone, Debug)]
pub struct MutationRecord {
    pub type_: String, // "childList", "attributes", "characterData"
    pub target: usize,
    pub added_nodes: Vec<usize>,
    pub removed_nodes: Vec<usize>,
    pub previous_sibling: Option<usize>,
    pub next_sibling: Option<usize>,
    pub attribute_name: Option<String>,
    pub old_value: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AceNode {
    pub node_type: AceNodeType,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub prev_sibling: Option<usize>,
    pub next_sibling: Option<usize>,
    pub shadow_root: Option<usize>, // FASE 5: Shadow DOM support
}

impl AceNode {
    pub fn get_text_content(&self) -> String {
        match &self.node_type {
            AceNodeType::Text(text) => text.clone(),
            _ => String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AceNodeType {
    Element(AceElement),
    Text(String),
    Comment(String),
    Document,
    ShadowRoot, // FASE 5: Shadow DOM root
    DocumentFragment,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AceElement {
    pub tag: String,
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
            }],
            root: 0,
            head: None,
            body: None,
            observers: HashMap::new(),
            pending_mutations: RefCell::new(HashMap::new()),
            active_element: None,
            subframes: None,
        }
    }

    /// Construtor a partir de kuchiki NodeRef
    pub fn from_kuchiki(kuchiki_root: NodeRef) -> Self {
        let mut nodes = Vec::new();
        
        let root_idx = Self::convert_recursive(&kuchiki_root, &mut nodes, None);

        let mut dom = Self {
            nodes,
            root: root_idx,
            head: None,
            body: None,
            observers: HashMap::new(),
            pending_mutations: RefCell::new(HashMap::new()),
            active_element: None,
            subframes: None,
        };

        dom.find_head_body();
        dom
    }
    pub fn get_node(&self, id: usize) -> Option<&AceNode> {
        self.nodes.get(id)
    }

    pub fn convert_recursive(
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
            shadow_root: None,
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
        
        // Notify observers
        self.notify_mutation(parent_idx, MutationRecord {
            type_: "childList".to_string(),
            target: parent_idx,
            added_nodes: vec![child_idx],
            removed_nodes: vec![],
            previous_sibling: prev_sibling,
            next_sibling: None,
            attribute_name: None,
            old_value: None,
        });
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
            
            // Notify observers
            self.notify_mutation(p_idx, MutationRecord {
                type_: "childList".to_string(),
                target: p_idx,
                added_nodes: vec![],
                removed_nodes: vec![node_idx],
                previous_sibling: prev_sibling,
                next_sibling: next_sibling,
                attribute_name: None,
                old_value: None,
            });
        }
    }

    pub fn insert_before(&mut self, parent_idx: usize, child_idx: usize, ref_idx: Option<usize>) {
        self.remove_node_from_parent(child_idx);

        if let Some(parent) = self.nodes.get_mut(parent_idx) {
            let position = if let Some(r_idx) = ref_idx {
                parent.children.iter().position(|&idx| idx == r_idx).unwrap_or(parent.children.len())
            } else {
                parent.children.len()
            };

            parent.children.insert(position, child_idx);
            
            let prev = if position > 0 { Some(parent.children[position - 1]) } else { None };
            let next = if position + 1 < parent.children.len() { Some(parent.children[position + 1]) } else { None };

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

        self.notify_mutation(parent_idx, MutationRecord {
            type_: "childList".to_string(),
            target: parent_idx,
            added_nodes: vec![child_idx],
            removed_nodes: vec![],
            previous_sibling: if let Some(p) = self.get_node(child_idx) { p.prev_sibling } else { None },
            next_sibling: ref_idx,
            attribute_name: None,
            old_value: None,
        });
    }

    pub fn set_inner_html_from_kuchiki(&mut self, parent_idx: usize, kuchiki_nodes: kuchiki::iter::Siblings) {
        let old_children = self.get_node(parent_idx).map(|n| n.children.clone()).unwrap_or_default();
        if let Some(node) = self.nodes.get_mut(parent_idx) {
            node.children.clear();
        }

        let mut new_children = Vec::new();
        for child in kuchiki_nodes {
            let child_idx = Self::convert_recursive(&child, &mut self.nodes, Some(parent_idx));
            new_children.push(child_idx);
        }

        if !new_children.is_empty() {
            for i in 0..new_children.len() {
                let curr = new_children[i];
                let prev = if i > 0 { Some(new_children[i-1]) } else { None };
                let next = if i < new_children.len() - 1 { Some(new_children[i+1]) } else { None };

                if let Some(node) = self.nodes.get_mut(curr) {
                    node.prev_sibling = prev;
                    node.next_sibling = next;
                }
            }
        }

        if let Some(node) = self.nodes.get_mut(parent_idx) {
            node.children = new_children;
        }

        self.notify_mutation(parent_idx, MutationRecord {
            type_: "childList".to_string(),
            target: parent_idx,
            added_nodes: self.get_node(parent_idx).map(|n| n.children.clone()).unwrap_or_default(),
            removed_nodes: old_children,
            previous_sibling: None,
            next_sibling: None,
            attribute_name: None,
            old_value: None,
        });
    }

    pub fn set_text_content_notify(&mut self, node_idx: usize, text: String) {
        let old_children = self.get_node(node_idx).map(|n| n.children.clone()).unwrap_or_default();
        
        if let Some(node) = self.nodes.get_mut(node_idx) {
            node.children.clear();
        }

        let text_idx = self.nodes.len();
        self.nodes.push(AceNode {
            node_type: AceNodeType::Text(text),
            parent: Some(node_idx),
            children: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
            shadow_root: None,
        });

        if let Some(node) = self.nodes.get_mut(node_idx) {
            node.children.push(text_idx);
        }

        self.notify_mutation(node_idx, MutationRecord {
            type_: "childList".to_string(),
            target: node_idx,
            added_nodes: vec![text_idx],
            removed_nodes: old_children,
            previous_sibling: None,
            next_sibling: None,
            attribute_name: None,
            old_value: None,
        });
    }

    pub fn serialize_subtree_text(&self, node_idx: usize) -> String {
        if let Some(node) = self.get_node(node_idx) {
            match &node.node_type {
                AceNodeType::Text(t) => return t.clone(),
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
                },
                AceNodeType::Text(t) => {
                    return t.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;");
                },
                AceNodeType::Comment(c) => {
                    return format!("<!--{}-->", c);
                },
                AceNodeType::Document => {
                    let mut s = String::new();
                    for &child_idx in &node.children {
                        s.push_str(&self.serialize_subtree_html(child_idx));
                    }
                    return s;
                },
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
        });
        
        if let Some(node) = self.nodes.get_mut(element_idx) {
            node.shadow_root = Some(shadow_idx);
        }
        
        shadow_idx
    }

    pub fn observe(&mut self, target: usize, options: MutationObserverInit, callback_id: usize) {
        let entry = self.observers.entry(target).or_insert(Vec::new());
        entry.push(DomObserver { callback_id, options });
    }

    pub fn remove_attribute_notify(&mut self, node_idx: usize, name: String) {
        let mut old_value = None;
        if let Some(node) = self.nodes.get_mut(node_idx) {
             if let AceNodeType::Element(element) = &mut node.node_type {
                 old_value = element.attributes.remove(&name);
             }
        }
        
        self.notify_mutation(node_idx, MutationRecord {
            type_: "attributes".to_string(),
            target: node_idx,
            added_nodes: vec![],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: Some(name),
            old_value,
        });
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
                       match record.type_.as_str() {
                           "childList" => if !obs.options.child_list { continue; },
                           "attributes" => if !obs.options.attributes { continue; },
                           "characterData" => if !obs.options.character_data { continue; },
                           _ => {},
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
    fn pending_mutations_mut(&self) -> std::cell::RefMut<HashMap<usize, Vec<MutationRecord>>> {
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
                let old_value = el.attributes.get(&name).cloned();
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
        
        self.notify_mutation(node_idx, MutationRecord {
            type_: "attributes".to_string(),
            target: node_idx,
            added_nodes: vec![],
            removed_nodes: vec![],
            previous_sibling: None,
            next_sibling: None,
            attribute_name: Some(name),
            old_value,
        });
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
                let prev = if i > 0 { Some(children_indices[i-1]) } else { None };
                let next = if i < children_indices.len() - 1 { Some(children_indices[i+1]) } else { None };
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
        use kuchiki::traits::TendrilSink;
        let parser = kuchiki::parse_html().from_utf8();
        let dom = parser.one(html.as_bytes());
        
        // Always look for <body> because kuchiki always creates one for HTML
        let mut body = None;
        for node in dom.inclusive_descendants() {
            if let Some(el) = node.as_element() {
                if el.name.local.as_ref() == "body" {
                    body = Some(node);
                    break;
                }
            }
        }

        let source = body.unwrap_or(dom);
        let mut imported_indices = Vec::new();
        for child in source.children() {
            let idx = Self::convert_recursive(&child, &mut self.nodes, None);
            imported_indices.push(idx);
        }

        if imported_indices.is_empty() { return; }

        // Determinar onde inserir baseado na posição
        match position.to_lowercase().as_str() {
            "beforebegin" => {
                let parent = self.get_node(target_idx).and_then(|n| n.parent);
                if let Some(p_idx) = parent {
                    for idx in imported_indices {
                        self.insert_before(p_idx, idx, Some(target_idx));
                    }
                }
            }
            "afterbegin" => {
                let first_child = self.get_node(target_idx).and_then(|n| n.children.first().cloned());
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
