
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum CssLength {
    Px(f32),
    Em(f32),
    Rem(f32),
    Percent(f32),
    Vh(f32),
    Vw(f32),
    Auto,
    Zero,
}

impl Default for CssLength {
    fn default() -> Self {
        Self::Auto
    }
}

impl fmt::Display for CssLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CssLength::Px(v) => write!(f, "{}px", v),
            CssLength::Em(v) => write!(f, "{}em", v),
            CssLength::Rem(v) => write!(f, "{}rem", v),
            CssLength::Percent(v) => write!(f, "{}%", v),
            CssLength::Vh(v) => write!(f, "{}vh", v),
            CssLength::Vw(v) => write!(f, "{}vw", v),
            CssLength::Auto => write!(f, "auto"),
            CssLength::Zero => write!(f, "0"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssColor {
    Named(String),
    Rgba(u8, u8, u8, f32),
    CurrentColor,
    Transparent,
}

impl Default for CssColor {
    fn default() -> Self {
        Self::Named("black".to_string()) // CanvasText equivalent
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssDisplay {
    None,
    Block,
    Inline,
    InlineBlock,
    Flex,
    Grid,
    // ...
}

impl Default for CssDisplay {
    fn default() -> Self {
        Self::Inline
    }
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub display: CssDisplay,
    pub color: CssColor,
    pub background_color: CssColor,
    pub font_size: CssLength,
    pub width: CssLength,
    pub height: CssLength,
    pub margin_top: CssLength,
    pub margin_right: CssLength,
    pub margin_bottom: CssLength,
    pub margin_left: CssLength,
    pub padding_top: CssLength,
    pub padding_right: CssLength,
    pub padding_bottom: CssLength,
    pub padding_left: CssLength,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: CssDisplay::Inline, // Initial value for display is inline (usually)
            color: CssColor::Named("black".to_string()),
            background_color: CssColor::Transparent,
            font_size: CssLength::Px(16.0), // Medium
            width: CssLength::Auto,
            height: CssLength::Auto,
            margin_top: CssLength::Zero,
            margin_right: CssLength::Zero,
            margin_bottom: CssLength::Zero,
            margin_left: CssLength::Zero,
            padding_top: CssLength::Zero,
            padding_right: CssLength::Zero,
            padding_bottom: CssLength::Zero,
            padding_left: CssLength::Zero,
        }
    }
}
