use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum CssLength {
    Px(f32),
    Percent(f32),
    Vw(f32),
    Vh(f32),
    Rem(f32),
    Em(f32),
    Fr(f32),
    Auto,
    Zero,
    Clamp(Box<CssLength>, Box<CssLength>, Box<CssLength>),
    Min(Vec<CssLength>),
    Max(Vec<CssLength>),
    MinMax(Box<CssLength>, Box<CssLength>),
    Repeat(String, Vec<CssLength>),
    MinContent,
    MaxContent,
    AutoFill,
    AutoFit,
    Calc(String),
    LineNames(Vec<String>),
    Name(String),
    Number(f32),
    Subgrid,
    Span(u16),
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
            CssLength::Fr(v) => write!(f, "{}fr", v),
            CssLength::Auto => write!(f, "auto"),
            CssLength::Zero => write!(f, "0"),
            CssLength::Clamp(min, val, max) => write!(f, "clamp({}, {}, {})", min, val, max),
            CssLength::Min(vals) => {
                let s: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                write!(f, "min({})", s.join(", "))
            }
            CssLength::Max(vals) => {
                let s: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                write!(f, "max({})", s.join(", "))
            }
            CssLength::Calc(s) => write!(f, "calc({})", s),
            CssLength::MinMax(min, max) => write!(f, "minmax({}, {})", min, max),
            CssLength::Repeat(count, sub) => {
                let s: Vec<String> = sub.iter().map(|v| v.to_string()).collect();
                write!(f, "repeat({}, {})", count, s.join(", "))
            }
            CssLength::MinContent => write!(f, "min-content"),
            CssLength::MaxContent => write!(f, "max-content"),
            CssLength::AutoFill => write!(f, "auto-fill"),
            CssLength::AutoFit => write!(f, "auto-fit"),
            CssLength::LineNames(names) => write!(f, "[{}]", names.join(" ")),
            CssLength::Name(name) => write!(f, "{}", name),
            CssLength::Number(v) => write!(f, "{}", v),
            CssLength::Subgrid => write!(f, "subgrid"),
            CssLength::Span(v) => write!(f, "span {}", v),
        }
    }
}

/// Resolve CSS length to pixels given parent and root font sizes
pub fn resolve_length(
    length: &CssLength,
    parent_font_size: f32,
    root_font_size: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> f32 {
    match length {
        CssLength::Px(v) => *v,
        CssLength::Percent(v) => *v / 100.0, // This needs context - usually handled by layout engine
        CssLength::Vw(v) => *v / 100.0 * viewport_width,
        CssLength::Vh(v) => *v / 100.0 * viewport_height,
        CssLength::Rem(v) => *v * root_font_size,
        CssLength::Em(v) => *v * parent_font_size,
        CssLength::Fr(v) => *v,
        CssLength::Zero => 0.0,
        CssLength::Auto => 0.0,
        CssLength::Number(v) => *v,
        CssLength::Subgrid => 0.0,
        CssLength::Span(v) => *v as f32,
        CssLength::Name(_) | CssLength::LineNames(_) => 0.0,
        CssLength::Clamp(min, val, max) => {
            let min_v = resolve_length(
                min,
                parent_font_size,
                root_font_size,
                viewport_width,
                viewport_height,
            );
            let val_v = resolve_length(
                val,
                parent_font_size,
                root_font_size,
                viewport_width,
                viewport_height,
            );
            let max_v = resolve_length(
                max,
                parent_font_size,
                root_font_size,
                viewport_width,
                viewport_height,
            );
            val_v.max(min_v).min(max_v)
        }
        CssLength::Min(vals) => vals
            .iter()
            .map(|v| {
                resolve_length(
                    v,
                    parent_font_size,
                    root_font_size,
                    viewport_width,
                    viewport_height,
                )
            })
            .fold(f32::INFINITY, f32::min),
        CssLength::Max(vals) => vals
            .iter()
            .map(|v| {
                resolve_length(
                    v,
                    parent_font_size,
                    root_font_size,
                    viewport_width,
                    viewport_height,
                )
            })
            .fold(f32::NEG_INFINITY, f32::max),
        CssLength::MinMax(min, max) => {
            let _min_v = resolve_length(
                min,
                parent_font_size,
                root_font_size,
                viewport_width,
                viewport_height,
            );
            let max_v = resolve_length(
                max,
                parent_font_size,
                root_font_size,
                viewport_width,
                viewport_height,
            );
            // Rough approximation: use max for now
            max_v
        }
        CssLength::Repeat(_count, sub) => {
            // Rough approximation: resolve first element * 3 (arbitrary)
            if let Some(first) = sub.first() {
                resolve_length(
                    first,
                    parent_font_size,
                    root_font_size,
                    viewport_width,
                    viewport_height,
                ) * 3.0
            } else {
                0.0
            }
        }
        CssLength::MinContent
        | CssLength::MaxContent
        | CssLength::AutoFill
        | CssLength::AutoFit => 0.0, // Needs layout context
        CssLength::Calc(_) => 0.0, // Needs complex parser
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
                let lower = name.to_lowercase();
                // Se já é hex (#rrggbb), retorna direto
                if lower.starts_with('#') {
                    return name.clone();
                }
                match lower.as_str() {
                    "white" => "#ffffff".to_string(),
                    "black" => "#000000".to_string(),
                    "red" => "#ff0000".to_string(),
                    "green" => "#008000".to_string(),
                    "blue" => "#0000ff".to_string(),
                    "yellow" => "#ffff00".to_string(),
                    "cyan" | "aqua" => "#00ffff".to_string(),
                    "magenta" | "fuchsia" => "#ff00ff".to_string(),
                    "orange" => "#ffa500".to_string(),
                    "pink" => "#ffc0cb".to_string(),
                    "purple" => "#800080".to_string(),
                    "brown" => "#a52a2a".to_string(),
                    "gray" | "grey" => "#808080".to_string(),
                    "silver" => "#c0c0c0".to_string(),
                    "navy" => "#000080".to_string(),
                    "teal" => "#008080".to_string(),
                    "maroon" => "#800000".to_string(),
                    "olive" => "#808000".to_string(),
                    "lime" => "#00ff00".to_string(),
                    "coral" => "#ff7f50".to_string(),
                    "gold" => "#ffd700".to_string(),
                    "crimson" => "#dc143c".to_string(),
                    "indigo" => "#4b0082".to_string(),
                    "violet" => "#ee82ee".to_string(),
                    "turquoise" => "#40e0d0".to_string(),
                    "salmon" => "#fa8072".to_string(),
                    "khaki" => "#f0e68c".to_string(),
                    "plum" => "#dda0dd".to_string(),
                    "orchid" => "#da70d6".to_string(),
                    "tomato" => "#ff6347".to_string(),
                    "beige" => "#f5f5dc".to_string(),
                    "ivory" => "#fffff0".to_string(),
                    "linen" => "#faf0e6".to_string(),
                    "wheat" => "#f5deb3".to_string(),
                    // Dark variants
                    "darkblue" => "#00008b".to_string(),
                    "darkgreen" => "#006400".to_string(),
                    "darkred" => "#8b0000".to_string(),
                    "darkcyan" => "#008b8b".to_string(),
                    "darkmagenta" => "#8b008b".to_string(),
                    "darkorange" => "#ff8c00".to_string(),
                    "darkviolet" => "#9400d3".to_string(),
                    "darkgray" | "darkgrey" => "#a9a9a9".to_string(),
                    "darkkhaki" => "#bdb76b".to_string(),
                    "darkolivegreen" => "#556b2f".to_string(),
                    "darkorchid" => "#9932cc".to_string(),
                    "darksalmon" => "#e9967a".to_string(),
                    "darkseagreen" => "#8fbc8f".to_string(),
                    "darkslateblue" => "#483d8b".to_string(),
                    "darkslategray" | "darkslategrey" => "#2f4f4f".to_string(),
                    "darkturquoise" => "#00ced1".to_string(),
                    "darkgoldenrod" => "#b8860b".to_string(),
                    // Light variants
                    "lightblue" => "#add8e6".to_string(),
                    "lightgreen" => "#90ee90".to_string(),
                    "lightgray" | "lightgrey" => "#d3d3d3".to_string(),
                    "lightcoral" => "#f08080".to_string(),
                    "lightcyan" => "#e0ffff".to_string(),
                    "lightyellow" => "#ffffe0".to_string(),
                    "lightpink" => "#ffb6c1".to_string(),
                    "lightsalmon" => "#ffa07a".to_string(),
                    "lightseagreen" => "#20b2aa".to_string(),
                    "lightskyblue" => "#87cefa".to_string(),
                    "lightsteelblue" => "#b0c4de".to_string(),
                    // Medium variants
                    "mediumblue" => "#0000cd".to_string(),
                    "mediumseagreen" => "#3cb371".to_string(),
                    "mediumslateblue" => "#7b68ee".to_string(),
                    "mediumspringgreen" => "#00fa9a".to_string(),
                    "mediumturquoise" => "#48d1cc".to_string(),
                    "mediumvioletred" => "#c71585".to_string(),
                    "mediumorchid" => "#ba55d3".to_string(),
                    "mediumpurple" => "#9370db".to_string(),
                    "mediumaquamarine" => "#66cdaa".to_string(),
                    // Others
                    "whitesmoke" => "#f5f5f5".to_string(),
                    "ghostwhite" => "#f8f8ff".to_string(),
                    "aliceblue" => "#f0f8ff".to_string(),
                    "lavender" => "#e6e6fa".to_string(),
                    "mistyrose" => "#ffe4e1".to_string(),
                    "snow" => "#fffafa".to_string(),
                    "seashell" => "#fff5ee".to_string(),
                    "mintcream" => "#f5fffa".to_string(),
                    "honeydew" => "#f0fff0".to_string(),
                    "azure" => "#f0ffff".to_string(),
                    "floralwhite" => "#fffaf0".to_string(),
                    "antiquewhite" => "#faebd7".to_string(),
                    "cornsilk" => "#fff8dc".to_string(),
                    "blanchedalmond" => "#ffebcd".to_string(),
                    "bisque" => "#ffe4c4".to_string(),
                    "navajowhite" => "#ffdead".to_string(),
                    "moccasin" => "#ffe4b5".to_string(),
                    "papayawhip" => "#ffefd5".to_string(),
                    "lemonchiffon" => "#fffacd".to_string(),
                    "oldlace" => "#fdf5e6".to_string(),
                    "peachpuff" => "#ffdab9".to_string(),
                    "powderblue" => "#b0e0e6".to_string(),
                    "rosybrown" => "#bc8f8f".to_string(),
                    "royalblue" => "#4169e1".to_string(),
                    "saddlebrown" => "#8b4513".to_string(),
                    "sandybrown" => "#f4a460".to_string(),
                    "seagreen" => "#2e8b57".to_string(),
                    "sienna" => "#a0522d".to_string(),
                    "skyblue" => "#87ceeb".to_string(),
                    "slateblue" => "#6a5acd".to_string(),
                    "slategray" | "slategrey" => "#708090".to_string(),
                    "springgreen" => "#00ff7f".to_string(),
                    "steelblue" => "#4682b4".to_string(),
                    "tan" => "#d2b48c".to_string(),
                    "thistle" => "#d8bfd8".to_string(),
                    "yellowgreen" => "#9acd32".to_string(),
                    "rebeccapurple" => "#663399".to_string(),
                    "transparent" => "transparent".to_string(),
                    "cornflowerblue" => "#6495ed".to_string(),
                    "cadetblue" => "#5f9ea0".to_string(),
                    "chartreuse" => "#7fff00".to_string(),
                    "chocolate" => "#d2691e".to_string(),
                    "deeppink" => "#ff1493".to_string(),
                    "deepskyblue" => "#00bfff".to_string(),
                    "dimgray" | "dimgrey" => "#696969".to_string(),
                    "dodgerblue" => "#1e90ff".to_string(),
                    "firebrick" => "#b22222".to_string(),
                    "forestgreen" => "#228b22".to_string(),
                    "gainsboro" => "#dcdcdc".to_string(),
                    "greenyellow" => "#adff2f".to_string(),
                    "hotpink" => "#ff69b4".to_string(),
                    "indianred" => "#cd5c5c".to_string(),
                    "lawngreen" => "#7cfc00".to_string(),
                    "limegreen" => "#32cd32".to_string(),
                    "midnightblue" => "#191970".to_string(),
                    "olivedrab" => "#6b8e23".to_string(),
                    "orangered" => "#ff4500".to_string(),
                    "palegreen" => "#98fb98".to_string(),
                    "paleturquoise" => "#afeeee".to_string(),
                    "palevioletred" => "#db7093".to_string(),
                    "peru" => "#cd853f".to_string(),
                    _ => "#000000".to_string(),
                }
            }
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

#[derive(Debug, Clone, PartialEq)]
pub enum CssVerticalAlign {
    Baseline,
    Sub,
    Super,
    Top,
    TextTop,
    Middle,
    Bottom,
    TextBottom,
    Length(CssLength),
}

impl Default for CssVerticalAlign {
    fn default() -> Self {
        CssVerticalAlign::Baseline
    }
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
    Contents,
    Table,
    TableRow,
    TableCell,
    TableHeader,
}

impl Default for CssDisplay {
    fn default() -> Self {
        Self::Inline
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssTextTransform {
    None,
    Uppercase,
    Lowercase,
    Capitalize,
}

impl Default for CssTextTransform {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssTextOverflow {
    Clip,
    Ellipsis,
}

impl Default for CssTextOverflow {
    fn default() -> Self {
        Self::Clip
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssWhiteSpace {
    Normal,
    NoWrap,
    Pre,
    PreWrap,
    PreLine,
}

impl Default for CssWhiteSpace {
    fn default() -> Self {
        Self::Normal
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
pub enum CssClear {
    None,
    Left,
    Right,
    Both,
}

impl Default for CssClear {
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

impl CssFontWeight {
    pub fn to_cosmic(&self) -> cosmic_text::Weight {
        match self {
            CssFontWeight::Normal => cosmic_text::Weight::NORMAL,
            CssFontWeight::Bold => cosmic_text::Weight::BOLD,
            CssFontWeight::Lighter => cosmic_text::Weight::THIN,
            CssFontWeight::Bolder => cosmic_text::Weight::EXTRA_BOLD,
            CssFontWeight::Weight(w) => cosmic_text::Weight(*w as u16),
        }
    }
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
    Auto, // For align-self
}

impl Default for CssAlignItems {
    fn default() -> Self {
        Self::Stretch
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssAlignContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    Stretch,
}

impl Default for CssAlignContent {
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
#[derive(Debug, Clone, PartialEq)]
pub enum CssBoxSizing {
    ContentBox,
    BorderBox,
}

impl Default for CssBoxSizing {
    fn default() -> Self {
        Self::ContentBox
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssVisibility {
    Visible,
    Hidden,
    Collapse,
}

impl Default for CssVisibility {
    fn default() -> Self {
        Self::Visible
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssCursor {
    Auto,
    Default,
    Pointer,
    Text,
    Wait,
    Help,
    NotAllowed,
    Grab,
    Grabbing,
}

impl Default for CssCursor {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssObjectFit {
    Fill,
    Contain,
    Cover,
    None,
    ScaleDown,
}

impl Default for CssObjectFit {
    fn default() -> Self {
        Self::Fill
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CssObjectPosition {
    pub x: CssLength,
    pub y: CssLength,
}

impl Default for CssObjectPosition {
    fn default() -> Self {
        Self {
            x: CssLength::Percent(50.0),
            y: CssLength::Percent(50.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssPointerEvents {
    Auto,
    None,
}

impl Default for CssPointerEvents {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransformFunction {
    Translate(CssLength, CssLength),
    TranslateX(CssLength),
    TranslateY(CssLength),
    Scale(f32, f32),
    Rotate(f32), // degrees
    RotateX(f32),
    RotateY(f32),
    RotateZ(f32),
    Skew(f32, f32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CssTransition {
    pub property: String,
    pub duration_ms: u32,
    pub timing_function: String, // e.g., "ease", "linear"
    pub delay_ms: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CssAnimation {
    pub name: String,
    pub duration_ms: u32,
    pub timing_function: String,
    pub delay_ms: u32,
    pub iteration_count: String, // "infinite" or number
    pub direction: String,       // "normal", "reverse", "alternate"...
    pub fill_mode: String,       // "none", "forwards", "backwards", "both"
}

#[derive(Debug, Clone, PartialEq)]
pub struct CssKeyframe {
    pub percentage: f32, // 0.0 to 100.0
    pub declarations: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: CssColor,
    pub inset: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    pub width: f32,
    pub color: CssColor,
    pub style: String, // "solid", "dashed", "dotted", etc.
    pub offset: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssFilter {
    Blur(CssLength),
    Brightness(f32),
    Contrast(f32),
    Grayscale(f32),
    HueRotate(f32), // degrees
    Invert(f32),
    Opacity(f32),
    Saturate(f32),
    Sepia(f32),
    DropShadow(BoxShadow),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssBlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl Default for CssBlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

/// CSS content property for pseudo-elements (::before, ::after)
#[derive(Debug, Clone, PartialEq)]
pub enum CssContent {
    None,
    Normal,
    String(String),
    // Future: Url, Counter, Attr, etc.
}

impl Default for CssContent {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    // Display & Layout
    pub display: CssDisplay,
    pub position: CssPosition,
    pub overflow: CssOverflow,
    pub z_index: i32,
    pub float: CssFloat,
    pub clear: CssClear,

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

    pub text_transform: CssTextTransform,
    pub text_overflow: CssTextOverflow,
    pub white_space: CssWhiteSpace,

    // Backgroundel - Margins
    pub margin_top: CssLength,
    pub margin_right: CssLength,
    pub margin_bottom: CssLength,
    pub margin_left: CssLength,

    // Box Model - Padding
    pub padding_top: CssLength,
    pub padding_right: CssLength,
    pub padding_bottom: CssLength,
    pub padding_left: CssLength,

    // Spacing
    pub letter_spacing: CssLength,
    pub word_spacing: CssLength,

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
    pub outline: Option<Outline>,
    pub line_height: CssLength,

    // Typography
    pub font_size: f32,
    pub font_family: String,
    pub font_weight: CssFontWeight,
    pub font_style: String,
    pub text_align: CssTextAlign,

    // Flexbox
    pub flex_direction: CssFlexDirection,
    pub justify_content: CssJustifyContent,
    pub align_items: CssAlignItems,
    pub flex_wrap: CssFlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: CssLength,

    // Grid Layout
    pub grid_template_columns: Vec<CssLength>,
    pub grid_template_rows: Vec<CssLength>,
    pub grid_column_start: CssLength,
    pub grid_column_end: CssLength,
    pub grid_row_start: CssLength,
    pub grid_row_end: CssLength,
    pub grid_column_gap: CssLength,
    pub grid_row_gap: CssLength,
    pub grid_template_areas: Vec<String>,

    // Flexbox/Grid alignment
    pub order: i32,
    pub align_self: CssAlignItems,
    pub align_content: CssAlignContent,

    // Pseudo-element content
    pub content: CssContent,

    // Modern CSS
    pub aspect_ratio: Option<f32>,
    pub box_sizing: CssBoxSizing,
    pub visibility: CssVisibility,
    pub cursor: CssCursor,
    pub pointer_events: CssPointerEvents,
    pub transform: Vec<TransformFunction>,
    pub object_fit: CssObjectFit,
    pub object_position: CssObjectPosition,
    pub filters: Vec<CssFilter>,
    pub backdrop_filters: Vec<CssFilter>,
    pub mix_blend_mode: CssBlendMode,
    pub transitions: Vec<CssTransition>,
    pub animations: Vec<CssAnimation>,
    pub clip_path: Option<String>,

    // Custom properties (CSS Variables)
    pub custom_properties: std::collections::HashMap<String, String>,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            // Display & Layout
            position: CssPosition::Static,
            overflow: CssOverflow::Visible,
            z_index: 0,
            float: CssFloat::None,
            clear: CssClear::None,

            // Dimensions
            width: CssLength::Auto,
            height: CssLength::Auto,
            min_width: CssLength::Zero,
            max_width: CssLength::Auto,
            min_height: CssLength::Zero,
            max_height: CssLength::Auto,

            display: CssDisplay::default(),

            // Position
            top: CssLength::Auto,
            right: CssLength::Auto,
            bottom: CssLength::Auto,
            left: CssLength::Auto,

            text_transform: CssTextTransform::default(),
            text_overflow: CssTextOverflow::default(),
            white_space: CssWhiteSpace::default(),

            // Backgrounds
            margin_top: CssLength::Zero,
            margin_right: CssLength::Zero,
            margin_bottom: CssLength::Zero,
            margin_left: CssLength::Zero,

            // Padding
            padding_top: CssLength::Zero,
            padding_right: CssLength::Zero,
            padding_bottom: CssLength::Zero,
            padding_left: CssLength::Zero,

            // Spacing
            letter_spacing: CssLength::Zero,
            word_spacing: CssLength::Zero,

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
            outline: None,
            line_height: CssLength::Px(1.2), // Default line-height

            // Typography
            font_size: 16.0,
            font_family: "sans-serif".to_string(),
            font_weight: CssFontWeight::Normal,
            font_style: "normal".to_string(),
            text_align: CssTextAlign::Left,

            // Flexbox
            flex_direction: CssFlexDirection::Row,
            justify_content: CssJustifyContent::FlexStart,
            align_items: CssAlignItems::Stretch,
            flex_wrap: CssFlexWrap::NoWrap,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: CssLength::Auto,

            // Grid Default
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            grid_column_start: CssLength::Auto,
            grid_column_end: CssLength::Auto,
            grid_row_start: CssLength::Auto,
            grid_row_end: CssLength::Auto,
            grid_column_gap: CssLength::Zero,
            grid_row_gap: CssLength::Zero,

            // Pseudo-element content
            content: CssContent::Normal,

            aspect_ratio: None,
            box_sizing: CssBoxSizing::ContentBox,
            visibility: CssVisibility::Visible,
            cursor: CssCursor::Auto,
            pointer_events: CssPointerEvents::Auto,
            transform: Vec::new(),
            object_fit: CssObjectFit::Fill,
            object_position: CssObjectPosition::default(),
            filters: Vec::new(),
            backdrop_filters: Vec::new(),
            mix_blend_mode: CssBlendMode::Normal,
            transitions: Vec::new(),
            animations: Vec::new(),
            clip_path: None,

            // Custom properties
            custom_properties: std::collections::HashMap::new(),

            // New Flexbox/Grid properties
            order: 0,
            align_self: CssAlignItems::Auto, // Default is auto, which computes to parent's align-items
            align_content: CssAlignContent::Stretch,
            grid_template_areas: Vec::new(),
        }
    }
}
