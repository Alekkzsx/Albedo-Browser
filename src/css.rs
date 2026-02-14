// ARQUIVO: src/css.rs

#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    Simple(SimpleSelector),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,  // Ex: "background-color"
    pub value: String, // Ex: "red" ou "#ff0000"
}

// Valor padrão para facilitar criação
impl Default for Rule {
    fn default() -> Self {
        Rule { selectors: Vec::new(), declarations: Vec::new() }
    }
}

pub fn parse(source: &str) -> Stylesheet {
    let mut stylesheet = Stylesheet::default();
    let mut clean_css = source.replace('\n', "").replace('\r', ""); // Remove quebra de linha
    
    // Divide por '}' para pegar cada bloco
    // Ex: "h1 { color: red } p { color: blue }"
    for block in clean_css.split('}') {
        let block = block.trim();
        if block.is_empty() { continue; }

        // Divide o seletor das regras pelo '{'
        if let Some((selector_str, properties_str)) = block.split_once('{') {
            let mut rule = Rule::default();
            
            // 1. Parsear o Seletor (Ex: "h1, div.box")
            for s in selector_str.split(',') {
                rule.selectors.push(parse_simple_selector(s.trim()));
            }

            // 2. Parsear as Propriedades (Ex: "color: red; margin: 10px;")
            for prop in properties_str.split(';') {
                if let Some((name, value)) = prop.split_once(':') {
                    rule.declarations.push(Declaration {
                        name: name.trim().to_string(),
                        value: value.trim().to_string(),
                    });
                }
            }
            
            stylesheet.rules.push(rule);
        }
    }
    stylesheet
}

fn parse_simple_selector(source: &str) -> Selector {
    let mut selector = SimpleSelector { 
        tag_name: None, 
        id: None, 
        class: Vec::new() 
    };

    // Parser "Ingênuo": Assume que é apenas TAG, CLASSE ou ID por enquanto.
    // Ex: "div", ".menu", "#header"
    if source.starts_with('.') {
        selector.class.push(source[1..].to_string());
    } else if source.starts_with('#') {
        selector.id = Some(source[1..].to_string());
    } else {
        selector.tag_name = Some(source.to_string());
    }

    Selector::Simple(selector)
}

// Dados do elemento que vêm do HTML para comparação
pub struct ElementData {
    pub tag_name: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
}

impl Stylesheet {
    // Retorna uma tupla (tamanho_fonte, cor_fundo, cor_texto) baseada nas regras
    pub fn calculate_style(&self, element: &ElementData) -> (f32, String, String) {
        // Valores Padrão (Fallback)
        let mut font_size = 14.0; 
        let mut bg_color = "transparent".to_string();
        let mut text_color = "black".to_string();
        
        // Padrões hardcoded básicos (Reset)
        if element.tag_name == "h1" { font_size = 32.0; }
        if element.tag_name == "h2" { font_size = 24.0; }
        if element.tag_name == "a"  { text_color = "blue".to_string(); }

        // VARRE TODAS AS REGRAS (O Cascading simplificado: última regra vence)
        for rule in &self.rules {
            if rule_matches(rule, element) {
                for decl in &rule.declarations {
                    match decl.name.as_str() {
                        "background-color" | "background" => bg_color = decl.value.clone(),
                        "color" => text_color = decl.value.clone(),
                        "font-size" => {
                            // Tenta parsear "16px" para float
                            if let Some(num_str) = decl.value.strip_suffix("px") {
                                if let Ok(val) = num_str.trim().parse::<f32>() {
                                    font_size = val;
                                }
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
        
        (font_size, bg_color, text_color)
    }
}

fn rule_matches(rule: &Rule, element: &ElementData) -> bool {
    for selector in &rule.selectors {
        if let Selector::Simple(simple) = selector {
            // Verifica Tag
            if let Some(tag) = &simple.tag_name {
                if tag != &element.tag_name { continue; }
            }
            // Verifica ID
            if let Some(id) = &simple.id {
                if Some(id.clone()) != element.id { continue; }
            }
            // Verifica Classe (Simplificado: verifica se tem a primeira classe)
            if !simple.class.is_empty() {
                if !element.classes.contains(&simple.class[0]) { continue; }
            }
            return true; // Deu match em algum seletor
        }
    }
    false
}
