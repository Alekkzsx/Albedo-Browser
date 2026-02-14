use kuchiki::NodeRef;
use std::sync::{Arc, Mutex};
use crate::engine::style::{Stylesheet, resolve_style};

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct ComputedCSSStyleDeclaration {
    #[qjs(skip_trace)]
    pub node: NodeRef,
    #[qjs(skip_trace)]
    pub stylesheet: Arc<Mutex<Stylesheet>>,
}

#[rquickjs::methods]
impl ComputedCSSStyleDeclaration {
    #[qjs(rename = "getPropertyValue")]
    pub fn get_property_value(&self, property: String) -> String {
        if let Some(element) = self.node.as_element() {
            let stylesheet = self.stylesheet.lock().unwrap();
            let style = resolve_style(&self.node, element, &stylesheet);
            style.get(&property)
        } else {
            String::new()
        }
    }

    // Common CSS property helpers
    #[qjs(get, rename = "color")]
    pub fn get_color(&self) -> String {
        self.get_property_value("color".to_string())
    }

    #[qjs(get, rename = "backgroundColor")]
    pub fn get_background_color(&self) -> String {
        self.get_property_value("background-color".to_string())
    }

    #[qjs(get, rename = "fontSize")]
    pub fn get_font_size(&self) -> String {
        self.get_property_value("font-size".to_string())
    }

    #[qjs(get, rename = "width")]
    pub fn get_width(&self) -> String {
        self.get_property_value("width".to_string())
    }

    #[qjs(get, rename = "height")]
    pub fn get_height(&self) -> String {
        self.get_property_value("height".to_string())
    }

    #[qjs(get, rename = "display")]
    pub fn get_display(&self) -> String {
        self.get_property_value("display".to_string())
    }
}
