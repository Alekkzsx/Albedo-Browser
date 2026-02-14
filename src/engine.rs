use reqwest::blocking::Client;
use std::time::Duration;
use kuchiki::traits::TendrilSink;
use crate::layout;

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
        
        // 1. Iniciar o Sistema de CSS (Fase 1 que estava "unused")
        // Vamos criar um CSS padrão básico para o Albedo
        let default_css = "h1 { color: #000000; font-size: 32px; } a { color: #0000ff; } p { color: #333333; }";
        let stylesheet = crate::css::parse(default_css);

        let mut cursor_y = 20.0;
        let start_x = 20.0;
        let container_width = 760.0;

        self.primitives.clear();

        // Fundo Branco
        self.primitives.push(ACEPrimitive {
            x: 0.0, y: 0.0, width: 2000.0, height: 8000.0,
            color: "#FFFFFF".to_string(), text: "".into(), font_size: 0.0, 
            image_url: None, link_url: None, node_ptr: 0, element_type: "box".into()
        });

        // 2. Seleção refinada para evitar duplicação (filtramos apenas os 'filhos' com texto)
        if let Ok(selectors) = document.select("h1, h2, h3, p, a, li") {
            for css_match in selectors {
                let node = css_match.as_node();
                let text_content = node.text_contents().trim().to_string();
                
                if text_content.is_empty() { continue; }

                let element = node.as_element().unwrap();
                let tag_name = element.name.local.to_string();

                // 3. USAR O MÓDULO CSS (Fase 1 integrada!)
                let element_data = crate::css::ElementData {
                    tag_name: tag_name.clone(),
                    id: None, // Futuro: pegar ID do atributo
                    classes: vec![], // Futuro: pegar classes
                };
                
                let (font_size, _bg_color, text_color_hex) = stylesheet.calculate_style(&element_data);

                // 4. USAR O MÓDULO LAYOUT (Fase 2 integrada!)
                let metrics = crate::layout::measure_text(&text_content, font_size, container_width);

                // 5. CRIAR O PRIMITIVO
                self.primitives.push(ACEPrimitive {
                    x: start_x,
                    y: cursor_y,
                    width: container_width,
                    height: metrics.height,
                    color: text_color_hex, // Cor vinda do CSS!
                    text: text_content,
                    font_size,
                    image_url: None,
                    link_url: element.attributes.borrow().get("href").map(|s| s.to_string()),
                    node_ptr: 0,
                    element_type: "text".into()
                });

                cursor_y += metrics.height + 15.0; // Espaçamento entre blocos
            }
        }
        println!("ENGINE: Layout ACE 1.5 finalizado. Altura: {}px", cursor_y);
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
