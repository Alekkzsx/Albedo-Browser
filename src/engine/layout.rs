use crate::engine::style::{Style, DisplayMode, FlexDirection, JustifyContent, AlignItems};

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




    pub fn layout(&mut self, containing_block: Dimensions) {
        match self.style.display {
            DisplayMode::Flex => self.layout_flex(containing_block),
            DisplayMode::Block => self.layout_block(containing_block),
            DisplayMode::Inline => {
                 // Inline elements inside a block context are usually wrapped in anonymous blocks or handled by parent
                 // For now, if we are here, treat as block-like for sizing but maybe unexpected
                 self.layout_block(containing_block);
            },
            _ => self.layout_block(containing_block),
        }
    }

    fn layout_flex(&mut self, containing_block: Dimensions) {
        // 1. Calculate width (Block-like behavior for container)
        self.calculate_block_width(containing_block);
        self.calculate_block_position(containing_block);

        // 2. Layout Children
        // Flexbox is complex. For this MVP:
        // - We assume single line (no wrap)
        // - We support Row and Column
        
        let direction = self.style.flex_direction;
        let mut main_axis_pos = 0.0;
        let mut cross_axis_pos = 0.0; // For now start at 0
        let mut max_cross_size: f32 = 0.0;

        let container_width = self.dimensions.content.width;
        
        // Pass 1: Measure children
        for child in &mut self.children {
            // Child needs to know its parent constraints
            // We give it the full container dimensions essentially
            child.layout(self.dimensions);
            
            // Should add margin handling here
        }

        // Pass 2: Position children along main axis
        // Handle Justify Content
        let total_main_size: f32 = self.children.iter().map(|c| {
            if direction == FlexDirection::Row { c.dimensions.content.width } else { c.dimensions.content.height }
        }).sum();

        let available_space = if direction == FlexDirection::Row { self.dimensions.content.width } else { self.dimensions.content.height }; // This might be 0 if height is auto
        let remaining_space = available_space - total_main_size;
        
        let spacing = match self.style.justify_content {
            JustifyContent::FlexStart => 0.0,
            JustifyContent::Center => remaining_space / 2.0,
            JustifyContent::FlexEnd => remaining_space,
            JustifyContent::SpaceBetween => if self.children.len() > 1 { remaining_space / (self.children.len() as f32 - 1.0) } else { 0.0 },
            JustifyContent::SpaceAround => if !self.children.is_empty() { remaining_space / (self.children.len() as f32) } else { 0.0 },
        };
        
        // Initial offset
        let mut current_main_pos = match self.style.justify_content {
            JustifyContent::Center | JustifyContent::FlexEnd => spacing,
            JustifyContent::SpaceAround => spacing / 2.0,
            _ => 0.0,
        };

        let step = match self.style.justify_content {
             JustifyContent::SpaceBetween => if self.children.len() > 1 { spacing } else { 0.0 },
             JustifyContent::SpaceAround => spacing,
             _ => 0.0 // For others, spacing is applied as offset or not applicable per item
        };
        
        // If JustifyContent is Center/End, 'step' is usually 0, 'spacing' is the initial offset. 
        // Logic fix:
        // Start: pos = 0
        // Center: pos = space/2
        // End: pos = space
        // SpaceBetween: pos = 0, increment = space / (n-1)
        // SpaceAround: pos = space/2, increment = space
        
        // Re-calcing specifically for loop
        current_main_pos = match self.style.justify_content {
             JustifyContent::FlexEnd => remaining_space,
             JustifyContent::Center => remaining_space / 2.0,
             JustifyContent::SpaceAround => spacing / 2.0,
             _ => 0.0
        };
        
        let item_spacing = match self.style.justify_content {
             JustifyContent::SpaceBetween => spacing,
             JustifyContent::SpaceAround => spacing,
             _ => 0.0
        };

        for child in &mut self.children {
             let (child_main_size, child_cross_size) = if direction == FlexDirection::Row { 
                 (child.dimensions.content.width, child.dimensions.content.height)
             } else {
                 (child.dimensions.content.height, child.dimensions.content.width)
             };

             if direction == FlexDirection::Row {
                 child.dimensions.content.x = self.dimensions.content.x + current_main_pos;
                 child.dimensions.content.y = self.dimensions.content.y; // AlignItems handling needed
             } else {
                 child.dimensions.content.x = self.dimensions.content.x;
                 child.dimensions.content.y = self.dimensions.content.y + current_main_pos;
             }

             // Align Items (Simple Stretch/Start) mechanism
             // If stretch and child Size is auto/0, set it to container cross size? 
             // Container cross size might be valid only if set explicitly or calculated from max content
             
             // Track max cross size for container height auto-calculation
             if child_cross_size > max_cross_size {
                 max_cross_size = child_cross_size;
             }

             current_main_pos += child_main_size + item_spacing;
        }

        // 3. Calculate Height / Cross Size
        if direction == FlexDirection::Row {
            if self.dimensions.content.height == 0.0 {
                self.dimensions.content.height = max_cross_size;
            }
            
            // Pass 3: Align Items now that we know container height
             for child in &mut self.children {
                 match self.style.align_items {
                     AlignItems::Center => {
                         let child_h = child.dimensions.content.height;
                         let parent_h = self.dimensions.content.height;
                         child.dimensions.content.y = self.dimensions.content.y + (parent_h - child_h) / 2.0;
                     },
                     AlignItems::FlexEnd => {
                          let child_h = child.dimensions.content.height;
                         let parent_h = self.dimensions.content.height;
                         child.dimensions.content.y = self.dimensions.content.y + (parent_h - child_h);
                     },
                     AlignItems::Stretch => {
                         // If child height is not fixed (todo check), stretch it.
                         // For now, force stretch if style not fixed
                         if child.style.display != DisplayMode::None {
                              child.dimensions.content.height = self.dimensions.content.height;
                         }
                     }
                      _ => {} // FlexStart (default 0)
                 }
             }

        } else {
            // Column: Height is sum of children (calculated in total_main_size + gaps)
             if self.dimensions.content.height == 0.0 {
                // If we used remaining_space logic above, it relied on fixed height.
                // If auto height, remaining space is 0 initially.
                // We typically just sum them up.
                // Re-set height to actual content used if it was 0
                self.dimensions.content.height = current_main_pos - item_spacing; // Removing last gap ? No, current_main_pos includes sizes.
                 // Actually current_main_pos includes all items + spacing. 
                 // If space-between/around was used, it implies we had a height.
                 
                 // If height was 0, total_main_size is what we used implicitly if we assumed available=total.
                 // Let's simplified assumption: if height is auto, we just stack them tightly (justify-content usually irrelevant for auto height unless min-height set)
                 if self.dimensions.content.height == 0.0 {
                     self.dimensions.content.height = total_main_size; 
                     // TODO: Add support for gap property
                 }
            }
             // Align Items for Column (Cross Axis = Width)
              for child in &mut self.children {
                 match self.style.align_items {
                     AlignItems::Center => {
                         let child_w = child.dimensions.content.width;
                         let parent_w = self.dimensions.content.width;
                         child.dimensions.content.x = self.dimensions.content.x + (parent_w - child_w) / 2.0;
                     },
                     AlignItems::FlexEnd => {
                          let child_w = child.dimensions.content.width;
                         let parent_w = self.dimensions.content.width;
                         child.dimensions.content.x = self.dimensions.content.x + (parent_w - child_w);
                     },
                      AlignItems::Stretch => {
                             child.dimensions.content.width = self.dimensions.content.width;
                     }
                      _ => {}
                 }
             }
        }
    }

    fn layout_block(&mut self, containing_block: Dimensions) {
        // Child width depends on parent width
        self.calculate_block_width(containing_block);

        // Position the box inside its parent
        self.calculate_block_position(containing_block);

        // Recursively layout children
        self.layout_block_children();

        // Parent height depends on children height
        self.calculate_block_height();
    }

    fn calculate_block_width(&mut self, _containing_block: Dimensions) {
        // For now, take 100% of parent width minus margins
        self.dimensions.content.width = _containing_block.content.width;
    }

    fn calculate_block_position(&mut self, _containing_block: Dimensions) {
        // Simplified: push everything down vertically
        self.dimensions.content.x = _containing_block.content.x;
        self.dimensions.content.y = _containing_block.content.y + _containing_block.content.height;
    }

    fn layout_block_children(&mut self) {
        let mut total_height = 0.0;
        for child in &mut self.children {
            child.layout(self.dimensions.clone());
            // Stack children vertically
            child.dimensions.content.y = self.dimensions.content.y + total_height;
            total_height += child.dimensions.content.height;
        }
        self.dimensions.content.height = total_height;
    }

    fn calculate_block_height(&mut self) {
        // If height is specified in style, use it, otherwise use total_height calculated above
        // For now, we already calculated it in layout_block_children
        if self.dimensions.content.height == 0.0 {
             self.dimensions.content.height = 20.0; // Minimal height for non-empty blocks
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
}

pub fn build_layout_tree(node: &kuchiki::NodeRef) -> Option<LayoutBox> {
    use kuchiki::NodeData;
    
    match node.data() {
        NodeData::Element(element) => {
            let tag = element.name.local.to_string();
            let mut style = Style::default_for_tag(&tag);
            
            if let Some(s) = element.attributes.borrow().get("style") {
                let inline = Style::parse_inline_style(s);
                // Merge inline into default (simplistic)
                style.color = inline.color;
                if inline.font_size != 16.0 { style.font_size = inline.font_size; }
            }

            if style.display == DisplayMode::None { return None; }

            let box_type = match style.display {
                DisplayMode::Block => BoxType::BlockNode,
                DisplayMode::Inline => BoxType::InlineNode,
                _ => BoxType::BlockNode,
            };

            let mut layout_node = LayoutBox::new(box_type, style);
            
            if tag == "img" {
                if let Some(src) = element.attributes.borrow().get("src") {
                    layout_node.image_url = Some(src.to_string());
                }
                
                // Parse optional width/height attributes
                 if let Some(w) = element.attributes.borrow().get("width") {
                    if let Ok(val) = w.parse::<f32>() {
                        layout_node.dimensions.content.width = val;
                        // If provided, assume it's fixed
                    }
                }
                if let Some(h) = element.attributes.borrow().get("height") {
                    if let Ok(val) = h.parse::<f32>() {
                        layout_node.dimensions.content.height = val;
                    }
                }
                // If no dimensions provided, we might default to something non-zero
                // for the placeholder until loaded?
                if layout_node.dimensions.content.width == 0.0 { layout_node.dimensions.content.width = 100.0; }
                if layout_node.dimensions.content.height == 0.0 { layout_node.dimensions.content.height = 100.0; }
            } else if tag == "a" {
                if let Some(href) = element.attributes.borrow().get("href") {
                    layout_node.link_url = Some(href.to_string());
                    // Links should look clickable
                    if layout_node.style.color == "#333333" { // Default color
                        layout_node.style.color = "blue".to_string();
                        // TODO: Underline
                    }
                }
            }

            for child in node.children() {
                if let Some(child_box) = build_layout_tree(&child) {
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
        _ => {
            None
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

impl LayoutBox {
    pub fn flatten(&self) -> Vec<RenderPrimitive> {
        let mut result = Vec::new();
        
        // Add current box as a primitive if it has visual aspects or text or image or is a link!
        // We want to emit a link primitive even if it's transparent, so it covers the area.
        
        let has_visual = !self.text_content.is_empty() || self.image_url.is_some() || self.style.background_color != "transparent";
        let is_link = self.link_url.is_some();
        
        if has_visual || is_link {
             // If it's just a link wrapper (like <a><img></a>), we might emit a transparent box with link_url.
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
            let mut children_primitives = child.flatten();
            // If parent is a link, should children inherit the link?
            // Usually, the parent container handles the click.
            // But if children are on top, they might block it unless click-through.
            // In Slint, we can just put TouchArea on parent.
            result.extend(children_primitives);
        }
        
        result
    }
}
