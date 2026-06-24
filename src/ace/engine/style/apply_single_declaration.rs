use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use cssparser::{CowRcStr, ParseError, Parser, ParserInput, SourceLocation, ToCss};
use precomputed_hash::PrecomputedHash;
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching::{ElementSelectorFlags, MatchingContext, MatchingMode};
use selectors::OpaqueElement;
use std::collections::HashMap;
pub mod css_values;
use self::css_values::{
    BackgroundImage, BoxShadow, ComputedStyle, CssAlignContent, CssAlignItems, CssBlendMode,
    CssBoxSizing, CssClear, CssColor, CssContent, CssCursor, CssDisplay, CssFlexDirection,
    CssFlexWrap, CssFloat, CssFontWeight, CssJustifyContent, CssLength, CssObjectFit,
    CssObjectPosition, CssOverflow, CssPointerEvents, CssPosition, CssTextAlign, CssTextOverflow,
    CssTextTransform, CssVisibility, CssWhiteSpace, Gradient, GradientStop, TextShadow,
};
use crate::ace::engine::style::css_values::CssFilter;
use crate::ace::engine::style::css_values::TransformFunction;

pub mod animation;



/// TODO: add docs
pub fn apply_single_declaration(
    style: &mut ComputedStyle,
    decl: &Declaration,
    current_font_size: f32,
    root_font_size: f32,
    phase_1_only: bool,
) {
    let name = decl.name.as_str();
    let val_raw = decl.value.trim();

    if phase_1_only {
        if name == "font-size" {
            let value = if val_raw.contains("var(") {
                resolve_css_variables(val_raw, style)
            } else {
                val_raw.to_string()
            };

            let parsed = parse_length(&value);
            style.font_size = match parsed {
                CssLength::Px(v) => v,
                CssLength::Em(v) => v * current_font_size,
                CssLength::Rem(v) => v * root_font_size,
                CssLength::Percent(v) => v / 100.0 * current_font_size,
                _ => style.font_size,
            };
        }
        return;
    }

    if name == "font-size" {
        return;
    }

    if name.starts_with("--") {
        style
            .custom_properties
            .insert(name.to_string(), val_raw.to_string());
        return;
    }

    let val_string = if val_raw.contains("var(") {
        resolve_css_variables(val_raw, style)
    } else {
        val_raw.to_string()
    };
    let value = val_string.as_str();

    let resolve_rel =
        |l: CssLength| -> CssLength { resolve_rel_static(l, current_font_size, root_font_size) };

    match name {
        n @ ("text-transform" | "text-overflow" | "white-space" | "whiteSpace"
            | "background-color" | "background" | "color"
            | "border-radius" | "box-shadow" | "text-shadow" | "background-image"
            | "content" | "aspect-ratio" | "box-sizing" | "boxSizing"
            | "visibility" | "cursor" | "pointer-events" | "pointerEvents"
            | "transform") => {
            apply_text_and_misc(style, n, value);
        }

        n @ ("display" | "position" | "overflow" | "float" | "clear"
            | "z-index" | "zIndex"
            | "width" | "height" | "top" | "right" | "bottom" | "left") => {
            apply_basic_layout(style, n, value, &resolve_rel);
        }

        n @ ("margin-top" | "margin-right" | "margin-bottom" | "margin-left" | "margin"
            | "padding-top" | "padding-right" | "padding-bottom" | "padding-left" | "padding") => {
            apply_box_sides(style, n, value, &resolve_rel);
        }

        n @ ("opacity" | "object-fit" | "objectFit" | "object-position" | "objectPosition"
            | "filter" | "backdrop-filter" | "backdropFilter"
            | "mix-blend-mode" | "mixBlendMode"
            | "transition" | "animation" | "clip-path" | "clipPath") => {
            apply_visual_effect(style, n, value);
        }

        "order" => {
            if let Ok(n) = value.parse::<i32>() {
                style.order = n;
            }
        }
        "align-self" | "alignSelf" => style.align_self = parse_align_items(value),
        "align-content" | "alignContent" => style.align_content = parse_align_content(value),

        n @ ("grid-template-columns" | "gridTemplateColumns"
            | "grid-template-rows" | "gridTemplateRows"
            | "grid-template-areas" | "gridTemplateAreas"
            | "grid-column-gap" | "column-gap"
            | "grid-row-gap" | "row-gap"
            | "gap"
            | "grid-column-start" | "gridColumnStart"
            | "grid-column-end" | "gridColumnEnd"
            | "grid-row-start" | "gridRowStart"
            | "grid-row-end" | "gridRowEnd"
            | "grid-column" | "gridColumn"
            | "grid-row" | "gridRow"
            | "grid-area" | "gridArea") => {
            apply_grid_property(style, n, value);
        }

        n @ ("flex-direction" | "flexDirection"
            | "justify-content" | "justifyContent"
            | "align-items" | "alignItems"
            | "flex-wrap" | "flexWrap"
            | "flex-grow" | "flexGrow"
            | "flex-shrink" | "flexShrink"
            | "flex-basis" | "flexBasis") => {
            apply_flexbox_property(style, n, value);
        }

        n @ ("border-top-width" | "borderTopWidth"
            | "border-right-width" | "borderRightWidth"
            | "border-bottom-width" | "borderBottomWidth"
            | "border-left-width" | "borderLeftWidth"
            | "border-top-color" | "borderTopColor"
            | "border-right-color" | "borderRightColor"
            | "border-bottom-color" | "borderBottomColor"
            | "border-left-color" | "borderLeftColor") => {
            apply_border_detail(style, n, value);
        }

        n @ ("outline-width" | "outlineWidth"
            | "outline-color" | "outlineColor"
            | "outline-style" | "outlineStyle"
            | "outline-offset" | "outlineOffset") => {
            apply_outline_property(style, n, value, style.font_size);
        }

        n @ ("font-family" | "fontFamily" | "font-weight" | "fontWeight"
            | "font-style" | "fontStyle" | "line-height" | "lineHeight"
            | "letter-spacing" | "letterSpacing" | "word-spacing" | "wordSpacing"
            | "text-align" | "textAlign") => {
            apply_typography(style, n, value);
        }

        "min-width" | "minWidth" => style.min_width = resolve_rel(parse_length(value)),
        "max-width" | "maxWidth" => style.max_width = resolve_rel(parse_length(value)),
        "min-height" | "minHeight" => style.min_height = resolve_rel(parse_length(value)),
        "max-height" | "maxHeight" => style.max_height = resolve_rel(parse_length(value)),

        _ => {}
    }
}
