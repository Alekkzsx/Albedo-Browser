use rquickjs::Result;
use kuchiki::NodeRef;
use kuchiki::traits::*;
use std::collections::HashSet;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct DomTokenList {
    #[qjs(skip_trace)]
    pub node: NodeRef,
}

impl DomTokenList {
    fn update_class_attribute(&self, classes: &HashSet<String>) {
        if let Some(data) = self.node.as_element() {
            let mut attrs = data.attributes.borrow_mut();
            if classes.is_empty() {
                attrs.remove("class");
            } else {
                let val = classes.iter().cloned().collect::<Vec<_>>().join(" ");
                attrs.insert("class", val);
            }
        }
    }

    fn get_classes(&self) -> HashSet<String> {
        if let Some(data) = self.node.as_element() {
            if let Some(class_attr) = data.attributes.borrow().get("class") {
                return class_attr.split_whitespace().map(|s| s.to_string()).collect();
            }
        }
        HashSet::new()
    }
}

#[rquickjs::methods]
impl DomTokenList {

    #[qjs(rename = "add")]
    pub fn add(&self, token: String) {
        if token.is_empty() || token.contains(char::is_whitespace) {
            return; // Invalid token
        }
        let mut classes = self.get_classes();
        classes.insert(token);
        self.update_class_attribute(&classes);
    }

    #[qjs(rename = "remove")]
    pub fn remove(&self, token: String) {
        let mut classes = self.get_classes();
        if classes.remove(&token) {
            self.update_class_attribute(&classes);
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
        present
    }

    #[qjs(rename = "contains")]
    pub fn contains(&self, token: String) -> bool {
        self.get_classes().contains(&token)
    }
}
