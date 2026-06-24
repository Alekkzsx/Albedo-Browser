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



pub(crate) fn apply_inline_styles(
    style: &mut ComputedStyle,
    inline_str: &str,
    parent_font_size: f32,
    root_font_size: f32,
) {
    let inline_decls = parse_inline_declarations(inline_str);
    for decl in &inline_decls {
        apply_single_declaration(style, decl, parent_font_size, root_font_size, true);
    }
    let current_font_size = style.font_size;
    let mut property_importance = std::collections::HashMap::new();
    for decl in &inline_decls {
        let prop_name = decl.name.clone();
        let is_important = decl.important;
        let current_weight = if is_important { 3 } else { 2 };
        let prev_weight = *property_importance.get(&prop_name).unwrap_or(&0);
        if current_weight >= prev_weight {
            apply_single_declaration(style, decl, current_font_size, root_font_size, false);
            property_importance.insert(prop_name, current_weight);
        }
    }
}
