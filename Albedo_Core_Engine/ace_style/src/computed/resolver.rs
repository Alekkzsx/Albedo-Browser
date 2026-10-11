//! # Resolução de Valores Computados e Propriedades Lógicas
//!
//! Converte valores da cascata em `ComputedStyle`, aplicando `var()`, matemática W3C (`calc`, `clamp`, `min`, `max`, etc.),
//! mapeamento bidi de propriedades lógicas (`inline-size`, `margin-inline`, etc.) e resolução de keywords universais.

use crate::cascade::sorter::CascadeDeclaration;
use crate::computed::inherit::{inherit_from_parent, is_inherited_property};
use crate::computed::style::ComputedStyle;
use crate::computed::variables::VariableResolver;
use crate::values::color::CssColor;
use crate::values::enums::*;
use crate::values::length::LengthPercentage;
use ace_core::intern::Atom;
use ace_core::math::layout_unit::LayoutUnit;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::Arc;

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
    let mut font_size_px = style.font_size().to_f32_px();

    // Se houver declaração de font-size, resolve primeiro para garantir cálculo correto de 'em'
    if let Some(decl) = winning_declarations.get(&Atom::new("font-size")) {
        if let Some(val) = resolve_value_str(&decl.value, &mut var_resolver) {
            apply_font_size(
                &mut style,
                &val,
                parent_style,
                root_font_size_px,
                viewport_size,
            );
            font_size_px = style.font_size().to_f32_px();
        }
    }

    // 3. Resolve direção e writing-mode primeiro para correto mapeamento de propriedades lógicas
    if let Some(decl) = winning_declarations.get(&Atom::new("direction")) {
        if let Some(val) = resolve_value_str(&decl.value, &mut var_resolver) {
            if let Some(dir) = Direction::parse(&val) {
                Arc::make_mut(&mut style.text).direction = dir;
            }
        }
    }

    if let Some(decl) = winning_declarations.get(&Atom::new("writing-mode")) {
        if let Some(val) = resolve_value_str(&decl.value, &mut var_resolver) {
            if let Some(wm) = WritingMode::parse(&val) {
                Arc::make_mut(&mut style.text).writing_mode = wm;
            }
        }
    }

    // 4. Resolve 'color' preliminarmente para suporte dynamic a 'currentColor' em bordas e fundos
    if let Some(decl) = winning_declarations.get(&Atom::new("color")) {
        if let Some(val) = resolve_value_str(&decl.value, &mut var_resolver) {
            if val != "inherit" && val != "initial" && val != "unset" {
                if let Some(c) = CssColor::parse(&val) {
                    Arc::make_mut(&mut style.color).color = c.resolve(style.color());
                }
            }
        }
    }

    // 5. Aplicação das demais propriedades
    for (prop, decl) in winning_declarations {
        let prop_name = prop.as_str();
        if prop_name.starts_with("--")
            || prop_name == "font-size"
            || prop_name == "direction"
            || prop_name == "writing-mode"
        {
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
        .map(|p| p.font_size().to_f32_px())
        .unwrap_or(16.0);

    if let Some(lp) = LengthPercentage::parse(val) {
        if let Some(unit) = lp.resolve(
            parent_font_size,
            root_font_size_px,
            viewport_size,
            Some(parent_font_size),
        ) {
            Arc::make_mut(&mut style.font).size = unit;
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
        LengthPercentage::parse(v)
            .and_then(|l| l.resolve(font_size_px, root_font_size_px, viewport_size, None))
    };

    let dir = style.direction();
    let wm = style.writing_mode();

    match prop {
        // Box Model & Geometria Física
        "display" => {
            if let Some(d) = Display::parse(val) {
                Arc::make_mut(&mut style.box_style).display = d;
            }
        }
        "position" => {
            if let Some(p) = Position::parse(val) {
                Arc::make_mut(&mut style.box_style).position = p;
            }
        }
        "box-sizing" => {
            if let Some(b) = BoxSizing::parse(val) {
                Arc::make_mut(&mut style.box_style).box_sizing = b;
            }
        }
        "width" => Arc::make_mut(&mut style.box_style).width = to_unit(val),
        "height" => Arc::make_mut(&mut style.box_style).height = to_unit(val),
        "min-width" => Arc::make_mut(&mut style.box_style).min_width = to_unit(val),
        "max-width" => Arc::make_mut(&mut style.box_style).max_width = to_unit(val),
        "min-height" => Arc::make_mut(&mut style.box_style).min_height = to_unit(val),
        "max-height" => Arc::make_mut(&mut style.box_style).max_height = to_unit(val),

        // Dimensões Lógicas (W3C CSS Logical Properties & Values L1)
        "inline-size" => {
            if wm.is_vertical() {
                Arc::make_mut(&mut style.box_style).height = to_unit(val);
            } else {
                Arc::make_mut(&mut style.box_style).width = to_unit(val);
            }
        }
        "min-inline-size" => {
            if wm.is_vertical() {
                Arc::make_mut(&mut style.box_style).min_height = to_unit(val);
            } else {
                Arc::make_mut(&mut style.box_style).min_width = to_unit(val);
            }
        }
        "max-inline-size" => {
            if wm.is_vertical() {
                Arc::make_mut(&mut style.box_style).max_height = to_unit(val);
            } else {
                Arc::make_mut(&mut style.box_style).max_width = to_unit(val);
            }
        }
        "block-size" => {
            if wm.is_vertical() {
                Arc::make_mut(&mut style.box_style).width = to_unit(val);
            } else {
                Arc::make_mut(&mut style.box_style).height = to_unit(val);
            }
        }
        "min-block-size" => {
            if wm.is_vertical() {
                Arc::make_mut(&mut style.box_style).min_width = to_unit(val);
            } else {
                Arc::make_mut(&mut style.box_style).min_height = to_unit(val);
            }
        }
        "max-block-size" => {
            if wm.is_vertical() {
                Arc::make_mut(&mut style.box_style).max_width = to_unit(val);
            } else {
                Arc::make_mut(&mut style.box_style).max_height = to_unit(val);
            }
        }

        // Margens Físicas
        "margin-top" => Arc::make_mut(&mut style.surrounding).margin.top = to_unit(val),
        "margin-right" => Arc::make_mut(&mut style.surrounding).margin.right = to_unit(val),
        "margin-bottom" => Arc::make_mut(&mut style.surrounding).margin.bottom = to_unit(val),
        "margin-left" => Arc::make_mut(&mut style.surrounding).margin.left = to_unit(val),

        // Margens Lógicas (Bidi & Writing Modes)
        "margin-inline-start" => {
            let u = to_unit(val);
            let surr = Arc::make_mut(&mut style.surrounding);
            if wm.is_vertical() {
                surr.margin.top = u;
            } else if dir.is_rtl() {
                surr.margin.right = u;
            } else {
                surr.margin.left = u;
            }
        }
        "margin-inline-end" => {
            let u = to_unit(val);
            let surr = Arc::make_mut(&mut style.surrounding);
            if wm.is_vertical() {
                surr.margin.bottom = u;
            } else if dir.is_rtl() {
                surr.margin.left = u;
            } else {
                surr.margin.right = u;
            }
        }
        "margin-block-start" => {
            let u = to_unit(val);
            let surr = Arc::make_mut(&mut style.surrounding);
            if wm == WritingMode::VerticalRl {
                surr.margin.right = u;
            } else if wm == WritingMode::VerticalLr {
                surr.margin.left = u;
            } else {
                surr.margin.top = u;
            }
        }
        "margin-block-end" => {
            let u = to_unit(val);
            let surr = Arc::make_mut(&mut style.surrounding);
            if wm == WritingMode::VerticalRl {
                surr.margin.left = u;
            } else if wm == WritingMode::VerticalLr {
                surr.margin.right = u;
            } else {
                surr.margin.bottom = u;
            }
        }

        // Paddings Físicos
        "padding-top" => {
            if let Some(u) = to_unit(val) {
                Arc::make_mut(&mut style.surrounding).padding.top = u;
            }
        }
        "padding-right" => {
            if let Some(u) = to_unit(val) {
                Arc::make_mut(&mut style.surrounding).padding.right = u;
            }
        }
        "padding-bottom" => {
            if let Some(u) = to_unit(val) {
                Arc::make_mut(&mut style.surrounding).padding.bottom = u;
            }
        }
        "padding-left" => {
            if let Some(u) = to_unit(val) {
                Arc::make_mut(&mut style.surrounding).padding.left = u;
            }
        }

        // Paddings Lógicos
        "padding-inline-start" => {
            if let Some(u) = to_unit(val) {
                let surr = Arc::make_mut(&mut style.surrounding);
                if wm.is_vertical() {
                    surr.padding.top = u;
                } else if dir.is_rtl() {
                    surr.padding.right = u;
                } else {
                    surr.padding.left = u;
                }
            }
        }
        "padding-inline-end" => {
            if let Some(u) = to_unit(val) {
                let surr = Arc::make_mut(&mut style.surrounding);
                if wm.is_vertical() {
                    surr.padding.bottom = u;
                } else if dir.is_rtl() {
                    surr.padding.left = u;
                } else {
                    surr.padding.right = u;
                }
            }
        }
        "padding-block-start" => {
            if let Some(u) = to_unit(val) {
                let surr = Arc::make_mut(&mut style.surrounding);
                if wm == WritingMode::VerticalRl {
                    surr.padding.right = u;
                } else if wm == WritingMode::VerticalLr {
                    surr.padding.left = u;
                } else {
                    surr.padding.top = u;
                }
            }
        }
        "padding-block-end" => {
            if let Some(u) = to_unit(val) {
                let surr = Arc::make_mut(&mut style.surrounding);
                if wm == WritingMode::VerticalRl {
                    surr.padding.left = u;
                } else if wm == WritingMode::VerticalLr {
                    surr.padding.right = u;
                } else {
                    surr.padding.bottom = u;
                }
            }
        }

        // Posicionamento Inset Lógico
        "inset-inline-start" => {
            let u = to_unit(val);
            let b = Arc::make_mut(&mut style.box_style);
            if wm.is_vertical() {
                b.top = u;
            } else if dir.is_rtl() {
                b.right = u;
            } else {
                b.left = u;
            }
        }
        "inset-inline-end" => {
            let u = to_unit(val);
            let b = Arc::make_mut(&mut style.box_style);
            if wm.is_vertical() {
                b.bottom = u;
            } else if dir.is_rtl() {
                b.left = u;
            } else {
                b.right = u;
            }
        }
        "inset-block-start" => {
            let u = to_unit(val);
            let b = Arc::make_mut(&mut style.box_style);
            if wm == WritingMode::VerticalRl {
                b.right = u;
            } else if wm == WritingMode::VerticalLr {
                b.left = u;
            } else {
                b.top = u;
            }
        }
        "inset-block-end" => {
            let u = to_unit(val);
            let b = Arc::make_mut(&mut style.box_style);
            if wm == WritingMode::VerticalRl {
                b.left = u;
            } else if wm == WritingMode::VerticalLr {
                b.right = u;
            } else {
                b.bottom = u;
            }
        }

        // Bordas
        "border-top-width" => {
            if let Some(u) = to_unit(val) {
                Arc::make_mut(&mut style.surrounding).border_width.top = u;
            }
        }
        "border-right-width" => {
            if let Some(u) = to_unit(val) {
                Arc::make_mut(&mut style.surrounding).border_width.right = u;
            }
        }
        "border-bottom-width" => {
            if let Some(u) = to_unit(val) {
                Arc::make_mut(&mut style.surrounding).border_width.bottom = u;
            }
        }
        "border-left-width" => {
            if let Some(u) = to_unit(val) {
                Arc::make_mut(&mut style.surrounding).border_width.left = u;
            }
        }
        "border-top-style" => {
            if let Some(s) = BorderStyle::parse(val) {
                Arc::make_mut(&mut style.surrounding).border_style.top = s;
            }
        }
        "border-right-style" => {
            if let Some(s) = BorderStyle::parse(val) {
                Arc::make_mut(&mut style.surrounding).border_style.right = s;
            }
        }
        "border-bottom-style" => {
            if let Some(s) = BorderStyle::parse(val) {
                Arc::make_mut(&mut style.surrounding).border_style.bottom = s;
            }
        }
        "border-left-style" => {
            if let Some(s) = BorderStyle::parse(val) {
                Arc::make_mut(&mut style.surrounding).border_style.left = s;
            }
        }
        "border-top-color" => {
            if let Some(c) = CssColor::parse(val) {
                Arc::make_mut(&mut style.surrounding).border_color.top = c.resolve(style.color());
            }
        }
        "border-right-color" => {
            if let Some(c) = CssColor::parse(val) {
                Arc::make_mut(&mut style.surrounding).border_color.right =
                    c.resolve(style.color());
            }
        }
        "border-bottom-color" => {
            if let Some(c) = CssColor::parse(val) {
                Arc::make_mut(&mut style.surrounding).border_color.bottom =
                    c.resolve(style.color());
            }
        }
        "border-left-color" => {
            if let Some(c) = CssColor::parse(val) {
                Arc::make_mut(&mut style.surrounding).border_color.left = c.resolve(style.color());
            }
        }

        // Cores
        "color" => {
            if let Some(c) = CssColor::parse(val) {
                Arc::make_mut(&mut style.color).color = c.resolve(style.color());
            }
        }
        "background-color" => {
            if let Some(c) = CssColor::parse(val) {
                Arc::make_mut(&mut style.color).background_color = c.resolve(style.color());
            }
        }

        // Tipografia
        "font-weight" => {
            if let Some(w) = FontWeight::parse(val) {
                Arc::make_mut(&mut style.font).weight = w;
            }
        }
        "font-style" => {
            if let Some(s) = FontStyle::parse(val) {
                Arc::make_mut(&mut style.font).style = s;
            }
        }
        "font-family" => Arc::make_mut(&mut style.font).family = SmolStr::new(val),
        "line-height" => {
            if let Some(u) = to_unit(val) {
                Arc::make_mut(&mut style.font).line_height = Some(u);
            }
        }
        "text-align" => {
            if let Some(a) = TextAlign::parse(val) {
                Arc::make_mut(&mut style.text).text_align = a;
            }
        }
        "text-decoration" => {
            if let Some(d) = TextDecoration::parse(val) {
                Arc::make_mut(&mut style.text).text_decoration = d;
            }
        }
        "white-space" => {
            if let Some(w) = WhiteSpace::parse(val) {
                Arc::make_mut(&mut style.text).white_space = w;
            }
        }
        "word-break" => {
            if let Some(w) = WordBreak::parse(val) {
                Arc::make_mut(&mut style.text).word_break = w;
            }
        }
        "overflow-wrap" => {
            if let Some(w) = OverflowWrap::parse(val) {
                Arc::make_mut(&mut style.text).overflow_wrap = w;
            }
        }

        // Posicionamento & Visibilidade
        "top" => Arc::make_mut(&mut style.box_style).top = to_unit(val),
        "right" => Arc::make_mut(&mut style.box_style).right = to_unit(val),
        "bottom" => Arc::make_mut(&mut style.box_style).bottom = to_unit(val),
        "left" => Arc::make_mut(&mut style.box_style).left = to_unit(val),
        "visibility" => {
            if let Some(v) = Visibility::parse(val) {
                Arc::make_mut(&mut style.text).visibility = v;
            }
        }
        "opacity" => {
            if let Ok(o) = val.parse::<f32>() {
                Arc::make_mut(&mut style.color).opacity = o.clamp(0.0, 1.0);
            }
        }
        "z-index" => {
            if let Some(z) = ZIndex::parse(val) {
                Arc::make_mut(&mut style.box_style).z_index = z;
            }
        }
        "overflow-x" => {
            if let Some(o) = Overflow::parse(val) {
                Arc::make_mut(&mut style.box_style).overflow_x = o;
            }
        }
        "overflow-y" => {
            if let Some(o) = Overflow::parse(val) {
                Arc::make_mut(&mut style.box_style).overflow_y = o;
            }
        }

        // Flexbox
        "flex-direction" => {
            if let Some(d) = FlexDirection::parse(val) {
                Arc::make_mut(&mut style.flex).flex_direction = d;
            }
        }
        "flex-wrap" => {
            if let Some(w) = FlexWrap::parse(val) {
                Arc::make_mut(&mut style.flex).flex_wrap = w;
            }
        }
        "justify-content" => {
            if let Some(j) = JustifyContent::parse(val) {
                Arc::make_mut(&mut style.flex).justify_content = j;
            }
        }
        "align-items" => {
            if let Some(a) = AlignItems::parse(val) {
                Arc::make_mut(&mut style.flex).align_items = a;
            }
        }
        "align-self" => {
            if let Some(a) = AlignSelf::parse(val) {
                Arc::make_mut(&mut style.flex).align_self = a;
            }
        }
        "align-content" => {
            if let Some(a) = AlignContent::parse(val) {
                Arc::make_mut(&mut style.flex).align_content = a;
            }
        }
        "flex-grow" => {
            if let Ok(g) = val.parse::<f32>() {
                Arc::make_mut(&mut style.flex).flex_grow = g.max(0.0);
            }
        }
        "flex-shrink" => {
            if let Ok(s) = val.parse::<f32>() {
                Arc::make_mut(&mut style.flex).flex_shrink = s.max(0.0);
            }
        }
        "flex-basis" => Arc::make_mut(&mut style.flex).flex_basis = to_unit(val),
        "order" => {
            if let Ok(o) = val.parse::<i32>() {
                Arc::make_mut(&mut style.flex).order = o;
            }
        }
        "row-gap" => Arc::make_mut(&mut style.flex).row_gap = to_unit(val),
        "column-gap" => Arc::make_mut(&mut style.flex).column_gap = to_unit(val),

        // Grid
        "grid-auto-flow" => {
            if let Some(g) = GridAutoFlow::parse(val) {
                Arc::make_mut(&mut style.grid).grid_auto_flow = g;
            }
        }

        // Pseudo-Elementos (Propriedade 'content')
        "content" => {
            let unquoted = val.trim().trim_matches('"').trim_matches('\'');
            style.content = Some(SmolStr::new(unquoted));
        }

        _ => {}
    }
}

fn apply_inherit_keyword(style: &mut ComputedStyle, parent: &ComputedStyle, prop: &str) {
    match prop {
        "color" => Arc::make_mut(&mut style.color).color = parent.color(),
        "font-size" => Arc::make_mut(&mut style.font).size = parent.font_size(),
        "font-weight" => Arc::make_mut(&mut style.font).weight = parent.font_weight(),
        "font-style" => Arc::make_mut(&mut style.font).style = parent.font_style(),
        "font-family" => {
            Arc::make_mut(&mut style.font).family = SmolStr::new(parent.font_family())
        }
        "line-height" => Arc::make_mut(&mut style.font).line_height = parent.line_height(),
        "text-align" => Arc::make_mut(&mut style.text).text_align = parent.text_align(),
        "visibility" => Arc::make_mut(&mut style.text).visibility = parent.visibility(),
        "white-space" => Arc::make_mut(&mut style.text).white_space = parent.white_space(),
        "word-break" => Arc::make_mut(&mut style.text).word_break = parent.word_break(),
        "overflow-wrap" => Arc::make_mut(&mut style.text).overflow_wrap = parent.overflow_wrap(),
        "direction" => Arc::make_mut(&mut style.text).direction = parent.direction(),
        "writing-mode" => Arc::make_mut(&mut style.text).writing_mode = parent.writing_mode(),
        "display" => Arc::make_mut(&mut style.box_style).display = parent.display(),
        "position" => Arc::make_mut(&mut style.box_style).position = parent.position(),
        "box-sizing" => Arc::make_mut(&mut style.box_style).box_sizing = parent.box_sizing(),
        "width" => Arc::make_mut(&mut style.box_style).width = parent.width(),
        "height" => Arc::make_mut(&mut style.box_style).height = parent.height(),
        "opacity" => Arc::make_mut(&mut style.color).opacity = parent.opacity(),
        _ => {}
    }
}

fn apply_initial_keyword(style: &mut ComputedStyle, prop: &str) {
    let initial = ComputedStyle::initial();
    match prop {
        "color" => Arc::make_mut(&mut style.color).color = initial.color(),
        "font-size" => Arc::make_mut(&mut style.font).size = initial.font_size(),
        "font-weight" => Arc::make_mut(&mut style.font).weight = initial.font_weight(),
        "font-style" => Arc::make_mut(&mut style.font).style = initial.font_style(),
        "font-family" => {
            Arc::make_mut(&mut style.font).family = SmolStr::new(initial.font_family())
        }
        "line-height" => Arc::make_mut(&mut style.font).line_height = initial.line_height(),
        "text-align" => Arc::make_mut(&mut style.text).text_align = initial.text_align(),
        "visibility" => Arc::make_mut(&mut style.text).visibility = initial.visibility(),
        "white-space" => Arc::make_mut(&mut style.text).white_space = initial.white_space(),
        "word-break" => Arc::make_mut(&mut style.text).word_break = initial.word_break(),
        "overflow-wrap" => Arc::make_mut(&mut style.text).overflow_wrap = initial.overflow_wrap(),
        "direction" => Arc::make_mut(&mut style.text).direction = initial.direction(),
        "writing-mode" => Arc::make_mut(&mut style.text).writing_mode = initial.writing_mode(),
        "display" => Arc::make_mut(&mut style.box_style).display = initial.display(),
        "position" => Arc::make_mut(&mut style.box_style).position = initial.position(),
        "box-sizing" => Arc::make_mut(&mut style.box_style).box_sizing = initial.box_sizing(),
        "width" => Arc::make_mut(&mut style.box_style).width = initial.width(),
        "height" => Arc::make_mut(&mut style.box_style).height = initial.height(),
        "opacity" => Arc::make_mut(&mut style.color).opacity = initial.opacity(),
        _ => {}
    }
}
