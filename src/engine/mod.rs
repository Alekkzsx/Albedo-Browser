pub mod dom;
pub mod parser;
pub mod style;
pub mod layout;

#[derive(Debug, Clone)]
pub struct AceEngine {
    pub dom: Option<dom::DomTree>,
}

impl AceEngine {
    pub fn new() -> Self {
        Self { dom: None }
    }

    pub fn load_html(&mut self, html: &str) {
        self.dom = Some(parser::parse_html(html));
    }

    pub fn render(&self) -> String {
        if let Some(dom) = &self.dom {
            // Phase 2: Layout Engine
            if let Some(mut layout_root) = layout::build_layout_tree(&dom.root) {
                let mut viewport = layout::Dimensions::default();
                viewport.content.width = 800.0;
                layout_root.layout(viewport);
                format!("LAYOUT TREE:\n{}", layout_root.render_debug(0))
            } else {
                "Error: Could not build layout tree.".to_string()
            }
        } else {
            "No content loaded.".to_string()
        }
    }

    pub fn render_visual(&self) -> Vec<layout::RenderPrimitive> {
        if let Some(dom) = &self.dom {
            if let Some(mut layout_root) = layout::build_layout_tree(&dom.root) {
                 let mut viewport = layout::Dimensions::default();
                 viewport.content.width = 800.0;
                 layout_root.layout(viewport);
                 return layout_root.flatten();
            }
        }
        Vec::new()
    }
}
