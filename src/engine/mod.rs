use reqwest::blocking::Client;
use std::time::Duration;
use kuchiki::traits::TendrilSink;
use taffy::prelude::*;
use std::sync::{Arc, Mutex};

pub mod dom;
pub mod style;
pub mod css_values;
#[cfg(test)]
mod dom_tests;

use self::dom::{AceDOM, AceNode, AceNodeType};
use self::style::Stylesheet;

// Estrutura Visual Simplificada
#[derive(Clone, Debug)]
pub struct ACEPrimitive {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: String,
    pub text: String,
    pub font_size: f32,
    pub image_url: Option<String>,
    pub link_url: Option<String>,
    pub node_ptr: usize,
    pub element_type: String, // "text", "image", "button", "box"
}

pub struct AceEngine {
    pub current_url: String,
    pub primitives: Vec<ACEPrimitive>,
    pub dom: Option<Arc<Mutex<AceDOM>>>, 
    pub stylesheet: Arc<Mutex<Stylesheet>>,
    client: Client,
    taffy: Taffy,
    root_node: Option<Node>,
}

impl Clone for AceEngine {
    fn clone(&self) -> Self {
        Self {
            current_url: self.current_url.clone(),
            primitives: self.primitives.clone(),
            dom: self.dom.clone(),
            stylesheet: self.stylesheet.clone(),
            client: self.client.clone(),
            taffy: Taffy::new(),
            root_node: None,
        }
    }
}

impl AceEngine {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("AlbedoBrowser/0.1")
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            current_url: "albedo://start".to_string(),
            primitives: Vec::new(),
            dom: None,
            stylesheet: Arc::new(Mutex::new(Stylesheet { rules: Vec::new() })),
            client,
            taffy: Taffy::new(),
            root_node: None,
        }
    }

    pub fn load_url(&mut self, url: &str) {
        println!("ENGINE: Carregando URL: {}", url);
        self.primitives.clear();
        self.current_url = url.to_string();
        
        self.taffy = Taffy::new();
        self.root_node = None;

        if url == "albedo://start" {
            return;
        }

        match self.client.get(url).send() {
            Ok(resp) => {
                if resp.status().is_success() {
                    let html = resp.text().unwrap_or_default();
                    self.parse_html(&html);
                } else {
                    self.render_error(&format!("Erro HTTP: {}", resp.status()));
                }
            }
            Err(e) => self.render_error(&format!("Erro de Conexão: {}", e)),
        }
    }
    pub fn load_html(&mut self, html: &str) {
        self.parse_html(html);
    }

    fn parse_html(&mut self, html: &str) {
        let document = kuchiki::parse_html().one(html);
        self.dom = Some(Arc::new(Mutex::new(AceDOM::new(document)))); 
        
        self.update_stylesheet();
    }

    pub fn update_stylesheet(&mut self) {
        let dom_ptr = if let Some(dom) = &self.dom {
            dom.clone()
        } else {
            return;
        };

        {
            let dom = dom_ptr.lock().unwrap();
            let mut new_style = style::Stylesheet { rules: Vec::new() };
            // Default CSS
            let default_css = "h1 { color: #000000; font-size: 32px; } a { color: #0000ff; text-decoration: underline; } p { color: #333333; } div { display: flex; flex-direction: column; }";
            new_style.rules.extend(style::parse(default_css).rules);

            // Extract all <style> tags traversing the flat nodes list
            for node in &dom.nodes {
                if let AceNodeType::Element(el) = &node.node_type {
                    if el.tag == "style" {
                         // Collect text from children
                         let mut css_text = String::new();
                         for &child_idx in &node.children {
                             if let Some(child) = dom.get_node(child_idx) {
                                 if let AceNodeType::Text(text) = &child.node_type {
                                     css_text.push_str(text);
                                 }
                             }
                         }
                         new_style.rules.extend(style::parse(&css_text).rules);
                    }
                }
            }
            *self.stylesheet.lock().unwrap() = new_style;
        }
        
        self.recompute_layout();
    }

    pub fn recompute_layout(&mut self) {
        let dom_ptr = if let Some(dom) = &self.dom {
            dom.clone()
        } else {
            return;
        };

        {
            let dom = dom_ptr.lock().unwrap();
            let root_idx = dom.root;
            let stylesheet_ptr = self.stylesheet.clone();
            let stylesheet = stylesheet_ptr.lock().unwrap();
            
            self.taffy = Taffy::new();
            
            let display_root = root_idx; 

            // Pass &mut self.taffy explicitly
            let root_node = build_layout_tree(&mut self.taffy, display_root, &dom, &stylesheet, None);
            self.root_node = Some(root_node);

            let available_space = Size {
                width: AvailableSpace::Definite(1024.0),
                height: AvailableSpace::MaxContent,
            };
            let _ = self.taffy.compute_layout(root_node, available_space);

            self.primitives.clear();
            // Background
            self.primitives.push(ACEPrimitive {
                x: 0.0, y: 0.0, width: 2000.0, height: 8000.0,
                color: "#FFFFFF".to_string(), text: "".into(), font_size: 0.0, 
                image_url: None, link_url: None, node_ptr: 0, element_type: "box".into()
            });
            
            // Pass fields explicitly to avoid &mut self borrow conflict
            generate_display_list(
                &mut self.primitives, 
                &self.taffy, 
                display_root, 
                &dom, 
                root_node, 
                0.0, 
                0.0, 
                &stylesheet, 
                None 
            );
        }
    }

    fn render_error(&mut self, msg: &str) {
        self.primitives.push(ACEPrimitive {
            x: 20.0, y: 20.0, width: 600.0, height: 50.0,
            color: "transparent".into(),
            text: msg.to_string(),
            font_size: 20.0, image_url: None, link_url: None, node_ptr: 0, element_type: "text".into()
        });
    }

    pub fn render_visual(&self) -> Vec<ACEPrimitive> {
        self.primitives.clone()
    }
}

// Helper functions (standalone to avoid borrow checker issues)
use crate::engine::css_values::{CssLength, CssColor, CssDisplay};

fn build_layout_tree(taffy: &mut Taffy, node_idx: usize, dom: &AceDOM, stylesheet: &Stylesheet, parent_style: Option<&ComputedStyle>) -> Node {
    let node = dom.get_node(node_idx).unwrap();
    
    let mut style = Style::default();
    
    // Now we use the passed parent_style for inheritance
    let computed = stylesheet.calculate_style(dom, node_idx, parent_style);
    
    style.display = match computed.display {
        CssDisplay::None => Display::None,
        CssDisplay::Grid => Display::Grid,
        CssDisplay::Flex => Display::Flex,
        CssDisplay::Block => Display::Flex, // Taffy treats block as flex-col usually or we simulate it
        CssDisplay::InlineBlock => Display::Flex, 
        CssDisplay::Inline => Display::Flex, 
    };
    
    if let AceNodeType::Element(el) = &node.node_type {
            match el.tag.as_str() {
                "head" | "script" | "style" | "title" | "meta" | "link" => {
                    style.display = Display::None;
                },
                _ => {}
            }
    }

    style.flex_direction = FlexDirection::Column;

    // Helper to convert CssLength to Taffy LengthPercentageAuto or Dimension
    fn to_taffy_lpa(l: &CssLength) -> LengthPercentageAuto {
        match l {
            CssLength::Px(v) => LengthPercentageAuto::Points(*v),
            CssLength::Percent(v) => LengthPercentageAuto::Percent(*v / 100.0),
            CssLength::Auto => LengthPercentageAuto::Auto,
            CssLength::Zero => LengthPercentageAuto::Points(0.0),
            _ => LengthPercentageAuto::Auto, // Fallback
        }
    }
    
    fn to_taffy_lp(l: &CssLength) -> LengthPercentage {
        match l {
            CssLength::Px(v) => LengthPercentage::Points(*v),
            CssLength::Percent(v) => LengthPercentage::Percent(*v / 100.0),
            CssLength::Zero => LengthPercentage::Points(0.0),
             _ => LengthPercentage::Points(0.0), // Fallback
        }
    }

    fn to_taffy_dim(l: &CssLength) -> Dimension {
        match l {
            CssLength::Px(v) => Dimension::Points(*v),
            CssLength::Percent(v) => Dimension::Percent(*v / 100.0),
            CssLength::Auto => Dimension::Auto,
             _ => Dimension::Auto, 
        }
    }

    style.margin = Rect {
        left: to_taffy_lpa(&computed.margin_left),
        right: to_taffy_lpa(&computed.margin_right),
        top: to_taffy_lpa(&computed.margin_top),
        bottom: to_taffy_lpa(&computed.margin_bottom),
    };
    style.padding = Rect {
        left: to_taffy_lp(&computed.padding_left),
        right: to_taffy_lp(&computed.padding_right),
        top: to_taffy_lp(&computed.padding_top),
        bottom: to_taffy_lp(&computed.padding_bottom),
    };

    style.size.width = to_taffy_dim(&computed.width);
    style.size.height = to_taffy_dim(&computed.height);

    if let AceNodeType::Text(text) = &node.node_type {
            let text_content = text.trim();
            if !text_content.is_empty() {
                style.size.width = Dimension::Percent(1.0);
                // Basic font mapping: assume 16px if not Px
                let font_size = match computed.font_size {
                    CssLength::Px(v) => v,
                    _ => 16.0,
                };
                style.size.height = Dimension::Points(font_size * 1.2);
            }
    }
    
    let taffy_node = taffy.new_leaf(style).unwrap();

    for &child_idx in &node.children {
            if let Some(child) = dom.get_node(child_idx) {
                if let AceNodeType::Text(t) = &child.node_type {
                    if t.trim().is_empty() {
                        continue;
                    }
                }
            }
        
        // Pass current computed style as parent style for children
        let child_node = build_layout_tree(taffy, child_idx, dom, stylesheet, Some(&computed));
        taffy.add_child(taffy_node, child_node).unwrap();
    }

    taffy_node
}

fn generate_display_list(
    primitives: &mut Vec<ACEPrimitive>,
    taffy: &Taffy,
    node_idx: usize, 
    dom: &AceDOM, 
    taffy_node: Node, 
    parent_x: f32, 
    parent_y: f32, 
    stylesheet: &Stylesheet, 
    parent_style: Option<&ComputedStyle> // Changed: pass full style context
) {
    let layout = match taffy.layout(taffy_node) {
        Ok(l) => l,
        Err(_) => return,
    };
    
    let x = parent_x + layout.location.x;
    let y = parent_y + layout.location.y;
    let width = layout.size.width;
    let height = layout.size.height;

    let node = dom.get_node(node_idx).unwrap();
    // Use parent style for inheritance
    let computed = stylesheet.calculate_style(dom, node_idx, parent_style);
    
    if computed.display == CssDisplay::None {
        return;
    }

    // Determine current text properties for rendering
    let current_font_size = match computed.font_size {
        CssLength::Px(v) => v,
        _ => 16.0,
    };
    
    let current_color = match &computed.color {
        CssColor::Named(s) => s.clone(),
        CssColor::Transparent => "transparent".to_string(), // Text shouldn't be transparent usually but ok
        CssColor::CurrentColor => "black".to_string(), // Fallback
         _ => "black".to_string(),
    };

    if let AceNodeType::Element(element) = &node.node_type {
        let tag_name = &element.tag;
        let mut link_url = None;
        let mut image_url = None;
        let mut element_type = "box".to_string();

        if tag_name == "a" {
            link_url = element.attributes.get("href").map(|s| s.to_string());
        } else if tag_name == "img" {
            image_url = element.attributes.get("src").map(|s| s.to_string());
            element_type = "image".to_string();
        }

        let bg_color = match &computed.background_color {
            CssColor::Named(s) => s.clone(),
            CssColor::Transparent => "transparent".to_string(),
             _ => "transparent".to_string(),
        };

        if bg_color != "transparent" || tag_name == "img" || tag_name == "a" {
                primitives.push(ACEPrimitive {
                x, y, width, height,
                color: bg_color,
                text: "".into(), font_size: 0.0,
                image_url, link_url, node_ptr: node_idx, element_type
                });
        }
    }
    
    if let AceNodeType::Text(text) = &node.node_type {
            let text_content = text.trim();
            if !text_content.is_empty() {
                primitives.push(ACEPrimitive {
                x, y, width, height,
                color: current_color.clone(),
                text: text_content.to_string(),
                font_size: current_font_size,
                image_url: None, link_url: None, node_ptr: node_idx, element_type: "text".into()     
                });
            }
    }

    let taffy_children = taffy.children(taffy_node).unwrap();
    let mut relevant_children = Vec::new();
    for &child_idx in &node.children {
        if let Some(child) = dom.get_node(child_idx) {
                if let AceNodeType::Text(t) = &child.node_type {
                    if t.trim().is_empty() {
                        continue;
                    }
                }
                relevant_children.push(child_idx);
        }
    }
    
    for (&child_dom_idx, &child_taffy) in relevant_children.iter().zip(taffy_children.iter()) {
        generate_display_list(primitives, taffy, child_dom_idx, dom, child_taffy, x, y, stylesheet, Some(&computed));
    }
}
