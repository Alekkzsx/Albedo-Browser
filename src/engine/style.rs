use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    Tag(String),
    Class(String),
    Id(String),
    Universal,
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
    // Buckets for faster matching
    pub id_rules: HashMap<String, Vec<(u32, Rule)>>,
    pub class_rules: HashMap<String, Vec<(u32, Rule)>>,
    pub tag_rules: HashMap<String, Vec<(u32, Rule)>>,
    pub universal_rules: Vec<(u32, Rule)>,
}

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub color: String,
    pub font_size: f32,
    pub background_color: String,
    pub display: DisplayMode,
    pub flex_direction: FlexDirection,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DisplayMode {
    #[default]
    Block,
    Inline,
    Flex,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    Center,
    FlexEnd,
    SpaceBetween,
    SpaceAround,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AlignItems {
    #[default]
    Stretch,
    FlexStart,
    Center,
    FlexEnd,
}

impl Selector {
    pub fn specificity(&self) -> u32 {
        match self {
            Selector::Id(_) => 100,
            Selector::Class(_) => 10,
            Selector::Tag(_) => 1,
            Selector::Universal => 0,
        }
    }
}

impl Stylesheet {
    pub fn parse(css: &str) -> Self {
        let mut stylesheet = Self::default();
        let mut input = css.to_string();
        
        while !input.trim().is_empty() {
            if let Some(brace_pos) = input.find('{') {
                let selectors_str = input[..brace_pos].trim();
                let mut selectors = Vec::new();
                for sel in selectors_str.split(',') {
                    let sel = sel.trim();
                    if sel == "*" {
                        selectors.push(Selector::Universal);
                    } else if sel.starts_with('.') {
                        selectors.push(Selector::Class(sel[1..].to_string()));
                    } else if sel.starts_with('#') {
                        selectors.push(Selector::Id(sel[1..].to_string()));
                    } else {
                        selectors.push(Selector::Tag(sel.to_string()));
                    }
                }

                input = input[brace_pos + 1..].to_string();
                
                if let Some(end_brace) = input.find('}') {
                    let body = &input[..end_brace];
                    let mut declarations = Vec::new();
                    for decl in body.split(';') {
                        let parts: Vec<&str> = decl.split(':').collect();
                        if parts.len() == 2 {
                            declarations.push(Declaration {
                                name: parts[0].trim().to_string(),
                                value: parts[1].trim().to_string(),
                            });
                        }
                    }
                    
                    let rule = Rule { selectors, declarations };
                    for selector in &rule.selectors {
                        let specificity = selector.specificity();
                        match selector {
                            Selector::Id(id) => stylesheet.id_rules.entry(id.clone()).or_default().push((specificity, rule.clone())),
                            Selector::Class(cls) => stylesheet.class_rules.entry(cls.clone()).or_default().push((specificity, rule.clone())),
                            Selector::Tag(tag) => stylesheet.tag_rules.entry(tag.clone()).or_default().push((specificity, rule.clone())),
                            Selector::Universal => stylesheet.universal_rules.push((specificity, rule.clone())),
                        }
                    }
                    stylesheet.rules.push(rule);
                    input = input[end_brace + 1..].to_string();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        stylesheet
    }
}

impl Style {
    pub fn new() -> Self {
        Self {
            color: "#333333".to_string(),
            font_size: 16.0,
            background_color: "transparent".to_string(),
            display: DisplayMode::Block,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
        }
    }


    pub fn get(&self, property: &str) -> String {
        match property {
            "color" => self.color.clone(),
            "font-size" => format!("{}px", self.font_size),
            "background-color" => self.background_color.clone(),
            "display" => format!("{:?}", self.display).to_lowercase(),
            "flex-direction" => match self.flex_direction {
                FlexDirection::Row => "row".to_string(),
                FlexDirection::Column => "column".to_string(),
            },
            "justify-content" => match self.justify_content {
                JustifyContent::FlexStart => "flex-start".to_string(),
                JustifyContent::Center => "center".to_string(),
                JustifyContent::FlexEnd => "flex-end".to_string(),
                JustifyContent::SpaceBetween => "space-between".to_string(),
                JustifyContent::SpaceAround => "space-around".to_string(),
            },
            "align-items" => match self.align_items {
                AlignItems::Stretch => "stretch".to_string(),
                AlignItems::FlexStart => "flex-start".to_string(),
                AlignItems::Center => "center".to_string(),
                AlignItems::FlexEnd => "flex-end".to_string(),
            },
            _ => String::new(),
        }
    }

    pub fn apply_declaration(&mut self, decl: &Declaration) {
        let val = decl.value.as_str();
        match decl.name.as_str() {
            "color" => self.color = val.to_string(),
            "font-size" => {
                let clean_val = val.replace("px", "").replace("pt", "");
                if let Ok(size) = clean_val.parse::<f32>() {
                    self.font_size = size;
                }
            },
            "background-color" | "background" => self.background_color = val.to_string(),
            "display" => {
                self.display = match val {
                    "block" => DisplayMode::Block,
                    "inline" => DisplayMode::Inline,
                    "flex" => DisplayMode::Flex,
                    "none" => DisplayMode::None,
                    _ => self.display,
                };
            },
            "flex-direction" => {
                self.flex_direction = match val {
                    "row" => FlexDirection::Row,
                    "column" => FlexDirection::Column,
                    _ => self.flex_direction,
                };
            },
            "justify-content" => {
                self.justify_content = match val {
                    "flex-start" => JustifyContent::FlexStart,
                    "center" => JustifyContent::Center,
                    "flex-end" => JustifyContent::FlexEnd,
                    "space-between" => JustifyContent::SpaceBetween,
                    "space-around" => JustifyContent::SpaceAround,
                    _ => self.justify_content,
                };
            },
            "align-items" => {
                self.align_items = match val {
                    "stretch" => AlignItems::Stretch,
                    "flex-start" => AlignItems::FlexStart,
                    "center" => AlignItems::Center,
                    "flex-end" => AlignItems::FlexEnd,
                    _ => self.align_items,
                };
            },
            _ => {}
        }
    }

    pub fn parse_inline_style(style_str: &str) -> Self {
        let mut style = Self::new();
        for decl_str in style_str.split(';') {
            let parts: Vec<&str> = decl_str.split(':').collect();
            if parts.len() == 2 {
                let name = parts[0].trim().to_string();
                let value = parts[1].trim().to_string();
                style.apply_declaration(&Declaration { name, value });
            }
        }
        style
    }

    pub fn default_for_tag(tag: &str) -> Self {
        let mut style = Self::new();
        match tag {
            "h1" => {
                style.font_size = 32.0;
                style.display = DisplayMode::Block;
            },
            "h2" => {
                style.font_size = 24.0;
                style.display = DisplayMode::Block;
            },
            "p" | "div" => {
                style.display = DisplayMode::Block;
            },
            _ => {
                style.display = DisplayMode::Inline;
            }
        }
        style
    }
}

pub fn resolve_style(_node: &kuchiki::NodeRef, element: &kuchiki::ElementData, stylesheet: &Stylesheet) -> Style {
    let tag = element.name.local.to_string();
    let mut style = Style::default_for_tag(&tag);
    
    let id = element.attributes.borrow().get("id").unwrap_or_default().to_string();
    let class_attr = element.attributes.borrow().get("class").unwrap_or_default().to_string();
    let classes: Vec<String> = class_attr.split_whitespace().map(|s| s.to_string()).collect();

    let mut matched_rules = Vec::new();
    
    // 1. Match ID rules
    if !id.is_empty() {
        if let Some(rules) = stylesheet.id_rules.get(&id) {
            matched_rules.extend(rules);
        }
    }
    
    // 2. Match Class rules
    for cls in &classes {
        if let Some(rules) = stylesheet.class_rules.get(cls) {
            matched_rules.extend(rules);
        }
    }
    
    // 3. Match Tag rules
    if let Some(rules) = stylesheet.tag_rules.get(&tag) {
        matched_rules.extend(rules);
    }
    
    // 4. Match Universal rules
    matched_rules.extend(&stylesheet.universal_rules);
    
    // Sort by specificity
    matched_rules.sort_by_key(|(spec, _)| *spec);
    
    for (_, rule) in matched_rules {
        for decl in &rule.declarations {
            style.apply_declaration(decl);
        }
    }
    
    // Inline styles (highest priority)
    if let Some(s) = element.attributes.borrow().get("style") {
        for decl_str in s.split(';') {
             let parts: Vec<&str> = decl_str.split(':').collect();
             if parts.len() == 2 {
                 let name = parts[0].trim().to_string();
                 let value = parts[1].trim().to_string();
                 style.apply_declaration(&Declaration { name, value });
             }
        }
    }
    
    style
}
