pub mod dom;
pub mod style;
pub mod layout;
pub mod parser;
pub mod internal_pages;

#[cfg(test)]
pub mod layout_tests;

pub use dom::DomTree;
pub use style::Style;
pub use layout::LayoutBox;

#[derive(Clone)]
pub struct AceEngine {
    pub dom: Option<dom::DomTree>,
    pub stylesheet: std::sync::Arc<std::sync::Mutex<style::Stylesheet>>,
}

impl AceEngine {
    pub fn new() -> Self {
        Self {
            dom: None,
            stylesheet: std::sync::Arc::new(std::sync::Mutex::new(style::Stylesheet::default())),
        }
    }

    pub fn load_html(&mut self, html: &str) {
        println!("[ACE] Loading HTML ({} bytes)...", html.len());
        self.dom = Some(crate::engine::parser::parse_html(html));
        println!("[ACE] DOM Parsed successfully.");
        self.sync_stylesheets();
    }

    pub fn load_css(&mut self, css: &str) {
        println!("[ACE] Loading CSS manually ({} bytes)...", css.len());
        let mut stylesheet = self.stylesheet.lock().unwrap();
        *stylesheet = style::Stylesheet::parse(css);
    }

    pub fn sync_stylesheets(&mut self) {
        if let Some(dom) = &self.dom {
            println!("[ACE] Syncing stylesheets from DOM...");
            let mut all_css = String::new();
            if let Ok(styles) = dom.root.select("style") {
                let mut count = 0;
                for style_node in styles {
                    all_css.push_str(&style_node.text_contents());
                    all_css.push('\n');
                    count += 1;
                }
                println!("[ACE] Found {} <style> tags.", count);
            }
            
            if !all_css.is_empty() {
                let mut stylesheet = self.stylesheet.lock().unwrap();
                *stylesheet = style::Stylesheet::parse(&all_css);
                println!("[ACE] Stylesheet updated ({} rules).", stylesheet.rules.len());
            } else {
                println!("[ACE] No internal CSS found.");
            }
        }
    }

    pub fn update_stylesheet(&mut self) {
        self.sync_stylesheets();
    }

    pub fn render(&self) -> String {
        let dom = match &self.dom {
            Some(d) => d,
            None => return "No DOM".to_string(),
        };

        println!("[ACE] Starting render debug...");
        if let Some(mut layout_root) = layout::build_layout_tree(&dom.root, &self.stylesheet.lock().unwrap(), 0) {
            let mut viewport = layout::Dimensions::default();
            viewport.content.width = 1024.0;
            viewport.content.height = 1200.0;
            println!("[ACE] Running layout calculation...");
            layout_root.layout(viewport);
            println!("[ACE] Render debug complete.");
            format!("LAYOUT TREE (Taffy Powered):\n{}", layout_root.render_debug(0))
        } else {
            println!("[ACE] Layout build failed.");
            "Layout failed".to_string()
        }
    }

    pub fn render_visual(&self) -> Vec<layout::RenderPrimitive> {
        let dom = match &self.dom {
            Some(d) => d,
            None => return Vec::new(),
        };

        println!("[ACE] Starting visual render...");
        if let Some(mut layout_root) = layout::build_layout_tree(&dom.root, &self.stylesheet.lock().unwrap(), 0) {
             let mut viewport = layout::Dimensions::default();
             viewport.content.width = 1024.0;
             viewport.content.height = 1200.0;
             println!("[ACE] Running layout calculation...");
             layout_root.layout(viewport);
             println!("[ACE] Flattening tree...");
             let primitives = layout_root.flatten();
             println!("[ACE] Visual render complete ({} primitives).", primitives.len());
             return primitives;
        }
        println!("[ACE] Visual layout build failed.");
        Vec::new()
    }
}

impl Default for AceEngine {
    fn default() -> Self {
        Self::new()
    }
}
