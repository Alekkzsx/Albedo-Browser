use taffy::prelude::*;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Length {
    Px(f32),
    Percent(f32),
    Auto,
    Initial,
}

impl Default for Length {
    fn default() -> Self {
        Length::Auto
    }
}

impl Length {
    pub fn parse(s: &str) -> Self {
        if s == "auto" { return Length::Auto; }
        if s == "initial" { return Length::Initial; }
        if s.ends_with('%') {
            if let Ok(val) = s[..s.len()-1].parse::<f32>() {
                return Length::Percent(val / 100.0);
            }
        }
        if s.ends_with("px") {
            if let Ok(val) = s[..s.len()-2].parse::<f32>() {
                return Length::Px(val);
            }
        }
        if let Ok(val) = s.parse::<f32>() {
             return Length::Px(val);
        }
        Length::Auto
    }

    pub fn to_taffy_dimension(&self) -> taffy::style::Dimension {
        match self {
            Length::Px(v) => taffy::style::Dimension::length(*v),
            Length::Percent(v) => taffy::style::Dimension::percent(*v),
            Length::Auto => taffy::style::Dimension::auto(),
            Length::Initial => taffy::style::Dimension::auto(),
        }
    }

    pub fn to_taffy_lp_auto(&self) -> taffy::style::LengthPercentageAuto {
        match self {
            Length::Px(v) => taffy::style::LengthPercentageAuto::length(*v),
            Length::Percent(v) => taffy::style::LengthPercentageAuto::percent(*v),
            Length::Auto => taffy::style::LengthPercentageAuto::auto(),
            Length::Initial => taffy::style::LengthPercentageAuto::auto(),
        }
    }

    pub fn to_taffy_lp(&self) -> taffy::style::LengthPercentage {
        match self {
            Length::Px(v) => taffy::style::LengthPercentage::length(*v),
            Length::Percent(v) => taffy::style::LengthPercentage::percent(*v),
            _ => taffy::style::LengthPercentage::length(0.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgba({}, {}, {}, {})", self.r, self.g, self.b, self.a)
    }
}

impl Color {
    pub fn parse(s: &str) -> Self {
        if s.starts_with('#') {
            if s.len() == 7 {
                let r = u8::from_str_radix(&s[1..3], 16).unwrap_or(0);
                let g = u8::from_str_radix(&s[3..5], 16).unwrap_or(0);
                let b = u8::from_str_radix(&s[5..7], 16).unwrap_or(0);
                return Color { r, g, b, a: 255 };
            }
        }
        match s {
            "red" => Color { r: 255, g: 0, b: 0, a: 255 },
            "green" => Color { r: 0, g: 255, b: 0, a: 255 },
            "blue" => Color { r: 0, g: 0, b: 255, a: 255 },
            "white" => Color { r: 255, g: 255, b: 255, a: 255 },
            "black" => Color { r: 0, g: 0, b: 0, a: 255 },
            "transparent" => Color { r: 0, g: 0, b: 0, a: 0 },
            _ => Color::default(),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color { r: 51, g: 51, b: 51, a: 255 } 
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayMode {
    #[default]
    Block,
    Inline,
    Flex,
    None,
}

pub use taffy::style::FlexDirection as FlexDirectionMode;
pub use taffy::style::JustifyContent as JustifyContentMode;
pub use taffy::style::AlignItems as AlignItemsMode;

#[derive(Debug, Clone)]
pub struct AlbedoStyle {
    pub display: DisplayMode,
    pub flex_direction: FlexDirectionMode,
    pub justify_content: JustifyContentMode,
    pub align_items: AlignItemsMode,
    
    pub color: Color,
    pub background_color: Color,
    pub font_size: f32,
    
    pub width: Length,
    pub height: Length,
    
    pub margin_top: Length,
    pub margin_right: Length,
    pub margin_bottom: Length,
    pub margin_left: Length,
    
    pub padding_top: Length,
    pub padding_right: Length,
    pub padding_bottom: Length,
    pub padding_left: Length,
    pub border_width: Length,
}

impl Default for AlbedoStyle {
    fn default() -> Self {
        Self {
            display: DisplayMode::Block,
            flex_direction: FlexDirectionMode::Row,
            justify_content: JustifyContentMode::FlexStart,
            align_items: AlignItemsMode::Stretch,
            color: Color::default(),
            background_color: Color::default(),
            font_size: 16.0,
            width: Length::Auto,
            height: Length::Auto,
            margin_top: Length::Auto,
            margin_right: Length::Auto,
            margin_bottom: Length::Auto,
            margin_left: Length::Auto,
            padding_top: Length::Auto,
            padding_right: Length::Auto,
            padding_bottom: Length::Auto,
            padding_left: Length::Auto,
            border_width: Length::Px(0.0),
        }
    }
}

impl AlbedoStyle {
    pub fn default_for_tag(tag: &str) -> Self {
        let mut style = Self::default();
        match tag {
            "h1" => style.font_size = 32.0,
            "h2" => style.font_size = 24.0,
            "span" | "a" | "b" | "i" => style.display = DisplayMode::Inline,
            _ => {}
        }
        style
    }

    pub fn parse_inline_style(css: &str) -> Self {
        let mut style = Self::default();
        let mut input = cssparser::ParserInput::new(css);
        let mut parser = cssparser::Parser::new(&mut input);
        let declarations = crate::engine::style::parser::parse_declarations(&mut parser);
        
        for decl in declarations {
            style.apply_declaration(&decl);
        }
        style
    }

    pub fn apply_declaration(&mut self, decl: &crate::engine::style::Declaration) {
        match decl.name.as_str() {
            "display" => {
                self.display = match decl.value.as_str() {
                    "block" => DisplayMode::Block,
                    "inline" => DisplayMode::Inline,
                    "flex" => DisplayMode::Flex,
                    "none" => DisplayMode::None,
                    _ => self.display,
                };
            }
            "flex-direction" => {
                self.flex_direction = match decl.value.as_str() {
                    "row" => FlexDirectionMode::Row,
                    "column" => FlexDirectionMode::Column,
                    "row-reverse" => FlexDirectionMode::RowReverse,
                    "column-reverse" => FlexDirectionMode::ColumnReverse,
                    _ => self.flex_direction,
                };
            }
            "justify-content" => {
                self.justify_content = match decl.value.as_str() {
                    "flex-start" | "start" => JustifyContentMode::FlexStart,
                    "flex-end" | "end" => JustifyContentMode::FlexEnd,
                    "center" => JustifyContentMode::Center,
                    "space-between" => JustifyContentMode::SpaceBetween,
                    "space-around" => JustifyContentMode::SpaceAround,
                    "space-evenly" => JustifyContentMode::SpaceEvenly,
                    _ => self.justify_content,
                };
            }
            "align-items" => {
                self.align_items = match decl.value.as_str() {
                    "flex-start" | "start" => AlignItemsMode::FlexStart,
                    "flex-end" | "end" => AlignItemsMode::FlexEnd,
                    "center" => AlignItemsMode::Center,
                    "baseline" => AlignItemsMode::Baseline,
                    "stretch" => AlignItemsMode::Stretch,
                    _ => self.align_items,
                };
            }
            "color" => self.color = Color::parse(&decl.value),
            "background-color" => self.background_color = Color::parse(&decl.value),
            "width" => self.width = Length::parse(&decl.value),
            "height" => self.height = Length::parse(&decl.value),
            "font-size" => {
                if let Ok(val) = decl.value.strip_suffix("px").unwrap_or(&decl.value).parse::<f32>() {
                    self.font_size = val;
                }
            }
            _ => {}
        }
    }

    pub fn taffy_display(&self) -> taffy::style::Display {
        match self.display {
            DisplayMode::Flex => taffy::style::Display::Flex,
            DisplayMode::None => taffy::style::Display::None,
            _ => taffy::style::Display::Block,
        }
    }

    pub fn to_taffy_style(&self) -> taffy::style::Style {
        taffy::style::Style {
            display: self.taffy_display(),
            flex_direction: self.flex_direction,
            justify_content: Some(self.justify_content),
            align_items: Some(self.align_items),
            size: taffy::geometry::Size {
                width: self.width.to_taffy_dimension(),
                height: self.height.to_taffy_dimension(),
            },
            margin: taffy::geometry::Rect {
                top: self.margin_top.to_taffy_lp_auto(),
                right: self.margin_right.to_taffy_lp_auto(),
                bottom: self.margin_bottom.to_taffy_lp_auto(),
                left: self.margin_left.to_taffy_lp_auto(),
            },
            padding: taffy::geometry::Rect {
                top: self.padding_top.to_taffy_lp(),
                right: self.padding_right.to_taffy_lp(),
                bottom: self.padding_bottom.to_taffy_lp(),
                left: self.padding_left.to_taffy_lp(),
            },
            border: taffy::geometry::Rect {
                top: self.border_width.to_taffy_lp(),
                right: self.border_width.to_taffy_lp(),
                bottom: self.border_width.to_taffy_lp(),
                left: self.border_width.to_taffy_lp(),
            },
            ..Default::default()
        }
    }

    pub fn get(&self, property: &str) -> String {
        match property {
            "display" => format!("{:?}", self.display).to_lowercase(),
            "color" => self.color.to_string(),
            "background-color" => self.background_color.to_string(),
            "width" => format!("{:?}", self.width),
            "height" => format!("{:?}", self.height),
            "font-size" => format!("{}px", self.font_size),
            _ => String::new(),
        }
    }
}
