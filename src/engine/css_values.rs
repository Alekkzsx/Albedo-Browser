use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum CssLength {
    Px(f32),
    Percent(f32),
    Vw(f32),
    Vh(f32),
    Rem(f32),
    Em(f32),
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
            CssLength::Percent(v) => write!(f, "{}%", v),
            CssLength::Vw(v) => write!(f, "{}vw", v),
            CssLength::Vh(v) => write!(f, "{}vh", v),
            CssLength::Rem(v) => write!(f, "{}rem", v),
            CssLength::Em(v) => write!(f, "{}em", v),
            CssLength::Auto => write!(f, "auto"),
            CssLength::Zero => write!(f, "0"),
        }
    }
}

/// Resolve CSS length to pixels given a base font size and viewport
pub fn resolve_length(length: &CssLength, base_font_size: f32, viewport_width: f32, viewport_height: f32) -> f32 {
    match length {
        CssLength::Px(v) => *v,
        CssLength::Percent(v) => *v / 100.0, // This needs context - simplified
        CssLength::Vw(v) => *v / 100.0 * viewport_width,
        CssLength::Vh(v) => *v / 100.0 * viewport_height,
        CssLength::Rem(v) => *v * base_font_size,
        CssLength::Em(v) => *v * base_font_size,
        CssLength::Zero => 0.0,
        CssLength::Auto => 0.0, // Auto resolves to 0 for position calculations
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssColor {
    Named(String),
    CurrentColor,
    Transparent,
    Rgba(u8, u8, u8, f32), // r, g, b, alpha
}

impl Default for CssColor {
    fn default() -> Self {
        Self::Named("black".to_string()) // CanvasText equivalent
    }
}

impl CssColor {
    pub fn to_rgba_string(&self) -> String {
        match self {
            CssColor::Named(name) => {
                // Simple color name to hex mapping
                match name.to_lowercase().as_str() {
                    "white" => "#ffffff".to_string(),
                    "black" => "#000000".to_string(),
                    "red" => "#ff0000".to_string(),
                    "green" => "#008000".to_string(),
                    "blue" => "#0000ff".to_string(),
                    "yellow" => "#ffff00".to_string(),
                    "gray" | "grey" => "#808080".to_string(),
                    "silver" => "#c0c0c0".to_string(),
                    "transparent" => "transparent".to_string(),
                    _ => format!("#000000"), // Default to black
                }
            },
            CssColor::CurrentColor => "#000000".to_string(),
            CssColor::Transparent => "transparent".to_string(),
            CssColor::Rgba(r, g, b, a) => {
                if *a < 1.0 {
                    format!("rgba({},{},{},{})", r, g, b, a)
                } else {
                    format!("#{:02x}{:02x}{:02x}", r, g, b)
                }
            }
        }
    }
}

/// Text shadow structure for text-shadow CSS property
#[derive(Debug, Clone, PartialEq)]
pub struct TextShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: CssColor,
}

impl Default for TextShadow {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            blur: 0.0,
            color: CssColor::Named("black".to_string()),
        }
    }
}

/// Gradient color stop
#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    pub color: CssColor,
    pub position: Option<f32>, // 0.0 to 1.0
}

/// Gradient type
#[derive(Debug, Clone, PartialEq)]
pub enum Gradient {
    Linear {
        angle: f32, // degrees
        stops: Vec<GradientStop>,
    },
    Radial {
        shape: String, // circle or ellipse
        stops: Vec<GradientStop>,
    },
}

/// Background image - supports colors, gradients, and URLs
#[derive(Debug, Clone, PartialEq)]
pub enum BackgroundImage {
    None,
    Color(CssColor),
    Gradient(Gradient),
    Url(String),
}

impl Default for BackgroundImage {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssDisplay {
    None,
    Block,
    Inline,
    InlineBlock,
    Flex,
    InlineFlex,
    Grid,
}

impl Default for CssDisplay {
    fn default() -> Self {
        Self::Inline
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssPosition {
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

impl Default for CssPosition {
    fn default() -> Self {
        Self::Static
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssOverflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl Default for CssOverflow {
    fn default() -> Self {
        Self::Visible
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssFloat {
    None,
    Left,
    Right,
}

impl Default for CssFloat {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssTextAlign {
    Left,
    Right,
    Center,
    Justify,
    Start,
    End,
}

impl Default for CssTextAlign {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssFontWeight {
    Normal,
    Bold,
    Lighter,
    Bolder,
    Weight(f32),
}

impl Default for CssFontWeight {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssFlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl Default for CssFlexDirection {
    fn default() -> Self {
        Self::Row
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssJustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl Default for CssJustifyContent {
    fn default() -> Self {
        Self::FlexStart
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssAlignItems {
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

impl Default for CssAlignItems {
    fn default() -> Self {
        Self::Stretch
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssFlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

impl Default for CssFlexWrap {
    fn default() -> Self {
        Self::NoWrap
    }
}

#[derive(Debug, Clone)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: CssColor,
    pub inset: bool,
}

impl Default for BoxShadow {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            blur: 0.0,
            spread: 0.0,
            color: CssColor::Named("black".to_string()),
            inset: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    // Display & Layout
    pub display: CssDisplay,
    pub position: CssPosition,
    pub overflow: CssOverflow,
    pub z_index: i32,
    pub float: CssFloat,
    
    // Box Model - Dimensions
    pub width: CssLength,
    pub height: CssLength,
    pub min_width: CssLength,
    pub max_width: CssLength,
    pub min_height: CssLength,
    pub max_height: CssLength,
    
    // Box Model - Position (for absolute/fixed)
    pub top: CssLength,
    pub right: CssLength,
    pub bottom: CssLength,
    pub left: CssLength,
    
    // Box Model - Margins
    pub margin_top: CssLength,
    pub margin_right: CssLength,
    pub margin_bottom: CssLength,
    pub margin_left: CssLength,
    
    // Box Model - Padding
    pub padding_top: CssLength,
    pub padding_right: CssLength,
    pub padding_bottom: CssLength,
    pub padding_left: CssLength,
    
    // Border
    pub border_width_top: CssLength,
    pub border_width_right: CssLength,
    pub border_width_bottom: CssLength,
    pub border_width_left: CssLength,
    pub border_color_top: CssColor,
    pub border_color_right: CssColor,
    pub border_color_bottom: CssColor,
    pub border_color_left: CssColor,
    pub border_radius_top_left: f32,
    pub border_radius_top_right: f32,
    pub border_radius_bottom_right: f32,
    pub border_radius_bottom_left: f32,
    
    // Visual
    pub color: CssColor,
    pub background_color: CssColor,
    pub background_image: BackgroundImage,
    pub opacity: f32,
    pub box_shadow: Vec<BoxShadow>,
    pub text_shadow: Vec<TextShadow>,
    pub line_height: CssLength,
    
    // Typography
    pub font_size: CssLength,
    pub font_family: String,
    pub font_weight: CssFontWeight,
    pub text_align: CssTextAlign,
    
    // Flexbox
    pub flex_direction: CssFlexDirection,
    pub justify_content: CssJustifyContent,
    pub align_items: CssAlignItems,
    pub flex_wrap: CssFlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: CssLength,
    
    // Custom properties (CSS Variables)
    pub custom_properties: std::collections::HashMap<String, String>,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            // Display & Layout
            display: CssDisplay::Inline,
            position: CssPosition::Static,
            overflow: CssOverflow::Visible,
            z_index: 0,
            float: CssFloat::None,
            
            // Dimensions
            width: CssLength::Auto,
            height: CssLength::Auto,
            min_width: CssLength::Zero,
            max_width: CssLength::Auto,
            min_height: CssLength::Zero,
            max_height: CssLength::Auto,
            
            // Position
            top: CssLength::Auto,
            right: CssLength::Auto,
            bottom: CssLength::Auto,
            left: CssLength::Auto,
            
            // Margins
            margin_top: CssLength::Zero,
            margin_right: CssLength::Zero,
            margin_bottom: CssLength::Zero,
            margin_left: CssLength::Zero,
            
            // Padding
            padding_top: CssLength::Zero,
            padding_right: CssLength::Zero,
            padding_bottom: CssLength::Zero,
            padding_left: CssLength::Zero,
            
            // Border
            border_width_top: CssLength::Zero,
            border_width_right: CssLength::Zero,
            border_width_bottom: CssLength::Zero,
            border_width_left: CssLength::Zero,
            border_color_top: CssColor::Named("black".to_string()),
            border_color_right: CssColor::Named("black".to_string()),
            border_color_bottom: CssColor::Named("black".to_string()),
            border_color_left: CssColor::Named("black".to_string()),
            border_radius_top_left: 0.0,
            border_radius_top_right: 0.0,
            border_radius_bottom_right: 0.0,
            border_radius_bottom_left: 0.0,
            
            // Visual
            color: CssColor::Named("black".to_string()),
            background_color: CssColor::Transparent,
            background_image: BackgroundImage::None,
            opacity: 1.0,
            box_shadow: Vec::new(),
            text_shadow: Vec::new(),
            line_height: CssLength::Px(1.2), // Default line-height
            
            // Typography
            font_size: CssLength::Px(16.0),
            font_family: "sans-serif".to_string(),
            font_weight: CssFontWeight::Normal,
            text_align: CssTextAlign::Left,
            
            // Flexbox
            flex_direction: CssFlexDirection::Row,
            justify_content: CssJustifyContent::FlexStart,
            align_items: CssAlignItems::Stretch,
            flex_wrap: CssFlexWrap::NoWrap,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: CssLength::Auto,
            
            // Custom properties
            custom_properties: std::collections::HashMap::new(),
        }
    }
}
