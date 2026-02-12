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

    pub fn parse_inline_style(style_str: &str) -> Self {
        let mut style = Self::new();
        for decl in style_str.split(';') {
            let parts: Vec<&str> = decl.split(':').collect();
            if parts.len() == 2 {
                let prop = parts[0].trim();
                let val = parts[1].trim();
                match prop {
                    "color" => style.color = val.to_string(),
                    "font-size" => {
                        if let Ok(size) = val.replace("px", "").parse::<f32>() {
                            style.font_size = size;
                        }
                    },
                    "background-color" => style.background_color = val.to_string(),
                    "display" => {
                        style.display = match val {
                            "block" => DisplayMode::Block,
                            "inline" => DisplayMode::Inline,
                            "flex" => DisplayMode::Flex,
                            "none" => DisplayMode::None,
                            _ => DisplayMode::Block,
                        };
                    },
                    "flex-direction" => {
                        style.flex_direction = match val {
                            "row" => FlexDirection::Row,
                            "column" => FlexDirection::Column,
                            _ => FlexDirection::Row,
                        };
                    },
                    "justify-content" => {
                        style.justify_content = match val {
                            "flex-start" => JustifyContent::FlexStart,
                            "center" => JustifyContent::Center,
                            "flex-end" => JustifyContent::FlexEnd,
                            "space-between" => JustifyContent::SpaceBetween,
                            "space-around" => JustifyContent::SpaceAround,
                            _ => JustifyContent::FlexStart,
                        };
                    },
                    "align-items" => {
                        style.align_items = match val {
                            "stretch" => AlignItems::Stretch,
                            "flex-start" => AlignItems::FlexStart,
                            "center" => AlignItems::Center,
                            "flex-end" => AlignItems::FlexEnd,
                            _ => AlignItems::Stretch,
                        };
                    },
                    _ => {}
                }
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
