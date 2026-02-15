use crate::engine::dom::AceDOM;
use std::sync::{Arc, Mutex};
use crate::engine::style::Stylesheet;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct ComputedCSSStyleDeclaration {
    #[qjs(skip_trace)]
    pub dom: Arc<Mutex<AceDOM>>,
    #[qjs(skip_trace)]
    pub node_idx: usize,
    #[qjs(skip_trace)]
    pub stylesheet: Arc<Mutex<Stylesheet>>,
}

#[rquickjs::methods]
impl ComputedCSSStyleDeclaration {
    #[qjs(rename = "getPropertyValue")]
    pub fn get_property_value(&self, property: String) -> String {
        let stylesheet = self.stylesheet.lock().unwrap();
        let dom = self.dom.lock().unwrap();
        let stylesheet_lock = self.stylesheet.lock().unwrap();
        let dom_lock = self.dom.lock().unwrap();
        // stylesheet.calculate_style signature was updated in previous steps to accept &AceDOM and usize
        // For now, we don't have parent context easily available here without traversing up.
        // We'll pass None for parent_style for now (inheritance will be limited for JS query until we fix this loop).
        let root_style = stylesheet_lock.calculate_style(&dom_lock, 0, None, None, None, None); // Using 0 as root element index usually
        let style = stylesheet_lock.calculate_style(&dom_lock, self.node_idx, None, Some(&root_style), None, None);
        
        match property.as_str() {
            "color" => format!("{:?}", style.color), // Todo: Implement proper Display or to_string for CssColor
            "background-color" | "background" => format!("{:?}", style.background_color),
            "font-size" => style.font_size.to_string(),
            "display" => format!("{:?}", style.display), // Todo: to_lowercase()
            "width" => style.width.to_string(),
            "height" => style.height.to_string(),
            "margin-top" => style.margin_top.to_string(),
            "margin-right" => style.margin_right.to_string(),
            "margin-bottom" => style.margin_bottom.to_string(),
            "margin-left" => style.margin_left.to_string(),
            "padding-top" => style.padding_top.to_string(),
            "padding-right" => style.padding_right.to_string(),
            "padding-bottom" => style.padding_bottom.to_string(),
            "padding-left" => style.padding_left.to_string(),
            _ => String::new(),
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
