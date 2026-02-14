use crate::engine::layout::types::*;
use crate::engine::style::{Style, DisplayMode};
use kuchiki::NodeRef;

pub fn build_layout_tree(node: &NodeRef, stylesheet: &crate::engine::style::Stylesheet) -> Option<LayoutBox> {
    use kuchiki::NodeData;
    
    match node.data() {
        NodeData::Element(element) => {
            let tag = element.name.local.to_string();
            let style = crate::engine::style::resolve_style(node, element, stylesheet);

            if style.display == DisplayMode::None { return None; }

            let box_type = match style.display {
                DisplayMode::Block => BoxType::BlockNode,
                DisplayMode::Inline => BoxType::InlineNode,
                DisplayMode::Flex => BoxType::BlockNode,
                _ => BoxType::BlockNode,
            };

            let mut layout_node = LayoutBox::new(box_type, style);
            layout_node.node = Some(node.clone());
            
            if tag == "img" {
                if let Some(src) = element.attributes.borrow().get("src") {
                    layout_node.image_url = Some(src.to_string());
                }
                 if let Some(w) = element.attributes.borrow().get("width") {
                    if let Ok(val) = w.parse::<f32>() {
                        layout_node.dimensions.content.width = val;
                    }
                }
                if let Some(h) = element.attributes.borrow().get("height") {
                    if let Ok(val) = h.parse::<f32>() {
                        layout_node.dimensions.content.height = val;
                    }
                }
                if layout_node.dimensions.content.width == 0.0 { layout_node.dimensions.content.width = 100.0; }
                if layout_node.dimensions.content.height == 0.0 { layout_node.dimensions.content.height = 100.0; }
            } else if tag == "a" {
                if let Some(href) = element.attributes.borrow().get("href") {
                    layout_node.link_url = Some(href.to_string());
                }
            }

            for child in node.children() {
                if let Some(child_box) = build_layout_tree(&child, stylesheet) {
                    layout_node.children.push(child_box);
                }
            }
            Some(layout_node)
        },
        NodeData::Text(text) => {
            let content = text.borrow();
            let trimmed = content.trim();
            if trimmed.is_empty() { return None; }
            
            let mut style = Style::new();
            style.display = DisplayMode::Inline;
            let mut layout_node = LayoutBox::new(BoxType::InlineNode, style);
            layout_node.node = Some(node.clone());
            layout_node.text_content = trimmed.to_string();
            Some(layout_node)
        },
        _ => None
    }
}
