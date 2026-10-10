//! # Enums Normativos do CSS (Box Model, Flexbox, Grid, Tipografia)
//!
//! Tipos enumerados fortemente tipados para propriedades normativas do CSS.

/// Estrutura genérica representando os 4 lados de uma caixa CSS (topo, direita, baixo, esquerda).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RectSides<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T> RectSides<T> {
    pub const fn new(top: T, right: T, bottom: T, left: T) -> Self {
        Self { top, right, bottom, left }
    }
}

impl<T: Clone> RectSides<T> {
    pub fn all(val: T) -> Self {
        Self {
            top: val.clone(),
            right: val.clone(),
            bottom: val.clone(),
            left: val,
        }
    }
}

/// Comportamento de exibição do elemento (`display`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Display {
    #[default]
    Inline,
    Block,
    InlineBlock,
    Flex,
    InlineFlex,
    Grid,
    InlineGrid,
    Table,
    TableRow,
    TableCell,
    ListItem,
    None,
}

impl Display {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_block(&self) -> bool {
        matches!(self, Self::Block | Self::Table | Self::Flex | Self::Grid)
    }

    pub fn is_flex(&self) -> bool {
        matches!(self, Self::Flex | Self::InlineFlex)
    }

    pub fn is_grid(&self) -> bool {
        matches!(self, Self::Grid | Self::InlineGrid)
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "inline" => Some(Self::Inline),
            "block" => Some(Self::Block),
            "inline-block" => Some(Self::InlineBlock),
            "flex" => Some(Self::Flex),
            "inline-flex" => Some(Self::InlineFlex),
            "grid" => Some(Self::Grid),
            "inline-grid" => Some(Self::InlineGrid),
            "table" => Some(Self::Table),
            "table-row" => Some(Self::TableRow),
            "table-cell" => Some(Self::TableCell),
            "list-item" => Some(Self::ListItem),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

/// Modelo de posicionamento do elemento (`position`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

impl Position {
    pub fn is_positioned(&self) -> bool {
        !matches!(self, Self::Static)
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "static" => Some(Self::Static),
            "relative" => Some(Self::Relative),
            "absolute" => Some(Self::Absolute),
            "fixed" => Some(Self::Fixed),
            "sticky" => Some(Self::Sticky),
            _ => None,
        }
    }
}

/// Modelo de cálculo de caixa (`box-sizing`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BoxSizing {
    #[default]
    ContentBox,
    BorderBox,
}

impl BoxSizing {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "content-box" => Some(Self::ContentBox),
            "border-box" => Some(Self::BorderBox),
            _ => None,
        }
    }
}

/// Comportamento de transbordamento (`overflow`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl Overflow {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "visible" => Some(Self::Visible),
            "hidden" => Some(Self::Hidden),
            "scroll" => Some(Self::Scroll),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

/// Visibilidade do elemento (`visibility`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
    Collapse,
}

impl Visibility {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "visible" => Some(Self::Visible),
            "hidden" => Some(Self::Hidden),
            "collapse" => Some(Self::Collapse),
            _ => None,
        }
    }
}

/// Direção do fluxo flexível (`flex-direction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl FlexDirection {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "row" => Some(Self::Row),
            "row-reverse" => Some(Self::RowReverse),
            "column" => Some(Self::Column),
            "column-reverse" => Some(Self::ColumnReverse),
            _ => None,
        }
    }
}

/// Quebra de linha em contêiner flexível (`flex-wrap`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

impl FlexWrap {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "nowrap" => Some(Self::NoWrap),
            "wrap" => Some(Self::Wrap),
            "wrap-reverse" => Some(Self::WrapReverse),
            _ => None,
        }
    }
}

/// Alinhamento no eixo principal (`justify-content`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl JustifyContent {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "flex-start" | "start" => Some(Self::FlexStart),
            "flex-end" | "end" => Some(Self::FlexEnd),
            "center" => Some(Self::Center),
            "space-between" => Some(Self::SpaceBetween),
            "space-around" => Some(Self::SpaceAround),
            "space-evenly" => Some(Self::SpaceEvenly),
            _ => None,
        }
    }
}

/// Alinhamento no eixo cruzado para itens (`align-items`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlignItems {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

impl AlignItems {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "stretch" => Some(Self::Stretch),
            "flex-start" | "start" => Some(Self::FlexStart),
            "flex-end" | "end" => Some(Self::FlexEnd),
            "center" => Some(Self::Center),
            "baseline" => Some(Self::Baseline),
            _ => None,
        }
    }
}

/// Alinhamento individual no eixo cruzado (`align-self`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlignSelf {
    #[default]
    Auto,
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

impl AlignSelf {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "stretch" => Some(Self::Stretch),
            "flex-start" | "start" => Some(Self::FlexStart),
            "flex-end" | "end" => Some(Self::FlexEnd),
            "center" => Some(Self::Center),
            "baseline" => Some(Self::Baseline),
            _ => None,
        }
    }
}

/// Alinhamento de múltiplas linhas flexíveis (`align-content`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlignContent {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
}

impl AlignContent {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "stretch" => Some(Self::Stretch),
            "flex-start" | "start" => Some(Self::FlexStart),
            "flex-end" | "end" => Some(Self::FlexEnd),
            "center" => Some(Self::Center),
            "space-between" => Some(Self::SpaceBetween),
            "space-around" => Some(Self::SpaceAround),
            _ => None,
        }
    }
}

/// Fluxo automático em grade CSS (`grid-auto-flow`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
    Dense,
    RowDense,
    ColumnDense,
}

impl GridAutoFlow {
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim().to_ascii_lowercase();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.contains(&"row") && parts.contains(&"dense") {
            Some(Self::RowDense)
        } else if parts.contains(&"column") && parts.contains(&"dense") {
            Some(Self::ColumnDense)
        } else if parts == ["row"] {
            Some(Self::Row)
        } else if parts == ["column"] {
            Some(Self::Column)
        } else if parts == ["dense"] {
            Some(Self::Dense)
        } else {
            None
        }
    }
}

/// Alinhamento horizontal de texto (`text-align`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAlign {
    #[default]
    Start,
    Left,
    Right,
    Center,
    Justify,
    End,
}

impl TextAlign {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "start" => Some(Self::Start),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "center" => Some(Self::Center),
            "justify" => Some(Self::Justify),
            "end" => Some(Self::End),
            _ => None,
        }
    }
}

/// Decoração de texto (`text-decoration`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    LineThrough,
    Overline,
}

impl TextDecoration {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "underline" => Some(Self::Underline),
            "line-through" => Some(Self::LineThrough),
            "overline" => Some(Self::Overline),
            _ => None,
        }
    }
}

/// Tratamento de espaços em branco (`white-space`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WhiteSpace {
    #[default]
    Normal,
    NoWrap,
    Pre,
    PreWrap,
    PreLine,
}

impl WhiteSpace {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "nowrap" => Some(Self::NoWrap),
            "pre" => Some(Self::Pre),
            "pre-wrap" => Some(Self::PreWrap),
            "pre-line" => Some(Self::PreLine),
            _ => None,
        }
    }
}

/// Quebra de palavras (`word-break`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WordBreak {
    #[default]
    Normal,
    BreakWord,
    BreakAll,
    Anywhere,
}

impl WordBreak {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "break-word" => Some(Self::BreakWord),
            "break-all" => Some(Self::BreakAll),
            "anywhere" => Some(Self::Anywhere),
            _ => None,
        }
    }
}

/// Quebra de estouro (`overflow-wrap`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OverflowWrap {
    #[default]
    Normal,
    BreakWord,
    Anywhere,
}

impl OverflowWrap {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "break-word" => Some(Self::BreakWord),
            "anywhere" => Some(Self::Anywhere),
            _ => None,
        }
    }
}

/// Estilo de fonte (`font-style`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

impl FontStyle {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "italic" => Some(Self::Italic),
            "oblique" => Some(Self::Oblique),
            _ => None,
        }
    }
}

/// Peso da fonte (`font-weight`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontWeight(pub u16);

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const EXTRA_LIGHT: Self = Self(200);
    pub const LIGHT: Self = Self(300);
    pub const NORMAL: Self = Self(400);
    pub const MEDIUM: Self = Self(500);
    pub const SEMI_BOLD: Self = Self(600);
    pub const BOLD: Self = Self(700);
    pub const EXTRA_BOLD: Self = Self(800);
    pub const BLACK: Self = Self(900);

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Some(Self::NORMAL),
            "bold" => Some(Self::BOLD),
            "lighter" => Some(Self(300)),
            "bolder" => Some(Self(700)),
            val => val.parse::<u16>().ok().map(Self),
        }
    }
}

/// Estilo de borda (`border-style`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BorderStyle {
    #[default]
    None,
    Hidden,
    Dotted,
    Dashed,
    Solid,
    Double,
    Groove,
    Ridge,
    Inset,
    Outset,
}

impl BorderStyle {
    pub fn is_visible(&self) -> bool {
        !matches!(self, Self::None | Self::Hidden)
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "hidden" => Some(Self::Hidden),
            "dotted" => Some(Self::Dotted),
            "dashed" => Some(Self::Dashed),
            "solid" => Some(Self::Solid),
            "double" => Some(Self::Double),
            "groove" => Some(Self::Groove),
            "ridge" => Some(Self::Ridge),
            "inset" => Some(Self::Inset),
            "outset" => Some(Self::Outset),
            _ => None,
        }
    }
}

/// Índice de empilhamento z-index (`z-index`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ZIndex {
    #[default]
    Auto,
    Integer(i32),
}

impl ZIndex {
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.eq_ignore_ascii_case("auto") {
            Some(Self::Auto)
        } else {
            trimmed.parse::<i32>().ok().map(Self::Integer)
        }
    }
}
