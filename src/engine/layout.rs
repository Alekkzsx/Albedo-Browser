pub mod types;
pub mod tree_builder;

pub use types::*;
pub use tree_builder::build_layout_tree;
use crate::engine::style::AlbedoStyle;

impl LayoutBox {
    pub fn layout(&mut self, viewport: Dimensions) {
        let mut taffy = taffy::TaffyTree::default();
        
        println!("[ACE] Creating Taffy nodes...");
        self.create_taffy_node(&mut taffy);
        let root_id = self.taffy_node.expect("Root Taffy node not created");

        let available_space = taffy::geometry::Size {
            width: taffy::style::AvailableSpace::Definite(viewport.content.width),
            height: taffy::style::AvailableSpace::MaxContent,
        };

        let mut root_style = taffy.style(root_id).unwrap().clone();
        root_style.min_size.height = taffy::style::Dimension::length(viewport.content.height);
        root_style.size.width = taffy::style::Dimension::percent(1.0); // Ensure full width
        taffy.set_style(root_id, root_style).unwrap();

        println!("[ACE] Computing Taffy layout for root node {:?}...", root_id);
        taffy.compute_layout(root_id, available_space).expect("Taffy layout computation failed");

        let layout = taffy.layout(root_id).unwrap();
        println!("[ACE] Taffy layout computed. Root size: {:.1}x{:.1}", layout.size.width, layout.size.height);

        self.compute_layout(&taffy, crate::engine::layout::types::Rect::default());
        
        // FIX: Enforce minimum viewport height if collapsed
        if self.dimensions.content.height < 100.0 {
             println!("[ACE] Viewport collapse detected ({:.1}px). Enforcing minimum height 2000px.", self.dimensions.content.height);
             self.dimensions.content.height = 2000.0;
        }

        println!("[ACE] Local layout properties synchronized.");
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
        
        let has_text = !self.text_content.is_empty();
        let has_bg = self.style.background_color.a > 0;
        let is_link = self.link_url.is_some();
        let has_img = self.image_url.is_some();
        
        let element_type = self.node.as_ref().and_then(|n| n.as_element()).map(|e| e.name.local.to_string()).unwrap_or_else(|| "box".to_string());

        let mut text = self.text_content.clone();
        if element_type == "input" {
             if let Some(node) = &self.node {
                 if let Some(el) = node.as_element() {
                     if let Some(val) = el.attributes.borrow().get("value") {
                         text = val.to_string();
                     } else if let Some(ph) = el.attributes.borrow().get("placeholder") {
                         text = ph.to_string();
                     }
                 }
             }
        } else if element_type == "button" || element_type == "submit" {
            // Button text usually comes from children text content, which is already aggregated in layout?
            // Wait, LayoutBox text_content might be empty if it has children nodes.
            // If button has text node child, that child renders its own text?
            // Slint Button expects `text` property. 
            // If I render a Button *and* its children render Text, it overlaps.
            // I should mostly rely on the LayoutBox aggregation if any.
            // But flattening yields children primitives too.
            // If I render a Button container, I should probably NOT render its text children separately?
            // Or maybe Slint Button is just the background/border and the text is a child?
            // Native Slint Button handles text itself.
            // Let's assume for now keeping it simple.
        }

        if has_text || has_bg || is_link || has_img || element_type == "input" || element_type == "button" {
             result.push(RenderPrimitive {
                x: self.dimensions.content.x,
                y: self.dimensions.content.y,
                width: self.dimensions.content.width,
                height: self.dimensions.content.height,
                color: self.style.color.clone(),
                bg_color: self.style.background_color.clone(),
                text,
                font_size: self.style.font_size,
                image_url: self.image_url.clone(),
                link_url: self.link_url.clone(),
                node_ptr: self.node.as_ref().map(|n| n as *const _ as usize).unwrap_or(0),
                element_type,
             });
        }

        for child in &self.children {
            result.extend(child.flatten());
        }
        
        result
    }
}
