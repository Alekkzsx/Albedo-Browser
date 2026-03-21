use crate::engine::dom::{AceDOM, AceNodeType};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct DomTokenList {
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<AceDOM>>,
    #[qjs(skip_trace)]
    pub index: usize,
    #[qjs(skip_trace)]
    pub mutations: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub stylesheet_dirty: Arc<Mutex<bool>>,
}

impl DomTokenList {
    fn update_class_attribute(&self, classes: &HashSet<String>) {
        if let Ok(mut dom) = self.dom.lock() {
            let val = if classes.is_empty() {
                None
            } else {
                Some(classes.iter().cloned().collect::<Vec<_>>().join(" "))
            };

            if let Some(v) = val {
                dom.set_attribute_notify(self.index, "class".to_string(), v);
            } else {
                // We need a remove_attribute_notify too, or just call notify manually
                let mut old_value = None;
                if let Some(node) = dom.nodes.get_mut(self.index) {
                    if let AceNodeType::Element(element) = &mut node.node_type {
                        old_value = element.attributes.remove("class");
                    }
                }
                dom.notify_mutation(
                    self.index,
                    crate::engine::dom::MutationRecord {
                        type_: crate::engine::dom::MutationType::Attributes,
                        target: self.index,
                        added_nodes: vec![],
                        removed_nodes: vec![],
                        previous_sibling: None,
                        next_sibling: None,
                        attribute_name: Some("class".to_string()),
                        old_value,
                    },
                );
            }
        }
    }

    fn get_classes(&self) -> HashSet<String> {
        if let Ok(dom) = self.dom.lock() {
            if let Some(node) = dom.get_node(self.index) {
                if let AceNodeType::Element(element) = &node.node_type {
                    if let Some(class_attr) = element.attributes.get("class") {
                        return class_attr
                            .split_whitespace()
                            .map(|s| s.to_string())
                            .collect();
                    }
                }
            }
        }
        HashSet::new()
    }
}

#[rquickjs::methods]
impl DomTokenList {
    fn mark_mutation(&self) {
        if let Ok(mut m) = self.mutations.lock() {
            *m = true;
        }
        // Invalidate styles if class changes
        if let Ok(mut sd) = self.stylesheet_dirty.lock() {
            *sd = true;
        }
    }

    #[qjs(rename = "add")]
    pub fn add(&self, token: String) {
        if token.is_empty() || token.contains(char::is_whitespace) {
            return; // Invalid token
        }
        let mut classes = self.get_classes();
        classes.insert(token);
        self.update_class_attribute(&classes);
        self.mark_mutation();
    }

    #[qjs(rename = "remove")]
    pub fn remove(&self, token: String) {
        let mut classes = self.get_classes();
        if classes.remove(&token) {
            self.update_class_attribute(&classes);
            self.mark_mutation();
        }
    }

    #[qjs(rename = "toggle")]
    pub fn toggle(&self, token: String) -> bool {
        let mut classes = self.get_classes();
        let present = if classes.contains(&token) {
            classes.remove(&token);
            false
        } else {
            classes.insert(token);
            true
        };
        self.update_class_attribute(&classes);
        self.mark_mutation();
        return present;
    }

    #[qjs(rename = "contains")]
    pub fn contains(&self, token: String) -> bool {
        self.get_classes().contains(&token)
    }

    #[qjs(get, rename = "length")]
    pub fn length(&self) -> usize {
        self.get_classes().len()
    }

    #[qjs(rename = "item")]
    pub fn item(&self, index: usize) -> Option<String> {
        let classes = self.get_classes();
        let mut vec: Vec<_> = classes.into_iter().collect();
        vec.sort(); // Consistent ordering
        vec.get(index).cloned()
    }

    #[qjs(rename = "replace")]
    pub fn replace(&self, old_token: String, new_token: String) -> bool {
        let mut classes = self.get_classes();
        if classes.remove(&old_token) {
            classes.insert(new_token);
            self.update_class_attribute(&classes);
            self.mark_mutation();
            return true;
        }
        false
    }

    #[qjs(get, rename = "value")]
    pub fn get_value(&self) -> String {
        self.get_classes()
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[qjs(set, rename = "value")]
    pub fn set_value(&self, val: String) {
        let classes: HashSet<String> = val.split_whitespace().map(|s| s.to_string()).collect();
        self.update_class_attribute(&classes);
        self.mark_mutation();
    }
}
