//! # Estilo Computado Imutável e Denso (`ComputedStyle`)
//!
//! Representação densa de todas as propriedades do Box Model, Flexbox, Grid, cores e tipografia.

use crate::values::enums::{
    AlignContent, AlignItems, AlignSelf, BorderStyle, BoxSizing, Display, FlexDirection, FlexWrap,
    FontStyle, FontWeight, GridAutoFlow, JustifyContent, Overflow, OverflowWrap, Position,
    RectSides, TextAlign, TextDecoration, Visibility, WhiteSpace, WordBreak, ZIndex,
};
use ace_core::intern::Atom;
use ace_core::math::border_radii::BorderRadii;
use ace_core::math::color::Color;
use ace_core::math::layout_unit::LayoutUnit;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

/// Estilo computado imutável, denso e alinhado de um elemento da árvore de renderização.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    // 1. Box Model & Dimensões
    pub display: Display,
    pub position: Position,
    pub box_sizing: BoxSizing,
    pub width: Option<LayoutUnit>,
    pub height: Option<LayoutUnit>,
    pub min_width: Option<LayoutUnit>,
    pub max_width: Option<LayoutUnit>,
    pub min_height: Option<LayoutUnit>,
    pub max_height: Option<LayoutUnit>,
    pub margin: RectSides<Option<LayoutUnit>>,
    pub padding: RectSides<LayoutUnit>,
    pub border_width: RectSides<LayoutUnit>,
    pub border_style: RectSides<BorderStyle>,
    pub border_color: RectSides<Color>,
    pub border_radius: BorderRadii,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub visibility: Visibility,
    pub opacity: f32,
    pub z_index: ZIndex,
    pub top: Option<LayoutUnit>,
    pub right: Option<LayoutUnit>,
    pub bottom: Option<LayoutUnit>,
    pub left: Option<LayoutUnit>,

    // 2. Cores & Tipografia
    pub color: Color,
    pub background_color: Color,
    pub font_size: LayoutUnit,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_family: SmolStr,
    pub line_height: Option<LayoutUnit>,
    pub text_align: TextAlign,
    pub text_decoration: TextDecoration,
    pub white_space: WhiteSpace,
    pub word_break: WordBreak,
    pub overflow_wrap: OverflowWrap,

    // 3. Flexbox
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

    // 4. Grid
    pub grid_auto_flow: GridAutoFlow,

    // 5. Custom Properties (--*)
    pub custom_properties: FxHashMap<Atom, SmolStr>,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self::initial()
    }
}

impl ComputedStyle {
    /// Retorna o conjunto de valores iniciais normativos do CSS.
    pub fn initial() -> Self {
        Self {
            display: Display::Inline,
            position: Position::Static,
            box_sizing: BoxSizing::ContentBox,
            width: None,
            height: None,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            margin: RectSides::all(Some(LayoutUnit::ZERO)),
            padding: RectSides::all(LayoutUnit::ZERO),
            border_width: RectSides::all(LayoutUnit::ZERO),
            border_style: RectSides::all(BorderStyle::None),
            border_color: RectSides::all(Color::BLACK),
            border_radius: BorderRadii::ZERO,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            visibility: Visibility::Visible,
            opacity: 1.0,
            z_index: ZIndex::Auto,
            top: None,
            right: None,
            bottom: None,
            left: None,

            color: Color::BLACK,
            background_color: Color::from_rgba(0, 0, 0, 0),
            font_size: LayoutUnit::from_px(16),
            font_weight: FontWeight::NORMAL,
            font_style: FontStyle::Normal,
            font_family: SmolStr::new("sans-serif"),
            line_height: None,
            text_align: TextAlign::Start,
            text_decoration: TextDecoration::None,
            white_space: WhiteSpace::Normal,
            word_break: WordBreak::Normal,
            overflow_wrap: OverflowWrap::Normal,

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

            grid_auto_flow: GridAutoFlow::Row,

            custom_properties: FxHashMap::default(),
        }
    }

    /// Retorna `true` se o elemento não for renderizado (`display: none`).
    #[inline]
    pub fn is_display_none(&self) -> bool {
        self.display == Display::None
    }
}
