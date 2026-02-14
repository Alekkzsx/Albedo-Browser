use kuchiki::NodeRef;
use kuchiki::traits::*;

#[derive(Clone)]
pub struct DomTree {
    pub root: NodeRef,
}

impl DomTree {
    pub fn new(root: NodeRef) -> Self {
        Self { root }
    }

    pub fn find_by_id(&self, id: &str) -> Option<NodeRef> {
        self.root.select(&format!("#{}", id)).ok()?.next().map(|m| m.as_node().clone())
    }

    pub fn find_by_ptr(&self, ptr: usize) -> Option<NodeRef> {
        self.find_recursive(&self.root, ptr)
    }

    fn find_recursive(&self, node: &NodeRef, ptr: usize) -> Option<NodeRef> {
        if node as *const _ as usize == ptr {
            return Some(node.clone());
        }
        for child in node.children() {
            if let Some(found) = self.find_recursive(&child, ptr) {
                return Some(found);
            }
        }
        None
    }
}

pub fn render_to_string(node: &NodeRef) -> String {
    let mut output = String::new();
    render_to_string_recursive(node, &mut output);
    output
}

fn render_to_string_recursive(node: &NodeRef, output: &mut String) {
    use kuchiki::NodeData;
    
    match node.data() {
        NodeData::Element(element) => {
            let tag = element.name.local.to_string();
            output.push_str(&format!("<{}>", tag));
            
             if let Some(style_attr) = element.attributes.borrow().get("style") {
                 let style = crate::engine::style::Style::parse_inline_style(style_attr);
                 if style.color != crate::engine::style::types::Color::parse("#333333") {
                     output.push_str(&format!(" [COLOR={:?}] ", style.color));
                 }
             }

            for child in node.children() {
                render_to_string_recursive(&child, output);
            }
            output.push_str(&format!("</{}>", tag));
        },
        NodeData::Text(text) => {
            output.push_str(&text.borrow());
        },
        _ => {
            for child in node.children() {
                render_to_string_recursive(&child, output);
            }
        }
    }
}
