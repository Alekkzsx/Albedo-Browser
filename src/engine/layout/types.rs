use crate::engine::style::{AlbedoStyle, Color};
use kuchiki::NodeRef;
use taffy::TaffyTree;
use taffy::tree::NodeId;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

#[derive(Debug, Clone)]
pub enum BoxType {
    BlockNode,
    InlineNode,
    AnonymousBlock,
}

pub struct LayoutBox {
    pub box_type: BoxType,
    pub style: AlbedoStyle,
    pub dimensions: Dimensions,
    pub children: Vec<LayoutBox>,
    pub text_content: String,
    pub image_url: Option<String>,
    pub link_url: Option<String>,
    pub node: Option<NodeRef>,
    pub taffy_node: Option<NodeId>,
}

impl LayoutBox {
    pub fn new(box_type: BoxType, style: AlbedoStyle) -> Self {
        Self {
            box_type,
            style,
            dimensions: Dimensions::default(),
            children: Vec::new(),
            text_content: String::new(),
            image_url: None,
            link_url: None,
            node: None,
            taffy_node: None,
        }
    }

    pub fn create_taffy_node(&mut self, taffy: &mut TaffyTree) {
        let taffy_style = self.style.to_taffy_style();

        if !self.text_content.is_empty() {
            let node = taffy.new_leaf(taffy_style).expect("Failed to create Taffy leaf");
            self.taffy_node = Some(node);
        } else {
            let mut child_nodes = Vec::new();
            for child in &mut self.children {
                child.create_taffy_node(taffy);
                if let Some(node_id) = child.taffy_node {
                    child_nodes.push(node_id);
                }
            }

            let node = taffy.new_with_children(taffy_style, &child_nodes).expect("Failed to create Taffy node");
            self.taffy_node = Some(node);
        }
    }

    pub fn compute_layout(&mut self, taffy: &TaffyTree, origin: Rect) {
        if let Some(node_id) = self.taffy_node {
            let layout = taffy.layout(node_id).unwrap();
            
            self.dimensions.content.x = origin.x + layout.location.x;
            self.dimensions.content.y = origin.y + layout.location.y;
            self.dimensions.content.width = layout.size.width;
            self.dimensions.content.height = layout.size.height;

            let child_origin = Rect {
                x: self.dimensions.content.x,
                y: self.dimensions.content.y,
                width: self.dimensions.content.width,
                height: self.dimensions.content.height,
            };

            for child in &mut self.children {
                child.compute_layout(taffy, child_origin);
            }
        }
    }
}

pub struct RenderPrimitive {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Color,
    pub bg_color: Color,
    pub text: String,
    pub font_size: f32,
    pub image_url: Option<String>,
    pub link_url: Option<String>,
    pub node_ptr: usize,
    pub element_type: String, // "box", "input", "button", "img"
}
