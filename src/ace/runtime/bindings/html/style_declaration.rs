use crate::ace::engine::dom::{AceDOM, AceNodeType};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct CssStyleDeclaration {
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<AceDOM>>,
    #[qjs(skip_trace)]
    pub index: usize,
    #[qjs(skip_trace)]
    pub mutations: Arc<Mutex<bool>>,
    #[qjs(skip_trace)]
    pub stylesheet_dirty: Arc<Mutex<bool>>,
}

impl CssStyleDeclaration {
    fn parse_style(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Ok(dom) = self.dom.lock() {
            if let Some(node) = dom.get_node(self.index) {
                if let AceNodeType::Element(element) = &node.node_type {
                    if let Some(style_attr) = element.attributes.get("style") {
                        for decl in style_attr.split(';') {
                            let decl = decl.trim();
                            if decl.is_empty() {
                                continue;
                            }
                            if let Some((key, val)) = decl.split_once(':') {
                                map.insert(key.trim().to_string(), val.trim().to_string());
                            }
                        }
                    }
                }
            }
        }
        map
    }

    fn update_style_attribute(&self, map: &HashMap<String, String>) {
        if let Ok(mut dom) = self.dom.lock() {
            if let Some(node) = dom.nodes.get_mut(self.index) {
                if let AceNodeType::Element(element) = &mut node.node_type {
                    if map.is_empty() {
                        element.attributes.remove("style");
                    } else {
                        let mut style_str = String::new();
                        for (key, val) in map {
                            style_str.push_str(key);
                            style_str.push_str(": ");
                            style_str.push_str(val);
                            style_str.push_str("; ");
                        }
                        element
                            .attributes
                            .insert("style".to_string(), style_str.trim().to_string());
                    }
                }
            }
        }
    }
}

#[rquickjs::methods]
impl CssStyleDeclaration {
    fn mark_mutation(&self) {
        if let Ok(mut m) = self.mutations.lock() {
            *m = true;
        }
        if let Ok(mut sd) = self.stylesheet_dirty.lock() {
            *sd = true;
        }
    }

    #[qjs(rename = "setProperty")]
    pub fn set_property(&self, property: String, value: String) {
        let mut map = self.parse_style();
        map.insert(property, value);
        self.update_style_attribute(&map);

        if let Ok(dom) = self.dom.lock() {
            dom.notify_mutation(
                self.index,
                crate::ace::engine::dom::MutationRecord {
                    type_: crate::ace::engine::dom::MutationType::Attributes,
                    target: self.index,
                    added_nodes: vec![],
                    removed_nodes: vec![],
                    previous_sibling: None,
                    next_sibling: None,
                    attribute_name: Some("style".to_string()),
                    old_value: None, // We could compute this but it's expensive
                },
            );
        }

        self.mark_mutation();
    }

    #[qjs(rename = "getPropertyValue")]
    pub fn get_property_value(&self, property: String) -> String {
        let map = self.parse_style();
        map.get(&property).cloned().unwrap_or_default()
    }

    #[qjs(rename = "removeProperty")]
    pub fn remove_property(&self, property: String) -> String {
        let mut map = self.parse_style();
        let property_value = map.remove(&property).unwrap_or_default();
        self.update_style_attribute(&map);

        if let Ok(dom) = self.dom.lock() {
            dom.notify_mutation(
                self.index,
                crate::ace::engine::dom::MutationRecord {
                    type_: crate::ace::engine::dom::MutationType::Attributes,
                    target: self.index,
                    added_nodes: vec![],
                    removed_nodes: vec![],
                    previous_sibling: None,
                    next_sibling: None,
                    attribute_name: Some("style".to_string()),
                    old_value: None,
                },
            );
        }

        self.mark_mutation();
        val
    }

    #[qjs(get, rename = "cssText")]
    pub fn get_css_text(&self) -> String {
        if let Ok(dom) = self.dom.lock() {
            if let Some(node) = dom.get_node(self.index) {
                if let AceNodeType::Element(element) = &node.node_type {
                    return element.attributes.get("style").cloned().unwrap_or_default();
                }
            }
        }
        String::new()
    }

    #[qjs(set, rename = "cssText")]
    pub fn set_css_text(&self, value: String) {
        if let Ok(mut dom) = self.dom.lock() {
            if let Some(node) = dom.nodes.get_mut(self.index) {
                if let AceNodeType::Element(element) = &mut node.node_type {
                    element.attributes.insert("style".to_string(), value);
                }
            }
        }
        self.mark_mutation();
    }

    // Common CSS property helpers
    #[qjs(get, rename = "color")]
    pub fn get_color(&self) -> String {
        self.get_property_value("color".to_string())
    }

    #[qjs(set, rename = "color")]
    pub fn set_color(&self, value: String) {
        self.set_property("color".to_string(), value);
    }

    #[qjs(get, rename = "backgroundColor")]
    pub fn get_background_color(&self) -> String {
        self.get_property_value("background-color".to_string())
    }

    #[qjs(set, rename = "backgroundColor")]
    pub fn set_background_color(&self, value: String) {
        self.set_property("background-color".to_string(), value);
    }

    #[qjs(get, rename = "fontSize")]
    pub fn get_font_size(&self) -> String {
        self.get_property_value("font-size".to_string())
    }

    #[qjs(set, rename = "fontSize")]
    pub fn set_font_size(&self, value: String) {
        self.set_property("font-size".to_string(), value);
    }

    #[qjs(get, rename = "width")]
    pub fn get_width(&self) -> String {
        self.get_property_value("width".to_string())
    }

    #[qjs(set, rename = "width")]
    pub fn set_width(&self, value: String) {
        self.set_property("width".to_string(), value);
    }

    #[qjs(get, rename = "height")]
    pub fn get_height(&self) -> String {
        self.get_property_value("height".to_string())
    }

    #[qjs(set, rename = "height")]
    pub fn set_height(&self, value: String) {
        self.set_property("height".to_string(), value);
    }

    #[qjs(get, rename = "display")]
    pub fn get_display(&self) -> String {
        self.get_property_value("display".to_string())
    }

    #[qjs(set, rename = "display")]
    pub fn set_display(&self, value: String) {
        self.set_property("display".to_string(), value);
    }

    #[qjs(get)]
    pub fn length(&self) -> usize {
        self.parse_style().len()
    }

    pub fn item(&self, index: usize) -> Option<String> {
        self.parse_style().keys().nth(index).cloned()
    }
}
