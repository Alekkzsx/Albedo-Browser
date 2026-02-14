use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    Tag(String),
    Class(String),
    Id(String),
    Universal,
    Compound(Vec<Selector>),
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
    // Buckets for faster matching: (specificity, rule_index, rule)
    pub id_rules: HashMap<String, Vec<(u32, usize, Rule)>>,
    pub class_rules: HashMap<String, Vec<(u32, usize, Rule)>>,
    pub tag_rules: HashMap<String, Vec<(u32, usize, Rule)>>,
    pub universal_rules: Vec<(u32, usize, Rule)>,
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
    
    // Dimension & Box Model
    pub width: String,
    pub height: String,
    pub margin_top: String,
    pub margin_right: String,
    pub margin_bottom: String,
    pub margin_left: String,
    pub padding_top: String,
    pub padding_right: String,
    pub padding_bottom: String,
    pub padding_left: String,
    pub border_width: String,
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
            Selector::Compound(parts) => parts.iter().map(|s| s.specificity()).sum(),
        }
    }
    
    pub fn matches(&self, element: &kuchiki::ElementData) -> bool {
        match self {
            Selector::Universal => true,
            Selector::Tag(tag) => element.name.local.to_string() == *tag,
            Selector::Id(id) => {
                element.attributes.borrow().get("id").map(|s| s.to_string() == *id).unwrap_or(false)
            },
            Selector::Class(cls) => {
                element.attributes.borrow().get("class")
                    .map(|s| s.split_whitespace().any(|c| c == cls))
                    .unwrap_or(false)
            },
            Selector::Compound(parts) => parts.iter().all(|s| s.matches(element)),
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
                    } else if sel.contains('.') || sel.contains('#') {
                        // Very simple compound selector parsing (e.g. p.important or div#target)
                        // This identifies parts but doesn't handle all CSS complexities
                        let mut parts = Vec::new();
                        let mut current = String::new();
                        let mut chars = sel.chars().peekable();
                        
                        while let Some(c) = chars.next() {
                            if c == '.' || c == '#' {
                                if !current.is_empty() {
                                    parts.push(Selector::Tag(current.clone()));
                                    current.clear();
                                }
                                let kind = c;
                                while let Some(&nc) = chars.peek() {
                                    if nc == '.' || nc == '#' || nc == ' ' { break; }
                                    current.push(chars.next().unwrap());
                                }
                                if kind == '.' {
                                    parts.push(Selector::Class(current.clone()));
                                } else {
                                    parts.push(Selector::Id(current.clone()));
                                }
                                current.clear();
                            } else {
                                current.push(c);
                            }
                        }
                        if !current.is_empty() {
                            parts.push(Selector::Tag(current));
                        }
                        
                        if parts.len() == 1 {
                            selectors.push(parts.remove(0));
                        } else {
                            selectors.push(Selector::Compound(parts));
                        }
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
                    
                    let rule_index = stylesheet.rules.len();
                    let rule = Rule { selectors, declarations };
                    for selector in &rule.selectors {
                        let specificity = selector.specificity();
                        match selector {
                            Selector::Id(id) => stylesheet.id_rules.entry(id.clone()).or_default().push((specificity, rule_index, rule.clone())),
                            Selector::Class(cls) => stylesheet.class_rules.entry(cls.clone()).or_default().push((specificity, rule_index, rule.clone())),
                            Selector::Tag(tag) => stylesheet.tag_rules.entry(tag.clone()).or_default().push((specificity, rule_index, rule.clone())),
                            Selector::Compound(parts) => {
                                // Add to the best bucket (Id > Class > Tag)
                                let mut best_bucket_found = false;
                                for p in parts {
                                    match p {
                                        Selector::Id(id) => {
                                            stylesheet.id_rules.entry(id.clone()).or_default().push((specificity, rule_index, rule.clone()));
                                            best_bucket_found = true;
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                                if !best_bucket_found {
                                    for p in parts {
                                        match p {
                                            Selector::Class(cls) => {
                                                stylesheet.class_rules.entry(cls.clone()).or_default().push((specificity, rule_index, rule.clone()));
                                                best_bucket_found = true;
                                                break;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                if !best_bucket_found {
                                    for p in parts {
                                        match p {
                                            Selector::Tag(tag) => {
                                                stylesheet.tag_rules.entry(tag.clone()).or_default().push((specificity, rule_index, rule.clone()));
                                                best_bucket_found = true;
                                                break;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                if !best_bucket_found {
                                    stylesheet.universal_rules.push((specificity, rule_index, rule.clone()));
                                }
                            }
                            Selector::Universal => stylesheet.universal_rules.push((specificity, rule_index, rule.clone())),
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
            width: "auto".to_string(),
            height: "auto".to_string(),
            margin_top: "0px".to_string(),
            margin_right: "0px".to_string(),
            margin_bottom: "0px".to_string(),
            margin_left: "0px".to_string(),
            padding_top: "0px".to_string(),
            padding_right: "0px".to_string(),
            padding_bottom: "0px".to_string(),
            padding_left: "0px".to_string(),
            border_width: "0px".to_string(),
        }
    }


    pub fn get(&self, property: &str) -> String {
        match property {
            "color" => self.color.clone(),
            "font-size" => format!("{}px", self.font_size),
            "background-color" => self.background_color.clone(),
            "display" => format!("{:?}", self.display).to_lowercase(),
            "flex-direction" => format!("{:?}", self.flex_direction).to_lowercase(),
            "justify-content" => format!("{:?}", self.justify_content).to_lowercase(),
            "align-items" => format!("{:?}", self.align_items).to_lowercase(),
            "width" => self.width.clone(),
            "height" => self.height.clone(),
            "margin-top" => self.margin_top.clone(),
            "margin-right" => self.margin_right.clone(),
            "margin-bottom" => self.margin_bottom.clone(),
            "margin-left" => self.margin_left.clone(),
            "padding-top" => self.padding_top.clone(),
            "padding-right" => self.padding_right.clone(),
            "padding-bottom" => self.padding_bottom.clone(),
            "padding-left" => self.padding_left.clone(),
            "border-width" => self.border_width.clone(),
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
            "width" => self.width = val.to_string(),
            "height" => self.height = val.to_string(),
            "margin" => {
                let parts: Vec<&str> = val.split_whitespace().collect();
                match parts.len() {
                    1 => {
                        self.margin_top = parts[0].to_string();
                        self.margin_right = parts[0].to_string();
                        self.margin_bottom = parts[0].to_string();
                        self.margin_left = parts[0].to_string();
                    }
                    2 => {
                        self.margin_top = parts[0].to_string();
                        self.margin_bottom = parts[0].to_string();
                        self.margin_left = parts[1].to_string();
                        self.margin_right = parts[1].to_string();
                    }
                    4 => {
                        self.margin_top = parts[0].to_string();
                        self.margin_right = parts[1].to_string();
                        self.margin_bottom = parts[2].to_string();
                        self.margin_left = parts[3].to_string();
                    }
                    _ => {}
                }
            },
            "margin-top" => self.margin_top = val.to_string(),
            "margin-right" => self.margin_right = val.to_string(),
            "margin-bottom" => self.margin_bottom = val.to_string(),
            "margin-left" => self.margin_left = val.to_string(),
            "padding" => {
                let parts: Vec<&str> = val.split_whitespace().collect();
                match parts.len() {
                    1 => {
                        self.padding_top = parts[0].to_string();
                        self.padding_right = parts[0].to_string();
                        self.padding_bottom = parts[0].to_string();
                        self.padding_left = parts[0].to_string();
                    }
                    2 => {
                        self.padding_top = parts[0].to_string();
                        self.padding_bottom = parts[0].to_string();
                        self.padding_left = parts[1].to_string();
                        self.padding_right = parts[1].to_string();
                    }
                    4 => {
                        self.padding_top = parts[0].to_string();
                        self.padding_right = parts[1].to_string();
                        self.padding_bottom = parts[2].to_string();
                        self.padding_left = parts[3].to_string();
                    }
                    _ => {}
                }
            },
            "padding-top" => self.padding_top = val.to_string(),
            "padding-right" => self.padding_right = val.to_string(),
            "padding-bottom" => self.padding_bottom = val.to_string(),
            "padding-left" => self.padding_left = val.to_string(),
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
    
    // We match from buckets, but we MUST re-verify Compound selectors
    // because they are pushed into buckets based on one of their parts.
    
    // 1. Match ID rules
    if !id.is_empty() {
        if let Some(rules) = stylesheet.id_rules.get(&id) {
            for (spec, idx, rule) in rules {
                if rule.selectors.iter().any(|s| s.matches(element)) {
                    matched_rules.push((*spec, *idx, rule.clone()));
                }
            }
        }
    }
    
    // 2. Match Class rules
    for cls in &classes {
        if let Some(rules) = stylesheet.class_rules.get(cls) {
            for (spec, idx, rule) in rules {
                if rule.selectors.iter().any(|s| s.matches(element)) {
                    matched_rules.push((*spec, *idx, rule.clone()));
                }
            }
        }
    }
    
    // 3. Match Tag rules
    if let Some(rules) = stylesheet.tag_rules.get(&tag) {
        for (spec, idx, rule) in rules {
            if rule.selectors.iter().any(|s| s.matches(element)) {
                matched_rules.push((*spec, *idx, rule.clone()));
            }
        }
    }
    
    // 4. Match Universal rules
    for (spec, idx, rule) in &stylesheet.universal_rules {
        if rule.selectors.iter().any(|s| s.matches(element)) {
            matched_rules.push((*spec, *idx, rule.clone()));
        }
    }

    // Deduplicate rules (a rule might be in multiple buckets if it has multiple selectors)
    // Actually, matched_rules stores (specificity, index, rule). 
    // We should deduplicate by rule index.
    matched_rules.sort_by_key(|(_, idx, _)| *idx);
    matched_rules.dedup_by_key(|(_, idx, _)| *idx);
    
    // Sort by specificity (major) and definition order (minor)
    matched_rules.sort_by_key(|(spec, idx, _)| (*spec, *idx));
    
    for (_, _, rule) in matched_rules {
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
