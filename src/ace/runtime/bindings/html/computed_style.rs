use crate::ace::engine::dom::AceDOM;
use crate::ace::engine::style::Stylesheet;
use std::sync::{Arc, Mutex};

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
        let stylesheet_lock = self.stylesheet.lock().unwrap();
        let dom_lock = self.dom.lock().unwrap();
        tracing::debug!(node_idx = self.node_idx, property = %property, "get_property_value");
        if let Some(node) = dom_lock.get_node(self.node_idx) {
            tracing::debug!(?node, "Node found");
        }
        tracing::debug!(rule_count = stylesheet_lock.rules.len(), "Author rules count");
        for (i, rule) in stylesheet_lock.rules.iter().enumerate() {
            tracing::debug!(index = i, ?rule, "Rule");
        }
        // stylesheet.calculate_style signature was updated in previous steps to accept &AceDOM and usize
        // For now, we don't have parent context easily available here without traversing up.
        // We'll pass None for parent_style for now (inheritance will be limited for JS query until we fix this loop).
        let root_style = stylesheet_lock.calculate_style(
            &dom_lock, 0, None, None, None, None, None, None, 0.0, 1024.0, 768.0, "light",
        );
        let style = stylesheet_lock.calculate_style(
            &dom_lock,
            self.node_idx,
            None,
            Some(&root_style),
            None,
            None,
            None,
            None,
            0.0,
            1024.0,
            768.0,
            "light",
        );

        match property.as_str() {
            "color" => format!("{:?}", style.color),
            "background-color" | "background" => format!("{:?}", style.background_color),
            "font-size" => format!("{}px", style.font_size),
            "display" => format!("{:?}", style.display).to_lowercase(),
            "width" => format!("{}px", style.width),
            "height" => format!("{}px", style.height),
            "margin-top" => format!("{}px", style.margin_top),
            "margin-right" => format!("{}px", style.margin_right),
            "margin-bottom" => format!("{}px", style.margin_bottom),
            "margin-left" => format!("{}px", style.margin_left),
            "padding-top" => format!("{}px", style.padding_top),
            "padding-right" => format!("{}px", style.padding_right),
            "padding-bottom" => format!("{}px", style.padding_bottom),
            "padding-left" => format!("{}px", style.padding_left),
            "position" => "static".to_string(), // Default
            "opacity" => "1".to_string(),
            "visibility" => "visible".to_string(),
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
