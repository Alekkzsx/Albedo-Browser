//! # Estilo Computado Imutável com Style Structs (ComputedStyle)
//!
//! Representação modular baseada no modelo Stylo/Quantum CSS com sub-estruturas temáticas
//! protegidas por `Arc` e métodos de conveniência de alta performance para layout e renderização.

use crate::computed::structs::{
    StyleBox, StyleColor, StyleFlex, StyleFont, StyleGrid, StyleSurrounding, StyleText,
    INITIAL_BOX, INITIAL_COLOR, INITIAL_FLEX, INITIAL_FONT, INITIAL_GRID, INITIAL_SURROUNDING,
    INITIAL_TEXT,
};
use crate::values::enums::{
    AlignContent, AlignItems, AlignSelf, BorderStyle, BoxSizing, Direction, Display, FlexDirection,
    FlexWrap, FontStyle, FontWeight, GridAutoFlow, JustifyContent, Overflow, OverflowWrap,
    Position, RectSides, TextAlign, TextDecoration, Visibility, WhiteSpace, WordBreak, WritingMode,
    ZIndex,
};
use ace_core::intern::Atom;
use ace_core::math::border_radii::BorderRadii;
use ace_core::math::color::Color;
use ace_core::math::layout_unit::LayoutUnit;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::Arc;

/// Estilo computado imutável, desacoplado em sub-estruturas temáticas em `Arc`.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    // 1. Box Model & Geometria (Reset)
    pub box_style: Arc<StyleBox>,

    // 2. Margens, Padding e Bordas (Reset)
    pub surrounding: Arc<StyleSurrounding>,

    // 3. Tipografia (Inherited)
    pub font: Arc<StyleFont>,

    // 4. Cores e Opacidade (Inherited / Reset)
    pub color: Arc<StyleColor>,

    // 5. Texto, Alinhamento, Visibilidade e Escrita (Inherited)
    pub text: Arc<StyleText>,

    // 6. Flexbox (Reset)
    pub flex: Arc<StyleFlex>,

    // 7. Grid Layout (Reset)
    pub grid: Arc<StyleGrid>,

    // 8. Conteúdo gerado para pseudo-elementos (`content: ...`)
    pub content: Option<SmolStr>,

    // 9. Custom Properties (--*)
    pub custom_properties: FxHashMap<Atom, SmolStr>,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self::initial()
    }
}

impl ComputedStyle {
    /// Retorna o conjunto de valores iniciais normativos do CSS via ponteiros estáticos globais.
    pub fn initial() -> Self {
        Self {
            box_style: Arc::clone(&INITIAL_BOX),
            surrounding: Arc::clone(&INITIAL_SURROUNDING),
            font: Arc::clone(&INITIAL_FONT),
            color: Arc::clone(&INITIAL_COLOR),
            text: Arc::clone(&INITIAL_TEXT),
            flex: Arc::clone(&INITIAL_FLEX),
            grid: Arc::clone(&INITIAL_GRID),
            content: None,
            custom_properties: FxHashMap::default(),
        }
    }

    // -----------------------------------------------------------------------
    // Métodos de Acesso Rápido Compatíveis com Clientes de Layout e Render
    // -----------------------------------------------------------------------

    #[inline]
    pub fn display(&self) -> Display {
        self.box_style.display
    }

    #[inline]
    pub fn position(&self) -> Position {
        self.box_style.position
    }

    #[inline]
    pub fn box_sizing(&self) -> BoxSizing {
        self.box_style.box_sizing
    }

    #[inline]
    pub fn width(&self) -> Option<LayoutUnit> {
        self.box_style.width
    }

    #[inline]
    pub fn height(&self) -> Option<LayoutUnit> {
        self.box_style.height
    }

    #[inline]
    pub fn min_width(&self) -> Option<LayoutUnit> {
        self.box_style.min_width
    }

    #[inline]
    pub fn max_width(&self) -> Option<LayoutUnit> {
        self.box_style.max_width
    }

    #[inline]
    pub fn min_height(&self) -> Option<LayoutUnit> {
        self.box_style.min_height
    }

    #[inline]
    pub fn max_height(&self) -> Option<LayoutUnit> {
        self.box_style.max_height
    }

    #[inline]
    pub fn top(&self) -> Option<LayoutUnit> {
        self.box_style.top
    }

    #[inline]
    pub fn right(&self) -> Option<LayoutUnit> {
        self.box_style.right
    }

    #[inline]
    pub fn bottom(&self) -> Option<LayoutUnit> {
        self.box_style.bottom
    }

    #[inline]
    pub fn left(&self) -> Option<LayoutUnit> {
        self.box_style.left
    }

    #[inline]
    pub fn z_index(&self) -> ZIndex {
        self.box_style.z_index
    }

    #[inline]
    pub fn overflow_x(&self) -> Overflow {
        self.box_style.overflow_x
    }

    #[inline]
    pub fn overflow_y(&self) -> Overflow {
        self.box_style.overflow_y
    }

    #[inline]
    pub fn margin(&self) -> &RectSides<Option<LayoutUnit>> {
        &self.surrounding.margin
    }

    #[inline]
    pub fn margin_top(&self) -> Option<LayoutUnit> {
        self.surrounding.margin.top
    }

    #[inline]
    pub fn margin_right(&self) -> Option<LayoutUnit> {
        self.surrounding.margin.right
    }

    #[inline]
    pub fn margin_bottom(&self) -> Option<LayoutUnit> {
        self.surrounding.margin.bottom
    }

    #[inline]
    pub fn margin_left(&self) -> Option<LayoutUnit> {
        self.surrounding.margin.left
    }

    #[inline]
    pub fn padding(&self) -> &RectSides<LayoutUnit> {
        &self.surrounding.padding
    }

    #[inline]
    pub fn padding_top(&self) -> LayoutUnit {
        self.surrounding.padding.top
    }

    #[inline]
    pub fn padding_right(&self) -> LayoutUnit {
        self.surrounding.padding.right
    }

    #[inline]
    pub fn padding_bottom(&self) -> LayoutUnit {
        self.surrounding.padding.bottom
    }

    #[inline]
    pub fn padding_left(&self) -> LayoutUnit {
        self.surrounding.padding.left
    }

    #[inline]
    pub fn border_width(&self) -> &RectSides<LayoutUnit> {
        &self.surrounding.border_width
    }

    #[inline]
    pub fn border_style(&self) -> &RectSides<BorderStyle> {
        &self.surrounding.border_style
    }

    #[inline]
    pub fn border_color(&self) -> &RectSides<Color> {
        &self.surrounding.border_color
    }

    #[inline]
    pub fn border_radius(&self) -> &BorderRadii {
        &self.surrounding.border_radius
    }

    #[inline]
    pub fn font_size(&self) -> LayoutUnit {
        self.font.size
    }

    #[inline]
    pub fn font_weight(&self) -> FontWeight {
        self.font.weight
    }

    #[inline]
    pub fn font_style(&self) -> FontStyle {
        self.font.style
    }

    #[inline]
    pub fn font_family(&self) -> &str {
        &self.font.family
    }

    #[inline]
    pub fn line_height(&self) -> Option<LayoutUnit> {
        self.font.line_height
    }

    #[inline]
    pub fn color(&self) -> Color {
        self.color.color
    }

    #[inline]
    pub fn background_color(&self) -> Color {
        self.color.background_color
    }

    #[inline]
    pub fn opacity(&self) -> f32 {
        self.color.opacity
    }

    #[inline]
    pub fn text_align(&self) -> TextAlign {
        self.text.text_align
    }

    #[inline]
    pub fn text_decoration(&self) -> TextDecoration {
        self.text.text_decoration
    }

    #[inline]
    pub fn white_space(&self) -> WhiteSpace {
        self.text.white_space
    }

    #[inline]
    pub fn word_break(&self) -> WordBreak {
        self.text.word_break
    }

    #[inline]
    pub fn overflow_wrap(&self) -> OverflowWrap {
        self.text.overflow_wrap
    }

    #[inline]
    pub fn visibility(&self) -> Visibility {
        self.text.visibility
    }

    #[inline]
    pub fn direction(&self) -> Direction {
        self.text.direction
    }

    #[inline]
    pub fn writing_mode(&self) -> WritingMode {
        self.text.writing_mode
    }

    #[inline]
    pub fn flex_direction(&self) -> FlexDirection {
        self.flex.flex_direction
    }

    #[inline]
    pub fn flex_wrap(&self) -> FlexWrap {
        self.flex.flex_wrap
    }

    #[inline]
    pub fn justify_content(&self) -> JustifyContent {
        self.flex.justify_content
    }

    #[inline]
    pub fn align_items(&self) -> AlignItems {
        self.flex.align_items
    }

    #[inline]
    pub fn align_self(&self) -> AlignSelf {
        self.flex.align_self
    }

    #[inline]
    pub fn align_content(&self) -> AlignContent {
        self.flex.align_content
    }

    #[inline]
    pub fn flex_grow(&self) -> f32 {
        self.flex.flex_grow
    }

    #[inline]
    pub fn flex_shrink(&self) -> f32 {
        self.flex.flex_shrink
    }

    #[inline]
    pub fn flex_basis(&self) -> Option<LayoutUnit> {
        self.flex.flex_basis
    }

    #[inline]
    pub fn order(&self) -> i32 {
        self.flex.order
    }

    #[inline]
    pub fn row_gap(&self) -> Option<LayoutUnit> {
        self.flex.row_gap
    }

    #[inline]
    pub fn column_gap(&self) -> Option<LayoutUnit> {
        self.flex.column_gap
    }

    #[inline]
    pub fn grid_auto_flow(&self) -> GridAutoFlow {
        self.grid.grid_auto_flow
    }

    #[inline]
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// Retorna `true` se o elemento não for renderizado (`display: none`).
    #[inline]
    pub fn is_display_none(&self) -> bool {
        self.box_style.display == Display::None
    }
}
