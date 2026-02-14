pub mod types;
pub mod flex;

pub use types::*;
use crate::engine::style::{Style, DisplayMode};
use kuchiki::NodeRef;

impl LayoutBox {
    pub fn layout(&mut self, containing_block: Dimensions) {
        match self.style.display {
            DisplayMode::Flex => flex::layout_flex(self, containing_block),
            DisplayMode::Block => self.layout_block(containing_block),
            DisplayMode::Inline => {
                 self.layout_block(containing_block);
            },
            _ => self.layout_block(containing_block),
        }
    }

    fn layout_block(&mut self, containing_block: Dimensions) {
        self.calculate_block_width(containing_block);
        self.calculate_block_position(containing_block);
        self.layout_block_children();
        self.calculate_block_height();
    }

    pub fn calculate_block_width(&mut self, containing_block: Dimensions) {
        self.dimensions.content.width = containing_block.content.width;
    }

    pub fn calculate_block_position(&mut self, containing_block: Dimensions) {
        self.dimensions.content.x = containing_block.content.x;
        self.dimensions.content.y = containing_block.content.y + containing_block.content.height;
    }

    fn layout_block_children(&mut self) {
        let mut total_height = 0.0;
        for child in &mut self.children {
            child.layout(self.dimensions.clone());
            child.dimensions.content.y = self.dimensions.content.y + total_height;
            total_height += child.dimensions.content.height;
        }
        self.dimensions.content.height = total_height;
    }

    fn calculate_block_height(&mut self) {
        if self.dimensions.content.height == 0.0 {
             self.dimensions.content.height = 20.0;
        }
    }

    pub fn render_debug(&self, indent: usize) -> String {
        let mut output = format!(
            "{:indent$}[{:?}] at ({:.1}, {:.1}) size {:.1}x{:.1}\n",
            "",
            self.box_type,
            self.dimensions.content.x,
            self.dimensions.content.y,
            self.dimensions.content.width,
            self.dimensions.content.height,
            indent = indent * 2
        );
        for child in &self.children {
            output.push_str(&child.render_debug(indent + 1));
        }
        output
    }

    pub fn flatten(&self) -> Vec<RenderPrimitive> {
        let mut result = Vec::new();
        let has_visual = !self.text_content.is_empty() || self.image_url.is_some() || self.style.background_color != "transparent";
        let is_link = self.link_url.is_some();
        
        if has_visual || is_link {
             result.push(RenderPrimitive {
                x: self.dimensions.content.x,
                y: self.dimensions.content.y,
                width: self.dimensions.content.width,
                height: self.dimensions.content.height,
                color: if has_visual { 
                    if !self.text_content.is_empty() { self.style.color.clone() } else { self.style.background_color.clone() }
                } else { "transparent".to_string() },
                text: self.text_content.clone(),
                font_size: self.style.font_size,
                image_url: self.image_url.clone(),
                link_url: self.link_url.clone(),
             });
        }

        for child in &self.children {
            result.extend(child.flatten());
        }
        
        result
    }
}

pub fn build_layout_tree(node: &NodeRef, stylesheet: &crate::engine::style::Stylesheet) -> Option<LayoutBox> {
    use kuchiki::NodeData;
    
    match node.data() {
        NodeData::Element(element) => {
            let tag = element.name.local.to_string();
            let style = resolve_style(node, element, stylesheet);

            if style.display == DisplayMode::None { return None; }

            let box_type = match style.display {
                DisplayMode::Block => BoxType::BlockNode,
                DisplayMode::Inline => BoxType::InlineNode,
                DisplayMode::Flex => BoxType::BlockNode,
                _ => BoxType::BlockNode,
            };

            let mut layout_node = LayoutBox::new(box_type, style);
            
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
            layout_node.text_content = trimmed.to_string();
            Some(layout_node)
        },
        _ => None
    }
}

fn resolve_style(_node: &NodeRef, element: &kuchiki::ElementData, stylesheet: &crate::engine::style::Stylesheet) -> Style {
    let tag = element.name.local.to_string();
    let mut style = Style::default_for_tag(&tag);
    
    let id = element.attributes.borrow().get("id").unwrap_or_default().to_string();
    let class_attr = element.attributes.borrow().get("class").unwrap_or_default().to_string();
    let classes: Vec<String> = class_attr.split_whitespace().map(|s| s.to_string()).collect();

    let mut matched_rules = Vec::new();
    for rule in &stylesheet.rules {
        for selector in &rule.selectors {
            let matches = match selector {
                crate::engine::style::Selector::Tag(t) => t == &tag,
                crate::engine::style::Selector::Class(c) => classes.contains(c),
                crate::engine::style::Selector::Id(i) => i == &id,
                crate::engine::style::Selector::Universal => true,
            };
            
            if matches {
                matched_rules.push((selector.specificity(), rule));
            }
        }
    }
    
    matched_rules.sort_by_key(|(spec, _)| *spec);
    
    for (_, rule) in matched_rules {
        for decl in &rule.declarations {
            style.apply_declaration(decl);
        }
    }
    
    if let Some(s) = element.attributes.borrow().get("style") {
        for decl_str in s.split(';') {
             let parts: Vec<&str> = decl_str.split(':').collect();
             if parts.len() == 2 {
                 let name = parts[0].trim().to_string();
                 let value = parts[1].trim().to_string();
                 style.apply_declaration(&crate::engine::style::Declaration { name, value });
             }
        }
    }
    
    style
}
