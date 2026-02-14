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
        
        let mut cursor_y = 10.0;
        let start_x = 20.0;

        // 1. FUNDO BRANCO GIGANTE (O papel)
        // Isso garante que não estamos desenhando cinza sobre cinza
        self.primitives.push(ACEPrimitive {
            x: 0.0, y: 0.0, width: 2000.0, height: 5000.0,
            color: "#FFFFFF".to_string(), // Branco Hexadecimal
            text: "".into(), font_size: 0.0, image_url: None, link_url: None, node_ptr: 0, 
            element_type: "box".into()
        });

        // 2. BUSCAR CONTEÚDO
        if let Ok(selectors) = document.select("h1, h2, p, a, div, li") {
            for css_match in selectors {
                let node = css_match.as_node();
                let text_content = node.text_contents();
                
                // Pula elementos vazios
                if text_content.trim().is_empty() { continue; }

                let tag_name = node.as_element().unwrap().name.local.to_string();
                
                // 3. MAPA DE ESTILOS (CORES HEXADECIMAIS HARDCODED)
                // Usando cores claras de fundo para facilitar leitura
                let (font_size, bg_color, height) = match tag_name.as_str() {
                    "h1" => (32.0, "#FFD700", 50.0),      // Amarelo Ouro
                    "h2" => (24.0, "#ADFF2F", 40.0),      // Verde Limão
                    "a"  => (16.0, "#E0FFFF", 30.0),      // Azul Ciano Claro (Links)
                    "li" => (14.0, "#F5F5DC", 25.0),      // Bege
                    "p"  => (14.0, "#FFFFFF", 20.0),      // Branco
                    "div"=> (14.0, "#EEEEEE", 20.0),      // Cinza Claro
                    _    => (14.0, "#FFFFFF", 20.0),
                };

                let link_url = if tag_name == "a" {
                    let attributes = node.as_element().unwrap().attributes.borrow();
                    attributes.get("href").map(|s| s.to_string())
                } else {
                    None
                };

                // 4. CRIAR O BLOCO VISUAL (ACEPrimitive)
                self.primitives.push(ACEPrimitive {
                    x: start_x,
                    y: cursor_y,
                    width: 760.0,
                    height: height,
                    color: bg_color.to_string(), // <--- CORRIGIDO: Usando a cor definida!
                    text: text_content.trim().to_string(),
                    font_size,
                    image_url: None,
                    link_url,
                    node_ptr: 0,
                    element_type: if tag_name == "input" { "input".into() } else { "text".into() }
                });

                cursor_y += height + 5.0; // Espaçamento
            }
        }
        println!("ENGINE: Gerados {} elementos visuais coloridos.", self.primitives.len());
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
