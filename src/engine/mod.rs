use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use url::Url;
use crate::services::resource_manager::{ResourceManager, ResourceType, ResourceResponse};
use tokio::sync::mpsc;
use kuchiki::traits::TendrilSink;

pub mod dom;
pub mod style;
pub mod css_values;
pub mod text;
pub mod svg;
#[cfg(test)]
mod dom_tests;

use self::dom::{AceDOM, AceNodeType};
use self::style::Stylesheet;
use self::css_values::CssFontWeight;
use taffy::prelude::*;
use taffy::Taffy;
use taffy::node::{MeasureFunc, Node};
use taffy::style::{TrackSizingFunction, GridPlacement};

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
    pub node_idx: usize, // Link back to DOM node
}

pub struct AceEngine {
    pub current_url: String,
    pub primitives: Arc<Mutex<Vec<ACEPrimitive>>>,
    pub dom: Option<Arc<Mutex<AceDOM>>>, 
    pub stylesheet: Arc<Mutex<Stylesheet>>,
    pub resource_manager: Option<ResourceManager>,
    taffy: Taffy,
    root_node: Option<Node>,
    pub viewport_width: f32, // Dynamic viewport width
    pub js_runtime: Option<crate::js::JsRuntime>,
    pub image_cache: Arc<Mutex<HashMap<String, slint::Image>>>,
    pub font_system: Arc<Mutex<cosmic_text::FontSystem>>,
    pub hovered_element: Option<usize>,
    pub focused_element: Option<usize>,
}

impl Clone for AceEngine {
    fn clone(&self) -> Self {
        Self {
            current_url: self.current_url.clone(),
            primitives: self.primitives.clone(),
            dom: self.dom.clone(),
            stylesheet: self.stylesheet.clone(),
            resource_manager: self.resource_manager.clone(),
            taffy: Taffy::new(),
            root_node: None,
            viewport_width: self.viewport_width,
            js_runtime: self.js_runtime.clone(),
            image_cache: self.image_cache.clone(),
            font_system: self.font_system.clone(),
            hovered_element: self.hovered_element,
            focused_element: self.focused_element,
        }
    }
}

impl AceEngine {
    pub fn new() -> Self {
        Self {
            current_url: "albedo://start".to_string(),
            primitives: Arc::new(Mutex::new(Vec::new())),
            dom: None,
            stylesheet: Arc::new(Mutex::new(Stylesheet { user_agent_rules: Vec::new(), rules: Vec::new(), media_rules: Vec::new() })),
            resource_manager: None,
            taffy: Taffy::new(),
            root_node: None,
            viewport_width: 1024.0, // Default width
            js_runtime: None,
            image_cache: Arc::new(Mutex::new(HashMap::new())),
            font_system: Arc::new(Mutex::new(cosmic_text::FontSystem::new())),
            hovered_element: None,
            focused_element: None,
        }
    }

    pub fn set_resource_manager(&mut self, rm: ResourceManager) {
        self.resource_manager = Some(rm);
    }

    pub fn load_url(&mut self, url: &str) {
        println!("ENGINE: Iniciando carregamento assíncrono: {}", url);
        self.primitives.lock().unwrap().clear();
        self.current_url = url.to_string();
        
        self.taffy = Taffy::new();
        self.root_node = None;

        if url == "albedo://start" {
            return;
        }

        if let Some(ref rm) = self.resource_manager {
            rm.fetch(url.to_string(), ResourceType::Html);
        } else {
            self.render_error("Resource Manager não inicializado.");
        }
    }
    fn parse_html(&mut self, html: &str) {
        let document = kuchiki::parse_html().one(html);
        let dom = Arc::new(Mutex::new(AceDOM::new(document)));
        self.dom = Some(dom.clone()); 
        
        // Initialize JS Runtime
        self.js_runtime = crate::js::init::init_js_for_url(&self.current_url, self);
        
        self.update_stylesheet();
        self.execute_scripts();
    }

    pub fn execute_scripts(&mut self) {
        let dom_ptr = if let Some(dom) = &self.dom {
            dom.clone()
        } else {
            return;
        };

        let rt_ptr = if let Some(rt) = self.js_runtime.as_ref() {
            rt.clone()
        } else {
            return;
        };

        let dom = dom_ptr.lock().unwrap();
        for node in &dom.nodes {
            if let AceNodeType::Element(el) = &node.node_type {
                if el.tag == "script" {
                    let mut script_content = String::new();
                    for &child_idx in &node.children {
                        if let Some(child) = dom.get_node(child_idx) {
                            if let AceNodeType::Text(text) = &child.node_type {
                                script_content.push_str(text);
                            }
                        }
                    }
                    
                    if !script_content.is_empty() {
                        println!("ENGINE: Executando script no carregamento");
                        let _ = rt_ptr.execute_script(&script_content);
                    }
                }
            }
        }
    }

    pub fn process_resource_responses(&mut self, rx: &mut mpsc::UnboundedReceiver<ResourceResponse>) -> bool {
        let mut needs_sync = false;
        
        while let Ok(res) = rx.try_recv() {
            println!("ENGINE: Recebida resposta assíncrona para: {}", res.url);
            match res.resource_type {
                ResourceType::Html => {
                    if let Ok(html) = String::from_utf8(res.data) {
                        self.parse_html(&html);
                        needs_sync = true;
                    }
                }
                ResourceType::Css => {
                    if let Ok(css_text) = String::from_utf8(res.data) {
                        println!("ENGINE: Aplicando CSS externo recebido ({} bytes)", css_text.len());
                        let parsed = style::parse(&css_text);
                        self.stylesheet.lock().unwrap().rules.extend(parsed.rules);
                        self.recompute_layout();
                        needs_sync = true;
                    }
                }
                ResourceType::Image => {
                    println!("ENGINE: Processando dados de imagem para: {}", res.url);
                    // Converter bytes para Slint Image (isso consome CPU, mas aqui já estamos no pulse)
                    if let Ok(img) = image::load_from_memory(&res.data) {
                        let rgba = img.to_rgba8();
                        let (width, height) = rgba.dimensions();
                        let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(width, height);
                        buffer.make_mut_bytes().copy_from_slice(&rgba.into_raw());
                        let slint_img = slint::Image::from_rgba8_premultiplied(buffer);
                        
                        self.image_cache.lock().unwrap().insert(res.url, slint_img);
                        needs_sync = true;
                    }
                }
            }
        }
        needs_sync
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
            let mut new_style = style::Stylesheet { user_agent_rules: Vec::new(), rules: Vec::new(), media_rules: Vec::new() };
            
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
                        let rel = el.attributes.get("rel").map(|s| s.to_lowercase()).unwrap_or_default();
                        let href = el.attributes.get("href").map(|s| s.to_string());
                        
                        if rel.contains("stylesheet") && href.is_some() {
                            let href = href.unwrap();
                            let css_url = if let Some(ref base) = base_url {
                                base.join(&href).ok()
                            } else {
                                Url::parse(&href).ok()
                            };

                            if let Some(css_url) = css_url {
                                if let Some(ref rm) = self.resource_manager {
                                    println!("ENGINE: Solicitando CSS externo: {}", css_url);
                                    rm.fetch(css_url.to_string(), ResourceType::Css);
                                }
                            }
                        }
                    }
                }
            }
            
            *self.stylesheet.lock().unwrap() = new_style;
        }
    }

    pub fn recompute_layout(&mut self) {
        let dom_ptr = if let Some(dom) = &self.dom {
            dom.clone()
        } else {
            return;
        };

        let viewport_width = self.viewport_width;
        let text_measurer = text::TextMeasurer::new(self.font_system.clone());
        let hovered_element = self.hovered_element;
        let focused_element = self.focused_element;

        {
            let dom = dom_ptr.lock().unwrap();
            
            // Use body as display root, fallback to root
            let display_root = dom.body.unwrap_or(dom.root);
            
            let stylesheet_ptr = self.stylesheet.clone();
            let stylesheet = stylesheet_ptr.lock().unwrap();
            
            self.taffy = Taffy::new();
            
            // First calculate root style (of <html> or body fallback)
            let root_style = stylesheet.calculate_style(&dom, display_root, None, None, hovered_element, focused_element);
            
            // Pass &mut self.taffy explicitly
            let root_node = build_layout_tree(&mut self.taffy, display_root, &dom, &stylesheet, None, Some(&root_style), &text_measurer, hovered_element, focused_element);
            self.root_node = Some(root_node);

            let available_space = Size {
                width: AvailableSpace::Definite(viewport_width),
                height: AvailableSpace::MaxContent,
            };
            let _ = self.taffy.compute_layout(root_node, available_space);

            let mut primitives = self.primitives.lock().unwrap();
            primitives.clear();
            // Background - use viewport width
            primitives.push(ACEPrimitive {
                x: 0.0, y: 0.0, width: viewport_width, height: 8000.0,
                color: "#FFFFFF".to_string(), text: "".into(), font_size: 0.0, 
                link_url: None, element_type: "box".into(), image_url: None,
                z_index: 0, overflow_hidden: false, opacity: 1.0, border_radius: 0.0,
                box_shadow: None, text_shadow: None, background_image: None,
                node_idx: display_root,
            });
            
            // Pass fields explicitly to avoid &mut self borrow conflict
            generate_display_list(
                &mut primitives, 
                &self.taffy, 
                display_root, 
                &dom, 
                root_node,
                0.0, 
                0.0, 
                &stylesheet, 
                None,
                Some(&root_style),
                self.image_cache.clone(),
                hovered_element,
                focused_element
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

    pub fn find_element_at_position(&self, x: f32, y: f32) -> Option<usize> {
        let primitives = self.primitives.lock().unwrap();
        // Search in reverse order (topmost first)
        for prim in primitives.iter().rev() {
            if x >= prim.x && x <= prim.x + prim.width &&
               y >= prim.y && y <= prim.y + prim.height {
                return Some(prim.node_idx);
            }
        }
        None
    }

    pub fn set_hover(&mut self, node_id: Option<usize>) {
        if self.hovered_element != node_id {
            self.hovered_element = node_id;
            // Recompute layout to apply :hover styles
            self.recompute_layout();
        }
    }

    fn render_error(&mut self, msg: &str) {
        self.primitives.lock().unwrap().push(ACEPrimitive {
            x: 20.0, y: 20.0, width: 600.0, height: 50.0,
            color: "transparent".into(),
            text: msg.to_string(),
            font_size: 20.0, link_url: None, element_type: "text".into(), image_url: None,
            z_index: 0, overflow_hidden: false, opacity: 1.0, border_radius: 0.0,
            box_shadow: None, text_shadow: None, background_image: None,
            node_idx: 0,
        });
    }

    pub fn render_visual(&self) -> Vec<ACEPrimitive> {
        self.primitives.lock().unwrap().clone()
    }

    pub fn check_mutations(&mut self) -> (bool, bool) {
        if let Some(ref rt) = self.js_runtime {
             let mut mutated = false;
             let mut style_dirty = false;
             
             if let Ok(mut m) = rt.mutations.lock() {
                 if *m {
                     mutated = true;
                     *m = false;
                 }
             }
             
             if let Ok(mut s) = rt.stylesheet_dirty.lock() {
                 if *s {
                     style_dirty = true;
                     *s = false;
                 }
             }
             
             return (mutated, style_dirty);
        }
        (false, false)
    }
}


// Helper functions (standalone to avoid borrow checker issues)
use crate::engine::css_values::{CssLength, CssColor, CssDisplay, CssFlexDirection, CssPosition, CssOverflow, ComputedStyle, BackgroundImage, Gradient, CssContent};

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

fn build_layout_tree(taffy: &mut Taffy, node_idx: usize, dom: &AceDOM, stylesheet: &Stylesheet, parent_style: Option<&ComputedStyle>, root_style: Option<&ComputedStyle>, text_measurer: &text::TextMeasurer, hovered_element: Option<usize>, focused_element: Option<usize>) -> Node {
    let node = dom.get_node(node_idx).unwrap();
    
    let mut style = Style::default();
    
    // Now we use the passed parent_style for inheritance
    let computed = stylesheet.calculate_style(dom, node_idx, parent_style, root_style, hovered_element, focused_element);
    
    if computed.display == CssDisplay::None {
        style.display = Display::None;
    } else {
        style.display = match computed.display {
            CssDisplay::None => Display::None,
            CssDisplay::Flex => Display::Flex,
            CssDisplay::InlineFlex => Display::Flex,
            CssDisplay::Grid => Display::Grid,
            CssDisplay::Block => Display::Flex,
            CssDisplay::InlineBlock => Display::Flex, 
            CssDisplay::Inline => Display::Flex, 
        };
    }
    
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
            CssLength::Rem(v) | CssLength::Em(v) | CssLength::Fr(v) => LengthPercentageAuto::Points(*v), // Fr fallback
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
            CssLength::Rem(v) | CssLength::Em(v) | CssLength::Fr(v) => LengthPercentage::Points(*v),
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
            CssLength::Rem(v) | CssLength::Em(v) => Dimension::Points(*v),
            CssLength::Fr(v) => Dimension::Percent(*v), // Taffy uses percent for fr if not explicitly grid tracks, but in grid tracks it uses TrackSizingFunction
            CssLength::Auto => Dimension::Auto,
            CssLength::Zero => Dimension::Points(0.0),
        }
    }
    
    fn to_taffy_track(l: &CssLength) -> TrackSizingFunction {
        match l {
            CssLength::Px(v) => TrackSizingFunction::Single(GridTrack { kind: GridTrackKind::Points(*v) }),
            CssLength::Percent(v) => TrackSizingFunction::Single(GridTrack { kind: GridTrackKind::Percent(*v / 100.0) }),
            CssLength::Fr(v) => TrackSizingFunction::Single(GridTrack { kind: GridTrackKind::Flex(*v) }),
            CssLength::Auto => TrackSizingFunction::Single(GridTrack { kind: GridTrackKind::Auto }),
            CssLength::Zero => TrackSizingFunction::Single(GridTrack { kind: GridTrackKind::Points(0.0) }),
            _ => TrackSizingFunction::Single(GridTrack { kind: GridTrackKind::Auto }),
        }
    }
    
    fn to_taffy_grid_pos(l: &CssLength) -> GridPlacement {
        match l {
            CssLength::Px(v) => GridPlacement::from_line_index((*v as i16).max(1)),
            _ => GridPlacement::Auto,
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

    // Grid Layout
    style.grid_template_columns = computed.grid_template_columns.iter().map(to_taffy_track).collect();
    style.grid_template_rows = computed.grid_template_rows.iter().map(to_taffy_track).collect();
    style.grid_column = Line {
        start: to_taffy_grid_pos(&computed.grid_column_start),
        end: to_taffy_grid_pos(&computed.grid_column_end),
    };
    style.grid_row = Line {
        start: to_taffy_grid_pos(&computed.grid_row_start),
        end: to_taffy_grid_pos(&computed.grid_row_end),
    };
    style.gap = Size {
        width: to_taffy_lp(&computed.grid_column_gap),
        height: to_taffy_lp(&computed.grid_row_gap),
    };

    style.size.width = to_taffy_dim(&computed.width);
    style.size.height = to_taffy_dim(&computed.height);

    if let AceNodeType::Text(text) = &node.node_type {
        let text_content = text.trim().to_string();
        if !text_content.is_empty() {
             let font_size = computed.font_size;
            
            let line_height = match computed.line_height {
                CssLength::Px(v) => v,
                CssLength::Percent(v) => font_size * (v / 100.0),
                _ => font_size * 1.2,
            };
            
            let family = Some(computed.font_family.clone());
            let weight = computed.font_weight.to_cosmic();

            let tm = text_measurer.clone();
            let measure_func = move |known_dims: Size<Option<f32>>, available_space: Size<AvailableSpace>| {
                let max_width = known_dims.width.or(match available_space.width {
                    AvailableSpace::Definite(v) => Some(v),
                    _ => None,
                });
                
                let (w, h) = tm.measure_text(&text_content, font_size, line_height, family.as_deref(), weight, max_width);
                Size { width: w, height: h }
            };

            return taffy.new_leaf_with_measure(style, MeasureFunc::Boxed(Box::new(measure_func))).unwrap();
        }
    }
    
    // Ensure body/html have a defined width
    if let AceNodeType::Element(el) = &node.node_type {
        if el.tag == "body" || el.tag == "html" {
            style.size.width = Dimension::Percent(1.0);
        }
    }
    
    let taffy_node = taffy.new_leaf(style).unwrap();
    
    // FASE 5: Shadow DOM support - if has shadow root, render it instead of children
    if let Some(shadow_idx) = node.shadow_root {
         let shadow_root_node = build_layout_tree(taffy, shadow_idx, dom, stylesheet, Some(&computed), root_style, text_measurer, hovered_element, focused_element);
         taffy.add_child(taffy_node, shadow_root_node).unwrap();
         return taffy_node;
    }

    // Skip children for <template> as they are inert
    if let AceNodeType::Element(el) = &node.node_type {
        if el.tag == "template" {
            return taffy_node;
        }
    }

    for &child_idx in &node.children {
            if let Some(child) = dom.get_node(child_idx) {
                if let AceNodeType::Text(t) = &child.node_type {
                    if t.trim().is_empty() {
                        continue;
                    }
                }
            }
        
        // Pass current computed style as parent style for children
        let child_node = build_layout_tree(taffy, child_idx, dom, stylesheet, Some(&computed), root_style, text_measurer, hovered_element, focused_element);
        taffy.add_child(taffy_node, child_node).unwrap();
    }

    taffy_node
}

// Helper function to render pseudo-elements (::before, ::after)
fn render_pseudo_element(
    primitives: &mut Vec<ACEPrimitive>,
    node_idx: usize,
    pseudo_name: &str,
    x: f32,
    y: f32,
    width: f32,
    computed: &ComputedStyle,
) {
    // Only render if content is a string
    if let CssContent::String(text) = &computed.content {
        let font_size = computed.font_size;
        let color = computed.color.to_rgba_string();
        
        primitives.push(ACEPrimitive {
            x,
            y,
            width,
            height: font_size * 1.2,
            color,
            text: text.clone(),
            font_size,
            link_url: None,
            element_type: format!("pseudo-{}", pseudo_name),
            image_url: None,
            z_index: computed.z_index,
            overflow_hidden: false,
            opacity: computed.opacity,
            border_radius: 0.0,
            box_shadow: None,
            text_shadow: None,
            background_image: None,
            node_idx,
        });
    }
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
    parent_style: Option<&ComputedStyle>,
    root_style: Option<&ComputedStyle>,
    image_cache: Arc<Mutex<HashMap<String, slint::Image>>>,
    hovered_element: Option<usize>,
    focused_element: Option<usize>
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
    let computed = stylesheet.calculate_style(dom, node_idx, parent_style, root_style, hovered_element, focused_element);
    
    if computed.display == CssDisplay::None {
        return;
    }

    // Determine current text properties for rendering
    let current_font_size = computed.font_size;
    
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
                    node_idx,
                });
                return; // Skip normal rendering for iframe
            }
        } else if tag_name == "input" || tag_name == "button" || tag_name == "textarea" || tag_name == "select" {
            // FASE 6: Form elements - render with background
            if tag_name == "input" {
                let input_type = element.attributes.get("type").map(|s| s.to_lowercase()).unwrap_or("text".to_string());
                element_type = format!("input-{}", input_type);
                
                // If text input, extract value as text
                if input_type == "text" || input_type == "password" || input_type == "search" {
                    let val = element.attributes.get("value").map(|s| s.to_string()).unwrap_or_default();
                    let placeholder = element.attributes.get("placeholder").map(|s| s.to_string()).unwrap_or_default();
                    
                    let mut text_to_show = if val.is_empty() { placeholder } else { val };
                    if input_type == "password" {
                        text_to_show = "*".repeat(text_to_show.len());
                    }

                    primitives.push(ACEPrimitive {
                        x, y, width, height,
                        color: "#ffffff".into(),
                        text: text_to_show,
                        font_size: 14.0,
                        link_url: None,
                        element_type: element_type.clone(),
                        image_url: None,
                        z_index: computed.z_index,
                        overflow_hidden: true,
                        opacity: computed.opacity,
                        border_radius: 2.0,
                        box_shadow: None, text_shadow: None, background_image: None,
                        node_idx,
                    });
                    return;
                } else if input_type == "checkbox" || input_type == "radio" {
                    let is_checked = element.attributes.contains_key("checked");
                    let char = if is_checked { if input_type == "checkbox" { "X" } else { "●" } } else { "" };
                    
                    primitives.push(ACEPrimitive {
                        x, y, width: 14.0, height: 14.0,
                        color: "#ffffff".into(),
                        text: char.into(),
                        font_size: 12.0,
                        link_url: None,
                        element_type: element_type.clone(),
                        image_url: None,
                        z_index: computed.z_index,
                        overflow_hidden: false,
                        opacity: computed.opacity,
                        border_radius: if input_type == "radio" { 7.0 } else { 2.0 },
                        box_shadow: None, text_shadow: None, background_image: None,
                        node_idx,
                    });
                    return;
                }
            } else if tag_name == "button" {
                element_type = "button".to_string();
                // Button text will be handled by children text nodes, 
                // but we might want a default if empty or value attribute
                if node.children.is_empty() {
                    let val = element.attributes.get("value").map(|s| s.to_string()).unwrap_or_default();
                    if !val.is_empty() {
                        primitives.push(ACEPrimitive {
                            x, y, width, height,
                            color: "#efefef".into(),
                            text: val,
                            font_size: 14.0,
                            link_url: None,
                            element_type: "button".into(),
                            image_url: None,
                            z_index: computed.z_index,
                            overflow_hidden: true,
                            opacity: computed.opacity,
                            border_radius: 3.0,
                            box_shadow: None, text_shadow: None, background_image: None,
                            node_idx,
                        });
                        return;
                    }
                }
            } else {
                element_type = tag_name.to_string();
            }
        } else if tag_name == "svg" {
            // FASE 6: SVG support
            let svg_content = dom.serialize_subtree(node_idx);
            // Use content hash for cache key
            let cache_key = format!("svg-{}x{}-{}", width, height, hash_string(&svg_content));
            
            let mut cache = image_cache.lock().unwrap();
            if !cache.contains_key(&cache_key) {
               if let Some(img) = crate::engine::svg::rasterize_svg(&svg_content, width, height) {
                    cache.insert(cache_key.clone(), img);
               }
            }
            
            if cache.contains_key(&cache_key) {
                element_type = "image".to_string();
                image_url = Some(cache_key);
            } else {
                element_type = "box".to_string();
            }
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
                    text: "".into(),
                    font_size: 0.0,
                    link_url,
                    element_type,
                    image_url,
                    z_index: computed.z_index,
                    overflow_hidden,
                    opacity,
                    border_radius,
                    box_shadow: box_shadow_str,
                    text_shadow: None,
                    background_image: bg_image_str,
                    node_idx,
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
                node_idx,
                });
            }
    }

    // FASE 5: Shadow DOM support - if has shadow root, prioritize its display list
    if let Some(shadow_idx) = node.shadow_root {
        if let Some(shadow_node) = dom.get_node(shadow_idx) {
             let shadow_taffy = taffy.children(taffy_node).unwrap().first().copied();
             if let Some(shadow_taffy) = shadow_taffy {
                generate_display_list(primitives, taffy, shadow_idx, dom, shadow_taffy, x, y, stylesheet, Some(&computed), root_style, image_cache.clone(), hovered_element, focused_element);
                return;
             }
        }
    }

    // Render ::before pseudo-element
    let before_style = stylesheet.calculate_pseudo_style(dom, node_idx, &style::AcePseudoElement::Before, hovered_element, focused_element);
    if before_style.content != CssContent::Normal && before_style.content != CssContent::None {
        render_pseudo_element(primitives, node_idx, "before", x, y, width, &before_style);
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
        generate_display_list(primitives, taffy, child_dom_idx, dom, child_taffy, x, y, stylesheet, Some(&computed), root_style, image_cache.clone(), hovered_element, focused_element);
    }
    
    // Render ::after pseudo-element
    let after_style = stylesheet.calculate_pseudo_style(dom, node_idx, &style::AcePseudoElement::After, hovered_element, focused_element);
    if after_style.content != CssContent::Normal && after_style.content != CssContent::None {
        // Calculate Y position for ::after (defaults to bottom of element for now)
        // In a real layout engine, this would be part of the flow
        let after_y = y + height; 
        render_pseudo_element(primitives, node_idx, "after", x, after_y, width, &after_style);
    }
}

fn hash_string(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hasher, Hash};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
