use rquickjs::Result;
use kuchiki::NodeRef;
use kuchiki::traits::*;
use std::collections::HashMap;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct CssStyleDeclaration {
    #[qjs(skip_trace)]
    pub node: NodeRef,
    #[qjs(skip_trace)]
    pub mutations: std::sync::Arc<std::sync::Mutex<bool>>,
    #[qjs(skip_trace)]
    pub stylesheet_dirty: std::sync::Arc<std::sync::Mutex<bool>>,
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
    fn mark_mutation(&self) {
        if let Ok(mut m) = self.mutations.lock() {
            *m = true;
        }
    }

    #[qjs(rename = "setProperty")]
    pub fn set_property(&self, property: String, value: String) {
        let mut map = self.parse_style();
        map.insert(property, value);
        self.update_style_attribute(&map);
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
        let val = map.remove(&property).unwrap_or_default();
        self.update_style_attribute(&map);
        self.mark_mutation();
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
            self.mark_mutation();
        }
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
}
