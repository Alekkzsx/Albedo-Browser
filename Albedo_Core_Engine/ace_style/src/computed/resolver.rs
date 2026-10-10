//! # Resolução de Valores Computados e Palavras-chave Universais
//!
//! Converte valores da cascata em `ComputedStyle`, aplicando `var()`, `inherit`, `initial`, `unset` e `revert`.

use crate::cascade::sorter::CascadeDeclaration;
use crate::computed::inherit::{inherit_from_parent, is_inherited_property};
use crate::computed::style::ComputedStyle;
use crate::computed::variables::VariableResolver;
use crate::values::color::CssColor;
use crate::values::enums::*;
use crate::values::length::Length;
use ace_core::intern::Atom;
use ace_core::math::layout_unit::LayoutUnit;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

/// Converte um conjunto de declarações vencedoras em um `ComputedStyle` final.
pub fn resolve_computed_style(
    winning_declarations: &FxHashMap<Atom, CascadeDeclaration>,
    parent_style: Option<&ComputedStyle>,
    root_font_size_px: f32,
    viewport_size: (f32, f32),
) -> ComputedStyle {
    let mut style = ComputedStyle::initial();

    if let Some(parent) = parent_style {
        inherit_from_parent(&mut style, parent);
    }

    // 1. Resolução preliminar de Custom Properties (--*)
    let mut custom_vars = style.custom_properties.clone();
    for (prop, decl) in winning_declarations {
        if prop.as_str().starts_with("--") {
            custom_vars.insert(prop.clone(), decl.value.clone());
        }
    }

    let mut var_resolver = VariableResolver::new(&custom_vars);
    style.custom_properties = var_resolver.resolve_all();

    // 2. Extrai font_size corrente para resolução de 'em'
    let mut font_size_px = style.font_size.to_f32_px();

    // Se houver declaração de font-size, resolve primeiro para garantir cálculo correto de 'em'
    if let Some(decl) = winning_declarations.get(&Atom::new("font-size")) {
        if let Some(val) = resolve_value_str(&decl.value, &mut var_resolver) {
            apply_font_size(&mut style, &val, parent_style, root_font_size_px, viewport_size);
            font_size_px = style.font_size.to_f32_px();
        }
    }

    // 3. Aplicação das demais propriedades
    for (prop, decl) in winning_declarations {
        let prop_name = prop.as_str();
        if prop_name.starts_with("--") || prop_name == "font-size" {
            continue;
        }

        let Some(val_str) = resolve_value_str(&decl.value, &mut var_resolver) else {
            continue;
        };

        // Palavras-chave universais
        if val_str == "inherit" {
            if let Some(parent) = parent_style {
                apply_inherit_keyword(&mut style, parent, prop_name);
            }
            continue;
        } else if val_str == "initial" {
            apply_initial_keyword(&mut style, prop_name);
            continue;
        } else if val_str == "unset" {
            if is_inherited_property(prop_name) {
                if let Some(parent) = parent_style {
                    apply_inherit_keyword(&mut style, parent, prop_name);
                }
            } else {
                apply_initial_keyword(&mut style, prop_name);
            }
            continue;
        }

        apply_property(
            &mut style,
            prop_name,
            &val_str,
            font_size_px,
            root_font_size_px,
            viewport_size,
        );
    }

    style
}

fn resolve_value_str(raw: &str, resolver: &mut VariableResolver) -> Option<String> {
    resolver
        .substitute_vars_in_str(raw, 0)
        .map(|s| s.trim().to_string())
}

fn apply_font_size(
    style: &mut ComputedStyle,
    val: &str,
    parent_style: Option<&ComputedStyle>,
    root_font_size_px: f32,
    viewport_size: (f32, f32),
) {
    let parent_font_size = parent_style
        .map(|p| p.font_size.to_f32_px())
        .unwrap_or(16.0);

    if let Some(len) = Length::parse(val) {
        if let Some(unit) = len.to_layout_unit_with_ref(
            parent_font_size,
            root_font_size_px,
            viewport_size,
            Some(parent_font_size),
        ) {
            style.font_size = unit;
        }
    }
}

fn apply_property(
    style: &mut ComputedStyle,
    prop: &str,
    val: &str,
    font_size_px: f32,
    root_font_size_px: f32,
    viewport_size: (f32, f32),
) {
    let to_unit = |v: &str| -> Option<LayoutUnit> {
        Length::parse(v).and_then(|l| l.to_layout_unit(font_size_px, root_font_size_px, viewport_size))
    };

    match prop {
        // Box Model
        "display" => {
            if let Some(d) = Display::parse(val) {
                style.display = d;
            }
        }
        "position" => {
            if let Some(p) = Position::parse(val) {
                style.position = p;
            }
        }
        "box-sizing" => {
            if let Some(b) = BoxSizing::parse(val) {
                style.box_sizing = b;
            }
        }
        "width" => style.width = to_unit(val),
        "height" => style.height = to_unit(val),
        "min-width" => style.min_width = to_unit(val),
        "max-width" => style.max_width = to_unit(val),
        "min-height" => style.min_height = to_unit(val),
        "max-height" => style.max_height = to_unit(val),

        // Margins
        "margin-top" => style.margin.top = to_unit(val),
        "margin-right" => style.margin.right = to_unit(val),
        "margin-bottom" => style.margin.bottom = to_unit(val),
        "margin-left" => style.margin.left = to_unit(val),

        // Paddings
        "padding-top" => {
            if let Some(u) = to_unit(val) {
                style.padding.top = u;
            }
        }
        "padding-right" => {
            if let Some(u) = to_unit(val) {
                style.padding.right = u;
            }
        }
        "padding-bottom" => {
            if let Some(u) = to_unit(val) {
                style.padding.bottom = u;
            }
        }
        "padding-left" => {
            if let Some(u) = to_unit(val) {
                style.padding.left = u;
            }
        }

        // Borders
        "border-top-width" => {
            if let Some(u) = to_unit(val) {
                style.border_width.top = u;
            }
        }
        "border-right-width" => {
            if let Some(u) = to_unit(val) {
                style.border_width.right = u;
            }
        }
        "border-bottom-width" => {
            if let Some(u) = to_unit(val) {
                style.border_width.bottom = u;
            }
        }
        "border-left-width" => {
            if let Some(u) = to_unit(val) {
                style.border_width.left = u;
            }
        }
        "border-top-style" => {
            if let Some(s) = BorderStyle::parse(val) {
                style.border_style.top = s;
            }
        }
        "border-right-style" => {
            if let Some(s) = BorderStyle::parse(val) {
                style.border_style.right = s;
            }
        }
        "border-bottom-style" => {
            if let Some(s) = BorderStyle::parse(val) {
                style.border_style.bottom = s;
            }
        }
        "border-left-style" => {
            if let Some(s) = BorderStyle::parse(val) {
                style.border_style.left = s;
            }
        }
        "border-top-color" => {
            if let Some(c) = CssColor::parse(val) {
                style.border_color.top = c.resolve(style.color);
            }
        }
        "border-right-color" => {
            if let Some(c) = CssColor::parse(val) {
                style.border_color.right = c.resolve(style.color);
            }
        }
        "border-bottom-color" => {
            if let Some(c) = CssColor::parse(val) {
                style.border_color.bottom = c.resolve(style.color);
            }
        }
        "border-left-color" => {
            if let Some(c) = CssColor::parse(val) {
                style.border_color.left = c.resolve(style.color);
            }
        }

        // Colors
        "color" => {
            if let Some(c) = CssColor::parse(val) {
                style.color = c.resolve(style.color);
            }
        }
        "background-color" => {
            if let Some(c) = CssColor::parse(val) {
                style.background_color = c.resolve(style.color);
            }
        }

        // Typography
        "font-weight" => {
            if let Some(w) = FontWeight::parse(val) {
                style.font_weight = w;
            }
        }
        "font-style" => {
            if let Some(s) = FontStyle::parse(val) {
                style.font_style = s;
            }
        }
        "font-family" => style.font_family = SmolStr::new(val),
        "text-align" => {
            if let Some(a) = TextAlign::parse(val) {
                style.text_align = a;
            }
        }
        "text-decoration" => {
            if let Some(d) = TextDecoration::parse(val) {
                style.text_decoration = d;
            }
        }
        "white-space" => {
            if let Some(w) = WhiteSpace::parse(val) {
                style.white_space = w;
            }
        }
        "word-break" => {
            if let Some(w) = WordBreak::parse(val) {
                style.word_break = w;
            }
        }
        "overflow-wrap" => {
            if let Some(w) = OverflowWrap::parse(val) {
                style.overflow_wrap = w;
            }
        }

        // Positioning & Visibility
        "top" => style.top = to_unit(val),
        "right" => style.right = to_unit(val),
        "bottom" => style.bottom = to_unit(val),
        "left" => style.left = to_unit(val),
        "visibility" => {
            if let Some(v) = Visibility::parse(val) {
                style.visibility = v;
            }
        }
        "opacity" => {
            if let Ok(o) = val.parse::<f32>() {
                style.opacity = o.clamp(0.0, 1.0);
            }
        }
        "z-index" => {
            if let Some(z) = ZIndex::parse(val) {
                style.z_index = z;
            }
        }
        "overflow-x" => {
            if let Some(o) = Overflow::parse(val) {
                style.overflow_x = o;
            }
        }
        "overflow-y" => {
            if let Some(o) = Overflow::parse(val) {
                style.overflow_y = o;
            }
        }

        // Flexbox
        "flex-direction" => {
            if let Some(d) = FlexDirection::parse(val) {
                style.flex_direction = d;
            }
        }
        "flex-wrap" => {
            if let Some(w) = FlexWrap::parse(val) {
                style.flex_wrap = w;
            }
        }
        "justify-content" => {
            if let Some(j) = JustifyContent::parse(val) {
                style.justify_content = j;
            }
        }
        "align-items" => {
            if let Some(a) = AlignItems::parse(val) {
                style.align_items = a;
            }
        }
        "align-self" => {
            if let Some(a) = AlignSelf::parse(val) {
                style.align_self = a;
            }
        }
        "align-content" => {
            if let Some(a) = AlignContent::parse(val) {
                style.align_content = a;
            }
        }
        "flex-grow" => {
            if let Ok(g) = val.parse::<f32>() {
                style.flex_grow = g.max(0.0);
            }
        }
        "flex-shrink" => {
            if let Ok(s) = val.parse::<f32>() {
                style.flex_shrink = s.max(0.0);
            }
        }
        "flex-basis" => style.flex_basis = to_unit(val),
        "order" => {
            if let Ok(o) = val.parse::<i32>() {
                style.order = o;
            }
        }
        "row-gap" => style.row_gap = to_unit(val),
        "column-gap" => style.column_gap = to_unit(val),

        // Grid
        "grid-auto-flow" => {
            if let Some(g) = GridAutoFlow::parse(val) {
                style.grid_auto_flow = g;
            }
        }

        _ => {}
    }
}

fn apply_inherit_keyword(style: &mut ComputedStyle, parent: &ComputedStyle, prop: &str) {
    match prop {
        "color" => style.color = parent.color,
        "font-size" => style.font_size = parent.font_size,
        "font-weight" => style.font_weight = parent.font_weight,
        "font-style" => style.font_style = parent.font_style,
        "font-family" => style.font_family = parent.font_family.clone(),
        "line-height" => style.line_height = parent.line_height,
        "text-align" => style.text_align = parent.text_align,
        "visibility" => style.visibility = parent.visibility,
        "white-space" => style.white_space = parent.white_space,
        "word-break" => style.word_break = parent.word_break,
        "overflow-wrap" => style.overflow_wrap = parent.overflow_wrap,
        "display" => style.display = parent.display,
        "position" => style.position = parent.position,
        "box-sizing" => style.box_sizing = parent.box_sizing,
        "width" => style.width = parent.width,
        "height" => style.height = parent.height,
        "opacity" => style.opacity = parent.opacity,
        _ => {}
    }
}

fn apply_initial_keyword(style: &mut ComputedStyle, prop: &str) {
    let initial = ComputedStyle::initial();
    match prop {
        "color" => style.color = initial.color,
        "font-size" => style.font_size = initial.font_size,
        "font-weight" => style.font_weight = initial.font_weight,
        "font-style" => style.font_style = initial.font_style,
        "font-family" => style.font_family = initial.font_family,
        "line-height" => style.line_height = initial.line_height,
        "text-align" => style.text_align = initial.text_align,
        "visibility" => style.visibility = initial.visibility,
        "white-space" => style.white_space = initial.white_space,
        "word-break" => style.word_break = initial.word_break,
        "overflow-wrap" => style.overflow_wrap = initial.overflow_wrap,
        "display" => style.display = initial.display,
        "position" => style.position = initial.position,
        "box-sizing" => style.box_sizing = initial.box_sizing,
        "width" => style.width = initial.width,
        "height" => style.height = initial.height,
        "opacity" => style.opacity = initial.opacity,
        _ => {}
    }
}
