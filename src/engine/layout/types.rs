use crate::engine::style::Style;

#[derive(Default, Debug, Clone, Copy)]
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
    pub style: Style,
    pub dimensions: Dimensions,
    pub children: Vec<LayoutBox>,
    pub text_content: String,
    pub image_url: Option<String>,
    pub link_url: Option<String>,
}

impl LayoutBox {
    pub fn new(box_type: BoxType, style: Style) -> Self {
        Self {
            box_type,
            style,
            dimensions: Dimensions::default(),
            children: Vec::new(),
            text_content: String::new(),
            image_url: None,
            link_url: None,
        }
    }
}

pub struct RenderPrimitive {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: String,
    pub text: String,
    pub font_size: f32,
    pub image_url: Option<String>,
    pub link_url: Option<String>,
}
