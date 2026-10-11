//! # Sub-estruturas Temáticas Imutáveis de Estilo (Stylo Model)
//!
//! Separação de propriedades em grupos de herança (Inherited) e reset (Reset),
//! permitindo compartilhamento de ponteiros estáticos globais e drástica redução de alocações na heap.

use crate::values::enums::{
    AlignContent, AlignItems, AlignSelf, BorderStyle, BoxSizing, Direction, Display, FlexDirection,
    FlexWrap, FontStyle, FontWeight, GridAutoFlow, JustifyContent, Overflow, OverflowWrap,
    Position, RectSides, TextAlign, TextDecoration, Visibility, WhiteSpace, WordBreak, WritingMode,
    ZIndex,
};
use ace_core::math::border_radii::BorderRadii;
use ace_core::math::color::Color;
use ace_core::math::layout_unit::LayoutUnit;
use smol_str::SmolStr;
use std::sync::{Arc, LazyLock};

// ---------------------------------------------------------------------------
// 1. Singletons Globais Estáticos para Reset Structs
// ---------------------------------------------------------------------------

pub static INITIAL_BOX: LazyLock<Arc<StyleBox>> = LazyLock::new(|| Arc::new(StyleBox::initial()));
pub static INITIAL_SURROUNDING: LazyLock<Arc<StyleSurrounding>> =
    LazyLock::new(|| Arc::new(StyleSurrounding::initial()));
pub static INITIAL_FONT: LazyLock<Arc<StyleFont>> =
    LazyLock::new(|| Arc::new(StyleFont::initial()));
pub static INITIAL_COLOR: LazyLock<Arc<StyleColor>> =
    LazyLock::new(|| Arc::new(StyleColor::initial()));
pub static INITIAL_TEXT: LazyLock<Arc<StyleText>> =
    LazyLock::new(|| Arc::new(StyleText::initial()));
pub static INITIAL_FLEX: LazyLock<Arc<StyleFlex>> =
    LazyLock::new(|| Arc::new(StyleFlex::initial()));
pub static INITIAL_GRID: LazyLock<Arc<StyleGrid>> =
    LazyLock::new(|| Arc::new(StyleGrid::initial()));

// ---------------------------------------------------------------------------
// 2. Structs Resetadas (Reset Structs)
// ---------------------------------------------------------------------------

/// Propriedades de dimensão, posicionamento e modelo de caixa.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleBox {
    pub display: Display,
    pub position: Position,
    pub box_sizing: BoxSizing,
    pub z_index: ZIndex,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub width: Option<LayoutUnit>,
    pub height: Option<LayoutUnit>,
    pub min_width: Option<LayoutUnit>,
    pub max_width: Option<LayoutUnit>,
    pub min_height: Option<LayoutUnit>,
    pub max_height: Option<LayoutUnit>,
    pub top: Option<LayoutUnit>,
    pub right: Option<LayoutUnit>,
    pub bottom: Option<LayoutUnit>,
    pub left: Option<LayoutUnit>,
}

impl Default for StyleBox {
    fn default() -> Self {
        Self::initial()
    }
}

impl StyleBox {
    pub fn initial() -> Self {
        Self {
            display: Display::Inline,
            position: Position::Static,
            box_sizing: BoxSizing::ContentBox,
            z_index: ZIndex::Auto,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            width: None,
            height: None,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            top: None,
            right: None,
            bottom: None,
            left: None,
        }
    }
}

/// Margens, espaçamentos internos (padding) e bordas da caixa.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleSurrounding {
    pub margin: RectSides<Option<LayoutUnit>>,
    pub padding: RectSides<LayoutUnit>,
    pub border_width: RectSides<LayoutUnit>,
    pub border_style: RectSides<BorderStyle>,
    pub border_color: RectSides<Color>,
    pub border_radius: BorderRadii,
}

impl Default for StyleSurrounding {
    fn default() -> Self {
        Self::initial()
    }
}

impl StyleSurrounding {
    pub fn initial() -> Self {
        Self {
            margin: RectSides::all(Some(LayoutUnit::ZERO)),
            padding: RectSides::all(LayoutUnit::ZERO),
            border_width: RectSides::all(LayoutUnit::ZERO),
            border_style: RectSides::all(BorderStyle::None),
            border_color: RectSides::all(Color::BLACK),
            border_radius: BorderRadii::ZERO,
        }
    }
}

/// Propriedades de Flexbox.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleFlex {
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub align_content: AlignContent,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Option<LayoutUnit>,
    pub order: i32,
    pub row_gap: Option<LayoutUnit>,
    pub column_gap: Option<LayoutUnit>,
}

impl Default for StyleFlex {
    fn default() -> Self {
        Self::initial()
    }
}

impl StyleFlex {
    pub fn initial() -> Self {
        Self {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            align_self: AlignSelf::Auto,
            align_content: AlignContent::Stretch,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: None,
            order: 0,
            row_gap: None,
            column_gap: None,
        }
    }
}

/// Propriedades de Grid Layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleGrid {
    pub grid_auto_flow: GridAutoFlow,
}

impl Default for StyleGrid {
    fn default() -> Self {
        Self::initial()
    }
}

impl StyleGrid {
    pub fn initial() -> Self {
        Self {
            grid_auto_flow: GridAutoFlow::Row,
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Structs Herdadas (Inherited Structs)
// ---------------------------------------------------------------------------

/// Tipografia e métricas de fonte (herdada).
#[derive(Debug, Clone, PartialEq)]
pub struct StyleFont {
    pub size: LayoutUnit,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub family: SmolStr,
    pub line_height: Option<LayoutUnit>,
}

impl Default for StyleFont {
    fn default() -> Self {
        Self::initial()
    }
}

impl StyleFont {
    pub fn initial() -> Self {
        Self {
            size: LayoutUnit::from_px(16),
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
            family: SmolStr::new("sans-serif"),
            line_height: None,
        }
    }
}

/// Cores principais e opacidade.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleColor {
    pub color: Color,
    pub background_color: Color,
    pub opacity: f32,
}

impl Default for StyleColor {
    fn default() -> Self {
        Self::initial()
    }
}

impl StyleColor {
    pub fn initial() -> Self {
        Self {
            color: Color::BLACK,
            background_color: Color::from_rgba(0, 0, 0, 0),
            opacity: 1.0,
        }
    }
}

/// Alinhamento, decoração e quebra de texto, visibilidade e escrita (herdada).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleText {
    pub text_align: TextAlign,
    pub text_decoration: TextDecoration,
    pub white_space: WhiteSpace,
    pub word_break: WordBreak,
    pub overflow_wrap: OverflowWrap,
    pub visibility: Visibility,
    pub direction: Direction,
    pub writing_mode: WritingMode,
}

impl Default for StyleText {
    fn default() -> Self {
        Self::initial()
    }
}

impl StyleText {
    pub fn initial() -> Self {
        Self {
            text_align: TextAlign::Start,
            text_decoration: TextDecoration::None,
            white_space: WhiteSpace::Normal,
            word_break: WordBreak::Normal,
            overflow_wrap: OverflowWrap::Normal,
            visibility: Visibility::Visible,
            direction: Direction::Ltr,
            writing_mode: WritingMode::HorizontalTb,
        }
    }
}
