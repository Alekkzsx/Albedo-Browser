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
    pub fn specificity(&self) -> (u32, u32, u32) {
        match self {
            Selector::Id(_) => (1, 0, 0),
            Selector::Class(_) => (0, 1, 0),
            Selector::Tag(_) => (0, 0, 1),
            Selector::Universal => (0, 0, 0),
        }
    }

    pub fn matches_tag(&self, tag: &str) -> bool {
        match self {
            Selector::Tag(t) => t == tag,
            Selector::Universal => true,
            _ => false,
        }
    }

    pub fn matches_class(&self, classes: &[String]) -> bool {
        match self {
            Selector::Class(c) => classes.contains(c),
            Selector::Universal => true,
            _ => false,
        }
    }

    pub fn matches_id(&self, id: &str) -> bool {
        match self {
            Selector::Id(i) => i == id,
            Selector::Universal => true,
            _ => false,
        }
    }
}

impl Stylesheet {
    pub fn parse(css: &str) -> Self {
        let mut stylesheet = Self::default();
        let mut input = css.to_string();
        
        // Very basic CSS parser
        while !input.trim().is_empty() {
            // Find selector block
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
                    stylesheet.rules.push(Rule { selectors, declarations });
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
