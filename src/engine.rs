use reqwest::blocking::Client;
use std::time::Duration;
use kuchiki::traits::TendrilSink;

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
    pub element_type: String, // "text", "input", "button", "box"
}

#[derive(Clone)]
pub struct AceEngine {
    pub current_url: String,
    pub primitives: Vec<ACEPrimitive>,
    client: Client,
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
            client,
        }
    }

    pub fn load_url(&mut self, url: &str) {
        println!("ENGINE: Carregando URL: {}", url);
        self.primitives.clear();
        self.current_url = url.to_string();

        if url == "albedo://start" {
            // Renderiza vazio, deixa a UI lidar com a StartPage
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
        
        let mut y_cursor = 10.0;
        let x_pad = 10.0;
        let max_width = 780.0;

        // Fundo Branco do Site
        self.primitives.push(ACEPrimitive {
            x: 0.0, y: 0.0, width: 2000.0, height: 5000.0,
            color: "white".into(), text: "".into(), font_size: 0.0,
            image_url: None, link_url: None, node_ptr: 0, element_type: "box".into()
        });

        // Seletor CSS Básico
        if let Ok(matches) = document.select("h1, h2, p, a, div, span") {
            for css_match in matches {
                let node = css_match.as_node();
                let text = node.text_contents().trim().to_string();
                
                if text.is_empty() { continue; }
                
                let tag = node.as_element().unwrap().name.local.to_string();
                
                // Estilos Hardcoded para teste
                let (f_size, color, height, is_link) = match tag.as_str() {
                    "h1" => (32.0, "#000000", 40.0, false),
                    "h2" => (24.0, "#222222", 30.0, false),
                    "a" => (16.0, "#0000FF", 20.0, true),
                    _ => (14.0, "#333333", 18.0, false),
                };

                let link_url = if is_link {
                     node.as_element().unwrap().attributes.borrow().get("href").map(|s| s.to_string())
                } else { None };

                self.primitives.push(ACEPrimitive {
                    x: x_pad,
                    y: y_cursor,
                    width: max_width,
                    height: height,
                    color: "transparent".into(),
                    text: text,
                    font_size: f_size,
                    image_url: None,
                    link_url,
                    node_ptr: 0,
                    element_type: "text".into()
                });
                
                y_cursor += height + 5.0;
            }
        }
        println!("ENGINE: Gerados {} elementos visuais.", self.primitives.len());
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
