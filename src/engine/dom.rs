use kuchiki::NodeData;
use kuchiki::traits::*;

#[derive(Debug, Clone)]
pub struct DomTree {
    pub root: kuchiki::NodeRef,
}

impl DomTree {
    pub fn new(root: kuchiki::NodeRef) -> Self {
        Self { root }
    }
    
    pub fn find_by_id(&self, id: &str) -> Option<kuchiki::NodeRef> {
        if let Ok(mut match_iter) = self.root.select(&format!("#{}", id)) {
            match_iter.next().map(|m| m.as_node().clone())
        } else {
            None
        }
    }
    
    pub fn text_contents(&self) -> String {
        self.root.text_contents()
    }

    pub fn render_text(&self) -> String {
        let mut output = String::new();
        Self::walk_tree(&self.root, &mut output);
        output
    }

    fn walk_tree(node: &kuchiki::NodeRef, output: &mut String) {
        // Handle element data
        if let NodeData::Element(element) = node.data() {
            let tag_name = element.name.local.to_string();
            
            // Basic User Agent Styles
            match tag_name.as_str() {
                "h1" => output.push_str("\n\n [STYLE size=32] "),
                "h2" => output.push_str("\n\n [STYLE size=24] "),
                "p" => output.push_str("\n"),
                "li" => output.push_str("\n • "),
                _ => {}
            }

            // Parse inline style attribute
            if let Some(style_attr) = element.attributes.borrow().get("style") {
                 let style = crate::engine::style::Style::parse_inline_style(style_attr);
                 if style.color != "#333333" {
                     output.push_str(&format!(" [COLOR={}] ", style.color));
                 }
            }
        }

        // Handle text nodes
        if let NodeData::Text(text) = node.data() {
            let content = text.borrow();
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                output.push_str(trimmed);
                output.push(' ');
            }
        }

        // Recurse children
        for child in node.children() {
            Self::walk_tree(&child, output);
        }

        // Post-processing for elements (closing tags basically)
        if let NodeData::Element(element) = node.data() {
            let tag_name = element.name.local.to_string();
            match tag_name.as_str() {
                "h1" | "h2" | "p" | "div" => output.push('\n'),
                _ => {}
            }
        }
    }
}
