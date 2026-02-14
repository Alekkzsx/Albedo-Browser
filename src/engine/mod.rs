pub mod dom;
pub mod parser;
pub mod style;
pub mod layout;

#[derive(Debug, Clone)]
pub struct AceEngine {
    pub dom: Option<dom::DomTree>,
    pub stylesheet: style::Stylesheet,
}

#[derive(Debug, Clone)]
pub struct Script {
    pub content: String,
    pub src: Option<String>,
}

impl AceEngine {
    pub fn new() -> Self {
        Self { 
            dom: None,
            stylesheet: style::Stylesheet::default(),
        }
    }

    pub fn load_html(&mut self, html: &str) -> Vec<Script> {
        let dom = parser::parse_html(html);
        
        // Extract <style> tags
        let mut css_text = String::new();
        Self::extract_styles(&dom.root, &mut css_text);
        
        // Extract <script> tags
        let mut scripts = Vec::new();
        Self::extract_scripts(&dom.root, &mut scripts);
        
        self.stylesheet = style::Stylesheet::parse(&css_text);
        self.dom = Some(dom);
        
        scripts
    }

    fn extract_styles(node: &kuchiki::NodeRef, output: &mut String) {
        if let Some(element) = node.as_element() {
            if element.name.local.to_string() == "style" {
                for child in node.children() {
                    if let Some(text) = child.as_text() {
                        output.push_str(&text.borrow());
                        output.push('\n');
                    }
                }
            }
        }
        for child in node.children() {
            Self::extract_styles(&child, output);
        }
    }

    fn extract_scripts(node: &kuchiki::NodeRef, output: &mut Vec<Script>) {
        if let Some(element) = node.as_element() {
            if element.name.local.to_string() == "script" {
                let src = element.attributes.borrow().get("src").map(|s| s.to_string());
                let mut content = String::new();
                
                for child in node.children() {
                    if let Some(text) = child.as_text() {
                        content.push_str(&text.borrow());
                    }
                }
                
                output.push(Script { content, src });
            }
        }
        for child in node.children() {
            Self::extract_scripts(&child, output);
        }
    }

    pub fn render(&self) -> String {
        if let Some(dom) = &self.dom {
            // Updated to pass stylesheet if layout engine supports it
            if let Some(mut layout_root) = layout::build_layout_tree(&dom.root, &self.stylesheet) {
                let mut viewport = layout::Dimensions::default();
                viewport.content.width = 800.0;
                layout_root.layout(viewport);
                format!("LAYOUT TREE:\n{}", layout_root.render_debug(0))
            } else {
                "Error: Could not build layout tree.".to_string()
            }
        } else {
            "No content loaded.".to_string()
        }
    }

    pub fn render_visual(&self) -> Vec<layout::RenderPrimitive> {
        if let Some(dom) = &self.dom {
            if let Some(mut layout_root) = layout::build_layout_tree(&dom.root, &self.stylesheet) {
                 let mut viewport = layout::Dimensions::default();
                 viewport.content.width = 800.0;
                 layout_root.layout(viewport);
                 return layout_root.flatten();
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests_scripts;
