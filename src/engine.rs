use reqwest::blocking::Client;
use std::time::Duration;
use kuchiki::traits::TendrilSink;
use taffy::prelude::*;
use crate::layout::{self, LayoutMetrics};

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

pub struct AceEngine {
    pub current_url: String,
    pub primitives: Vec<ACEPrimitive>,
    client: Client,
    taffy: Taffy, // O motor geométrico
    root_node: Option<Node>, // O nó principal da página (Taffy ID)
}

impl Clone for AceEngine {
    fn clone(&self) -> Self {
        Self {
            current_url: self.current_url.clone(),
            primitives: self.primitives.clone(),
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
            client,
            taffy: Taffy::new(),
            root_node: None,
        }
    }

    pub fn load_url(&mut self, url: &str) {
        println!("ENGINE: Carregando URL: {}", url);
        self.primitives.clear();
        self.current_url = url.to_string();
        
        // Reset Taffy for new page
        self.taffy = Taffy::new();
        self.root_node = None;

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
        
        // 1. Iniciar o Sistema de CSS
        let default_css = "h1 { color: #000000; font-size: 32px; } a { color: #0000ff; } p { color: #333333; } div { display: flex; flex-direction: column; }";
        let stylesheet = crate::css::parse(default_css); 

        // 2. Construir a Árvore de Layout (HTML -> Taffy)
        let root_element = if let Ok(body) = document.select_first("body") {
            body.as_node().clone()
        } else {
            document.clone()
        };

        let root_node = self.build_layout_tree(&root_element, &stylesheet);
        self.root_node = Some(root_node);

        println!("ENGINE: Layout Tree construída.");
        
        // 3. Calcular Layout (Taffy Compute)
        let available_space = Size {
            width: AvailableSpace::Definite(1024.0), // Largura fixa ou da janela
            height: AvailableSpace::MaxContent, // Altura cresce com conteúdo
        };
        
        if let Err(e) = self.taffy.compute_layout(root_node, available_space) {
             println!("ENGINE: Erro no layout: {:?}", e);
             return;
        }
        
        // 4. Gerar Display List (Primitives)
        self.primitives.clear();
        
        // Background padrão
        self.primitives.push(ACEPrimitive {
            x: 0.0, y: 0.0, width: 2000.0, height: 8000.0,
            color: "#FFFFFF".to_string(), text: "".into(), font_size: 0.0, 
            image_url: None, link_url: None, node_ptr: 0, element_type: "box".into()
        });
        
        self.generate_display_list(&root_element, root_node, 0.0, 0.0, &stylesheet);
        
        println!("ENGINE: Primitives gerados: {}", self.primitives.len());
    }

    fn build_layout_tree(&mut self, html_node: &kuchiki::NodeRef, stylesheet: &crate::css::Stylesheet) -> Node {
        let mut style = Style::default();
        style.display = Display::Flex;
        style.flex_direction = FlexDirection::Column;
        
        // Aplicar estilos básicos baseados em tags (Mockado por enquanto, pois o CSS está separado)
        if let Some(element) = html_node.as_element() {
            let tag = element.name.local.to_string();
             // Exemplo de mapeamento básico CSS -> Taffy Style poderia ser aqui
             // Mas por simplicidade mantemos default flex column
             if tag == "div" || tag == "p" || tag == "h1" {
                 style.size.width = Dimension::Percent(1.0);
             }
        }

        // Se for texto, tenta estimar tamanho
        if let Some(text_ref) = html_node.as_text() {
             let text_content = text_ref.borrow().trim().to_string();
             if !text_content.is_empty() {
                 style.size = Size {
                    width: Dimension::Percent(1.0), // Ocupa largura do pai
                    height: Dimension::Points(20.0), // Altura fixa estimada (fallback)
                 };
                 // Idealmente usaríamos measure func aqui, mas Taffy Rust requer closures complexas com 'static ou box
                 // Vamos simplificar para o ACE 2.0: Altura fixa por linha de texto ou auto via filhos.
                 // Melhor: Dimension::Auto e deixamos o conteúdo empurrar se fosse container.
                 // Mas como é leaf de texto, precisamos dar uma altura.
                 
                 // Vamos usar layout::measure_text para estimar a altura baseada na largura esperada (difícil saber a largura final antes do layout)
                 // Solução Sábia: Usar uma altura mínima baseada na fonte padrão.
                 style.size.height = Dimension::Points(18.0);
             }
        }
        
        // Criar nó
        let node = self.taffy.new_leaf(style).unwrap();

        // Recursão para filhos
        for child in html_node.children() {
            // Ignorar whitespace puro
            if let Some(text) = child.as_text() {
                if text.borrow().trim().is_empty() {
                    continue;
                }
            }
            
            let child_node = self.build_layout_tree(&child, stylesheet);
            self.taffy.add_child(node, child_node).unwrap();
        }

        node
    }
    
    fn generate_display_list(&mut self, html_node: &kuchiki::NodeRef, taffy_node: Node, parent_x: f32, parent_y: f32, stylesheet: &crate::css::Stylesheet) {
        let layout = self.taffy.layout(taffy_node).unwrap();
        
        let x = parent_x + layout.location.x;
        let y = parent_y + layout.location.y;
        let width = layout.size.width;
        let height = layout.size.height;

        // Renderizar Elemento HTML
        if let Some(element) = html_node.as_element() {
            let tag_name = element.name.local.to_string();
            let element_data = crate::css::ElementData {
                tag_name: tag_name.clone(),
                id: None,
                classes: vec![],
            };
            
            let (font_size, bg_color, _text_color) = stylesheet.calculate_style(&element_data);
            
            // Se tiver cor de fundo, desenha box
            if bg_color != "transparent" {
                 self.primitives.push(ACEPrimitive {
                    x, y, width, height,
                    color: bg_color,
                    text: "".into(), font_size: 0.0,
                    image_url: None, link_url: None, node_ptr: 0, element_type: "box".into()     
                 });
            }
        }
        
        // Renderizar Texto
        if let Some(text_ref) = html_node.as_text() {
             let text_content = text_ref.borrow().trim().to_string();
             if !text_content.is_empty() {
                 // Cor do texto
                 let color = "#000000".to_string(); // Padrão, idealmente herdar do pai
                 // Para herança real precisariamos passar o style down.
                 
                 self.primitives.push(ACEPrimitive {
                    x, y, width, height,
                    color,
                    text: text_content,
                    font_size: 16.0, // Hardcoded ou vindo do elemento pai se tivessemos contexto
                    image_url: None, link_url: None, node_ptr: 0, element_type: "text".into()     
                 });
             }
        }

        // Recursão Sincronizada
        let taffy_children = self.taffy.children(taffy_node).unwrap();
        let html_children: Vec<_> = html_node.children().filter(|child| {
            if let Some(text) = child.as_text() {
                !text.borrow().trim().is_empty()
            } else {
                true
            }
        }).collect();
        
        for (child_html, &child_taffy) in html_children.iter().zip(taffy_children.iter()) {
            self.generate_display_list(child_html, child_taffy, x, y, stylesheet);
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
