use rquickjs::Result;
use kuchiki::NodeRef;
use kuchiki::traits::*;
use std::collections::HashMap;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct CssStyleDeclaration {
    #[qjs(skip_trace)]
    pub node: NodeRef,
}

impl CssStyleDeclaration {
    fn parse_style(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(data) = self.node.as_element() {
            if let Some(style_attr) = data.attributes.borrow().get("style") {
                for decl in style_attr.split(';') {
                    let decl = decl.trim();
                    if decl.is_empty() { continue; }
                    if let Some((key, val)) = decl.split_once(':') {
                        map.insert(key.trim().to_string(), val.trim().to_string());
                    }
                }
            }
        }
        map
    }

    fn update_style_attribute(&self, map: &HashMap<String, String>) {
        if let Some(data) = self.node.as_element() {
            let mut attrs = data.attributes.borrow_mut();
            if map.is_empty() {
                attrs.remove("style");
            } else {
                let mut style_str = String::new();
                for (key, val) in map {
                    style_str.push_str(key);
                    style_str.push_str(": ");
                    style_str.push_str(val);
                    style_str.push_str("; ");
                }
                attrs.insert("style", style_str.trim().to_string());
            }
        }
    }
}

#[rquickjs::methods]
impl CssStyleDeclaration {
    #[qjs(rename = "setProperty")]
    pub fn set_property(&self, property: String, value: String) {
        let mut map = self.parse_style();
        map.insert(property, value);
        self.update_style_attribute(&map);
    }

    #[qjs(rename = "getPropertyValue")]
    pub fn get_property_value(&self, property: String) -> String {
        let map = self.parse_style();
        map.get(&property).cloned().unwrap_or_default()
    }

    #[qjs(rename = "removeProperty")]
    pub fn remove_property(&self, property: String) -> String {
        let mut map = self.parse_style();
        let val = map.remove(&property).unwrap_or_default();
        self.update_style_attribute(&map);
        val
    }
    
    #[qjs(get, rename = "cssText")]
    pub fn get_css_text(&self) -> String {
        if let Some(data) = self.node.as_element() {
            if let Some(style_attr) = data.attributes.borrow().get("style") {
                return style_attr.to_string();
            }
        }
        String::new()
    }

    #[qjs(set, rename = "cssText")]
    pub fn set_css_text(&self, value: String) {
        if let Some(data) = self.node.as_element() {
            data.attributes.borrow_mut().insert("style", value);
        }
    }
}
