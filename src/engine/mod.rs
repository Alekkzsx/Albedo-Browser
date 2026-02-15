use reqwest::blocking::Client;
use std::time::Duration;
use kuchiki::traits::TendrilSink;
use taffy::prelude::*;
use std::sync::{Arc, Mutex};
use url::Url;

pub mod dom;
pub mod style;
pub mod css_values;
#[cfg(test)]
mod dom_tests;

use self::dom::{AceDOM, AceNodeType};
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
    pub link_url: Option<String>,
    pub element_type: String,
    pub image_url: Option<String>, // URL da imagem para baixar
    pub z_index: i32, // FASE 1: z-index for stacking
    pub overflow_hidden: bool, // FASE 1: overflow handling
    pub opacity: f32, // FASE 2: opacity
    pub border_radius: f32, // FASE 2: border-radius
    
    // Novas propriedades visuais (FASE MELHORIA)
    pub box_shadow: Option<String>, // CSS box-shadow string for rendering
    pub text_shadow: Option<String>, // CSS text-shadow string
    pub background_image: Option<String>, // CSS background (gradient or url)
}

pub struct AceEngine {
    pub current_url: String,
    pub primitives: Vec<ACEPrimitive>,
    pub dom: Option<Arc<Mutex<AceDOM>>>, 
    pub stylesheet: Arc<Mutex<Stylesheet>>,
    client: Client,
    taffy: Taffy,
    root_node: Option<Node>,
    pub viewport_width: f32, // Dynamic viewport width
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
            viewport_width: self.viewport_width,
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
            stylesheet: Arc::new(Mutex::new(Stylesheet { rules: Vec::new(), media_rules: Vec::new() })),
            client,
            taffy: Taffy::new(),
            root_node: None,
            viewport_width: 1024.0, // Default width
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

        let base_url = Url::parse(&self.current_url).ok();

        {
            let dom = dom_ptr.lock().unwrap();
            let mut new_style = style::Stylesheet { rules: Vec::new(), media_rules: Vec::new() };
            
            // FASE 6: Default CSS - User agent stylesheet with semantic elements
            let default_css = r#"
                html, body { display: block; margin: 0; padding: 0; }
                h1 { color: #000000; font-size: 32px; display: block; margin: 16px 0; }
                h2 { color: #000000; font-size: 24px; display: block; margin: 14px 0; }
                h3 { color: #000000; font-size: 20px; display: block; margin: 12px 0; }
                h4, h5, h6 { color: #000000; display: block; margin: 10px 0; }
                p { color: #333333; display: block; margin: 16px 0; }
                a { color: #0000ff; text-decoration: underline; }
                div { display: flex; flex-direction: column; }
                span { display: inline; }
                ul, ol { display: block; padding-left: 40px; margin: 16px 0; }
                li { display: list-item; }
                img { display: inline-block; }
                table { display: table; border-collapse: collapse; }
                tr { display: table-row; }
                td, th { display: table-cell; padding: 4px 8px; border: 1px solid #ccc; }
                
                /* FASE 6: Semantic HTML elements */
                header { display: block; margin: 0 0 16px 0; }
                footer { display: block; margin: 16px 0 0 0; }
                nav { display: block; margin: 8px 0; }
                article { display: block; margin: 16px 0; padding: 16px; }
                section { display: block; margin: 16px 0; }
                aside { display: block; margin: 16px 0; }
                
                /* FASE 6: Form elements */
                input { display: inline-block; margin: 4px; padding: 4px 8px; border: 1px solid #ccc; }
                button { display: inline-block; margin: 4px; padding: 6px 12px; cursor: pointer; }
                textarea { display: inline-block; margin: 4px; padding: 4px 8px; border: 1px solid #ccc; }
                select { display: inline-block; margin: 4px; padding: 4px 8px; border: 1px solid #ccc; }
            "#;
            new_style.rules.extend(style::parse(default_css).rules);

            // Extract all <style> tags
            for node in &dom.nodes {
                if let AceNodeType::Element(el) = &node.node_type {
                    if el.tag == "style" {
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

            // Extract all <link rel="stylesheet"> tags and fetch external CSS
            for node in &dom.nodes {
                if let AceNodeType::Element(el) = &node.node_type {
                    if el.tag == "link" {
                        // Check if it's a stylesheet link
                        let rel = el.attributes.get("rel").map(|s| s.to_lowercase()).unwrap_or_default();
                        let href = el.attributes.get("href").map(|s| s.to_string());
                        
                        if rel.contains("stylesheet") && href.is_some() {
                            let href = href.unwrap();
                            
                            // Resolve relative URL
                            let css_url = if let Some(ref base) = base_url {
                                base.join(&href).ok()
                            } else {
                                Url::parse(&href).ok()
                            };

                            if let Some(css_url) = css_url {
                                println!("ENGINE: Fetching external CSS: {}", css_url);
                                
                                // Fetch the CSS file
                                if let Ok(resp) = self.client.get(css_url.as_str())
                                    .timeout(Duration::from_secs(10))
                                    .send() 
                                {
                                    if resp.status().is_success() {
                                        if let Ok(css_text) = resp.text() {
                                            println!("ENGINE: Loaded {} bytes of external CSS", css_text.len());
                                            new_style.rules.extend(style::parse(&css_text).rules);
                                        }
                                    } else {
                                        println!("ENGINE: Failed to fetch CSS: {}", resp.status());
                                    }
                                } else {
                                    println!("ENGINE: Error fetching external CSS: {}", css_url);
                                }
                            }
                        }
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

        let viewport_width = self.viewport_width;

        {
            let dom = dom_ptr.lock().unwrap();
            
            // Use body as display root, fallback to root
            let display_root = dom.body.unwrap_or(dom.root);
            
            let stylesheet_ptr = self.stylesheet.clone();
            let stylesheet = stylesheet_ptr.lock().unwrap();
            
            self.taffy = Taffy::new();
            
            // Pass &mut self.taffy explicitly
            let root_node = build_layout_tree(&mut self.taffy, display_root, &dom, &stylesheet, None);
            self.root_node = Some(root_node);

            let available_space = Size {
                width: AvailableSpace::Definite(viewport_width),
                height: AvailableSpace::MaxContent,
            };
            let _ = self.taffy.compute_layout(root_node, available_space);

            self.primitives.clear();
            // Background - use viewport width
            self.primitives.push(ACEPrimitive {
                x: 0.0, y: 0.0, width: viewport_width, height: 8000.0,
                color: "#FFFFFF".to_string(), text: "".into(), font_size: 0.0, 
                link_url: None, element_type: "box".into(), image_url: None,
                z_index: 0, overflow_hidden: false, opacity: 1.0, border_radius: 0.0,
                box_shadow: None, text_shadow: None, background_image: None,
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

    /// Set viewport size for responsive layout
    pub fn set_viewport_size(&mut self, width: f32) {
        if (self.viewport_width - width).abs() > 1.0 {
            self.viewport_width = width;
            self.recompute_layout();
        }
    }

    fn render_error(&mut self, msg: &str) {
        self.primitives.push(ACEPrimitive {
            x: 20.0, y: 20.0, width: 600.0, height: 50.0,
            color: "transparent".into(),
            text: msg.to_string(),
            font_size: 20.0, link_url: None, element_type: "text".into(), image_url: None,
            z_index: 0, overflow_hidden: false, opacity: 1.0, border_radius: 0.0,
            box_shadow: None, text_shadow: None, background_image: None,
        });
    }

    pub fn render_visual(&self) -> Vec<ACEPrimitive> {
        self.primitives.clone()
    }
}

// Helper functions (standalone to avoid borrow checker issues)
use crate::engine::css_values::{CssLength, CssColor, CssDisplay, CssFlexDirection, CssPosition, CssOverflow, ComputedStyle, BackgroundImage, Gradient};

fn format_gradient(g: &Gradient) -> String {
    match g {
        Gradient::Linear { angle, stops } => {
            let mut result = format!("linear-gradient({}deg", angle);
            for stop in stops {
                let color = stop.color.to_rgba_string();
                if let Some(pos) = stop.position {
                    result.push_str(&format!(", {} {}%", color, (pos * 100.0) as i32));
                } else {
                    result.push_str(&format!(", {}", color));
                }
            }
            result.push(')');
            result
        },
        Gradient::Radial { shape, stops } => {
            let mut result = format!("radial-gradient({}", shape);
            for stop in stops {
                let color = stop.color.to_rgba_string();
                if let Some(pos) = stop.position {
                    result.push_str(&format!(", {} {}%", color, (pos * 100.0) as i32));
                } else {
                    result.push_str(&format!(", {}", color));
                }
            }
            result.push(')');
            result
        },
    }
}

fn build_layout_tree(taffy: &mut Taffy, node_idx: usize, dom: &AceDOM, stylesheet: &Stylesheet, parent_style: Option<&ComputedStyle>) -> Node {
    let node = dom.get_node(node_idx).unwrap();
    
    let mut style = Style::default();
    
    // Now we use the passed parent_style for inheritance
    let computed = stylesheet.calculate_style(dom, node_idx, parent_style);
    
    style.display = match computed.display {
        CssDisplay::None => Display::None,
        CssDisplay::Flex => Display::Flex,
        CssDisplay::InlineFlex => Display::Flex,
        CssDisplay::Grid => Display::Grid,
        CssDisplay::Block => Display::Flex,
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

    style.flex_direction = match computed.flex_direction {
        CssFlexDirection::Row => FlexDirection::Row,
        CssFlexDirection::RowReverse => FlexDirection::RowReverse,
        CssFlexDirection::Column => FlexDirection::Column,
        CssFlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
    };

    // FASE 1: Apply CSS position (static, relative, absolute, fixed)
    // Note: Taffy 0.3 only has Relative and Absolute
    style.position = match computed.position {
        CssPosition::Static | CssPosition::Relative | CssPosition::Sticky => Position::Relative,
        CssPosition::Absolute | CssPosition::Fixed => Position::Absolute,
    };

    // FASE 1: Apply position offsets (top/left/right/bottom)
    style.inset = Rect {
        left: to_taffy_lpa(&computed.left),
        right: to_taffy_lpa(&computed.right),
        top: to_taffy_lpa(&computed.top),
        bottom: to_taffy_lpa(&computed.bottom),
    };

    // Helper to convert CssLength to Taffy LengthPercentageAuto or Dimension
    fn to_taffy_lpa(l: &CssLength) -> LengthPercentageAuto {
        match l {
            CssLength::Px(v) => LengthPercentageAuto::Points(*v),
            CssLength::Percent(v) => LengthPercentageAuto::Percent(*v / 100.0),
            CssLength::Vw(v) => LengthPercentageAuto::Percent(*v / 100.0), // Simplified
            CssLength::Vh(v) => LengthPercentageAuto::Percent(*v / 100.0), // Simplified
            CssLength::Rem(v) => LengthPercentageAuto::Points(*v * 16.0), // Simplified: 1rem = 16px
            CssLength::Em(v) => LengthPercentageAuto::Points(*v * 16.0), // Simplified: 1em = 16px
            CssLength::Auto => LengthPercentageAuto::Auto,
            CssLength::Zero => LengthPercentageAuto::Points(0.0),
        }
    }
    
    fn to_taffy_lp(l: &CssLength) -> LengthPercentage {
        match l {
            CssLength::Px(v) => LengthPercentage::Points(*v),
            CssLength::Percent(v) => LengthPercentage::Percent(*v / 100.0),
            CssLength::Vw(v) => LengthPercentage::Percent(*v / 100.0), // Simplified
            CssLength::Vh(v) => LengthPercentage::Percent(*v / 100.0), // Simplified
            CssLength::Rem(v) => LengthPercentage::Points(*v * 16.0),
            CssLength::Em(v) => LengthPercentage::Points(*v * 16.0),
            CssLength::Zero => LengthPercentage::Points(0.0),
            CssLength::Auto => LengthPercentage::Points(0.0),
        }
    }

    fn to_taffy_dim(l: &CssLength) -> Dimension {
        match l {
            CssLength::Px(v) => Dimension::Points(*v),
            CssLength::Percent(v) => Dimension::Percent(*v / 100.0),
            CssLength::Vw(v) => Dimension::Percent(*v / 100.0), // Simplified
            CssLength::Vh(v) => Dimension::Percent(*v / 100.0), // Simplified
            CssLength::Rem(v) => Dimension::Points(*v * 16.0),
            CssLength::Em(v) => Dimension::Points(*v * 16.0),
            CssLength::Auto => Dimension::Auto,
            CssLength::Zero => Dimension::Points(0.0),
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

    // Ensure minimum width for text elements and flex containers
    if let AceNodeType::Text(text) = &node.node_type {
            let text_content = text.trim();
            if !text_content.is_empty() {
                // Use percent width but ensure parent has constraints
                style.size.width = Dimension::Percent(1.0);
                // Basic font mapping: assume 16px if not Px
                let font_size = match computed.font_size {
                    CssLength::Px(v) => v,
                    _ => 16.0,
                };
                style.size.height = Dimension::Points(font_size * 1.2);
                // Add min-width to ensure text is visible
                style.min_size.width = Dimension::Points(100.0);
            }
    }
    
    // Ensure body/html have a defined width
    if let AceNodeType::Element(el) = &node.node_type {
        if el.tag == "body" || el.tag == "html" {
            style.size.width = Dimension::Percent(1.0);
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
        CssColor::Transparent => "transparent".to_string(),
        CssColor::CurrentColor => "black".to_string(),
        CssColor::Rgba(r, g, b, a) => {
            if *a < 1.0 {
                format!("rgba({},{},{},{})", r, g, b, a)
            } else {
                format!("#{:02x}{:02x}{:02x}", r, g, b)
            }
        }
    };

    if let AceNodeType::Element(element) = &node.node_type {
        let tag_name = &element.tag;
        let mut link_url = None;
        let mut element_type = "box".to_string();
        let mut image_url = None;

        if tag_name == "a" {
            link_url = element.attributes.get("href").map(|s| s.to_string());
        } else if tag_name == "img" {
            element_type = "image".to_string();
            // Extract image URL from src attribute
            image_url = element.attributes.get("src").map(|s| s.to_string());
        } else if tag_name == "iframe" {
            // FASE 6: iframe support - render as placeholder
            element_type = "iframe".to_string();
            // Extract src if present
            let src = element.attributes.get("src").map(|s| s.to_string());
            if let Some(src) = src {
                primitives.push(ACEPrimitive {
                    x, y, width: width.max(100.0), height: height.max(50.0),
                    color: "#f0f0f0".to_string(),
                    text: format!("[iframe: {}]", src),
                    font_size: 12.0,
                    link_url: None,
                    element_type: "iframe".to_string(),
                    image_url: None,
                    z_index: computed.z_index,
                    overflow_hidden: false,
                    opacity: computed.opacity,
                    border_radius: 0.0,
                    box_shadow: None, text_shadow: None, background_image: None,
                });
                return; // Skip normal rendering for iframe
            }
        } else if tag_name == "input" || tag_name == "button" || tag_name == "textarea" || tag_name == "select" {
            // FASE 6: Form elements - render with background
            element_type = tag_name.to_string();
        }

        let bg_color = match &computed.background_color {
            CssColor::Named(s) => s.clone(),
            CssColor::Transparent => "transparent".to_string(),
            CssColor::CurrentColor => "transparent".to_string(),
            CssColor::Rgba(r, g, b, a) => {
                if *a < 1.0 {
                    format!("rgba({},{},{},{})", r, g, b, a)
                } else {
                    format!("#{:02x}{:02x}{:02x}", r, g, b)
                }
            }
        };

        if bg_color != "transparent" || tag_name == "img" || tag_name == "a" || 
           tag_name == "input" || tag_name == "button" || tag_name == "textarea" || 
           tag_name == "select" || tag_name == "iframe" {
                // FASE 1: Get overflow value
                let overflow_hidden = matches!(computed.overflow, CssOverflow::Hidden);
                // FASE 2: Get opacity and border-radius
                let opacity = computed.opacity;
                let border_radius = computed.border_radius_top_left.max(computed.border_radius_top_right)
                    .max(computed.border_radius_bottom_left).max(computed.border_radius_bottom_right);
                
                // MELHORIA: Get box-shadow
                let box_shadow_str = if computed.box_shadow.is_empty() {
                    None
                } else {
                    let shadows: Vec<String> = computed.box_shadow.iter().map(|s| {
                        let color = s.color.to_rgba_string();
                        if s.inset {
                            format!("inset {}px {}px {}px {}px {}", s.offset_x, s.offset_y, s.blur, s.spread, color)
                        } else {
                            format!("{}px {}px {}px {}px {}", s.offset_x, s.offset_y, s.blur, s.spread, color)
                        }
                    }).collect();
                    Some(shadows.join(", "))
                };
                
                // MELHORIA: Get background-image
                let bg_image_str = match &computed.background_image {
                    BackgroundImage::None => None,
                    BackgroundImage::Color(c) => Some(c.to_rgba_string()),
                    BackgroundImage::Gradient(g) => {
                        Some(format_gradient(g))
                    },
                    BackgroundImage::Url(url) => Some(format!("url({})", url)),
                };
                
                // For form elements, ensure there's a background
                let bg = if bg_color == "transparent" && 
                    (tag_name == "input" || tag_name == "button" || tag_name == "textarea" || tag_name == "select") {
                    "#ffffff".to_string()
                } else {
                    bg_color
                };
                
                // Override background with gradient if present
                let final_bg = if let Some(ref gradient) = bg_image_str {
                    if !gradient.starts_with("url(") && gradient != "transparent" {
                        gradient.clone()
                    } else {
                        bg
                    }
                } else {
                    bg
                };
                
                primitives.push(ACEPrimitive {
                x, y, width, height,
                color: final_bg,
                text: "".into(), font_size: 0.0,
                link_url, element_type, image_url,
                z_index: computed.z_index, overflow_hidden, opacity, border_radius,
                box_shadow: box_shadow_str, text_shadow: None, background_image: bg_image_str,
                });
        }
    }
    
    if let AceNodeType::Text(text) = &node.node_type {
            let text_content = text.trim();
            if !text_content.is_empty() {
                // MELHORIA: Get text-shadow
                let text_shadow_str = if computed.text_shadow.is_empty() {
                    None
                } else {
                    let shadows: Vec<String> = computed.text_shadow.iter().map(|s| {
                        let color = s.color.to_rgba_string();
                        format!("{}px {}px {}px {}", s.offset_x, s.offset_y, s.blur, color)
                    }).collect();
                    Some(shadows.join(", "))
                };
                
                primitives.push(ACEPrimitive {
                x, y, width, height,
                color: current_color.clone(),
                text: text_content.to_string(),
                font_size: current_font_size,
                link_url: None, element_type: "text".into(), image_url: None,
                z_index: computed.z_index, overflow_hidden: false, opacity: computed.opacity, border_radius: 0.0,
                box_shadow: None, text_shadow: text_shadow_str, background_image: None,
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
