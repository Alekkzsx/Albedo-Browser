pub mod types;
pub mod flex;
pub mod block;
pub mod tree_builder;

pub use types::*;
pub use tree_builder::build_layout_tree;
use crate::engine::style::DisplayMode;

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
                node_ptr: self.node.as_ref().map(|n| &*n as *const _ as usize).unwrap_or(0),
             });
        }

        for child in &self.children {
            result.extend(child.flatten());
        }
        
        result
    }
}


