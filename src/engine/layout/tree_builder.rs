use crate::engine::layout::types::*;
use crate::engine::style::{AlbedoStyle, DisplayMode};
use kuchiki::NodeRef;
use kuchiki::traits::*;

pub fn build_layout_tree(node: &NodeRef, stylesheet: &crate::engine::style::Stylesheet, depth: usize) -> Option<LayoutBox> {
    use kuchiki::NodeData;
    
    if depth > 500 {
        println!("[ACE] CRITICAL: Max recursion depth (500) reached in layout tree builder!");
        return None;
    }

    match node.data() {
        NodeData::Element(element) => {
            let tag = element.name.local.to_string();
            let mut style = crate::engine::style::resolve_style(node, element, stylesheet);

            if style.display == DisplayMode::None { return None; }

            if tag == "img" {
                if let Some(w) = element.attributes.borrow().get("width") {
                    if let Ok(val) = w.parse::<f32>() {
                        style.width = crate::engine::style::types::Length::Px(val);
                    }
                }
                if let Some(h) = element.attributes.borrow().get("height") {
                    if let Ok(val) = h.parse::<f32>() {
                        style.height = crate::engine::style::types::Length::Px(val);
                    }
                }
            }
            
            // Log element processing
            if depth < 5 {
                println!("[ACE] {:indent$}Building: <{}>", "", tag, indent = depth * 2);
            }

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
            } else if tag == "a" {
                if let Some(href) = element.attributes.borrow().get("href") {
                    layout_node.link_url = Some(href.to_string());
                }
            }

            for child in node.children() {
                if let Some(child_box) = build_layout_tree(&child, stylesheet, depth + 1) {
                    layout_node.children.push(child_box);
                }
            }
            Some(layout_node)
        },
        NodeData::Text(text) => {
            let content = text.borrow();
            let trimmed = content.trim();
            if trimmed.is_empty() { return None; }
            
            let mut style = AlbedoStyle::default();
            style.display = DisplayMode::Inline;
            
            let mut layout_node = LayoutBox::new(BoxType::InlineNode, style);
            layout_node.node = Some(node.clone());
            layout_node.text_content = trimmed.to_string();
            Some(layout_node)
        },
        _ => {
            for child in node.children() {
                if let Some(box_node) = build_layout_tree(&child, stylesheet, depth + 1) {
                    return Some(box_node);
                }
            }
            None
        }
    }
}
