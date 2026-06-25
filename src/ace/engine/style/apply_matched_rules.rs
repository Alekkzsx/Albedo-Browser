use super::*;
use crate::ace::engine::dom::{AceDOM, AceNodeType};
use cssparser::{CowRcStr, ParseError, Parser, ParserInput, SourceLocation, ToCss};
use precomputed_hash::PrecomputedHash;
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching::{ElementSelectorFlags, MatchingContext, MatchingMode};
use selectors::OpaqueElement;
use std::collections::HashMap;
use crate::ace::engine::style::css_values::{
    BackgroundImage, BoxShadow, ComputedStyle, CssAlignContent, CssAlignItems, CssBlendMode,
    CssBoxSizing, CssClear, CssColor, CssContent, CssCursor, CssDisplay, CssFlexDirection,
    CssFlexWrap, CssFloat, CssFontWeight, CssJustifyContent, CssLength, CssObjectFit,
    CssObjectPosition, CssOverflow, CssPointerEvents, CssPosition, CssTextAlign, CssTextOverflow,
    CssTextTransform, CssVisibility, CssWhiteSpace, Gradient, GradientStop, TextShadow,
};
use crate::ace::engine::style::css_values::CssFilter;
use crate::ace::engine::style::css_values::TransformFunction;





pub(crate) fn apply_matched_rules(
    style: &mut ComputedStyle,
    matched_rules: &[MatchedRule],
    parent_font_size: f32,
    root_font_size: f32,
) {
    for match_rule in matched_rules {
        for decl in &match_rule.rule.declarations {
            apply_single_declaration(style, decl, parent_font_size, root_font_size, true);
        }
    }

    let mut property_importance = std::collections::HashMap::new();
    for match_rule in matched_rules {
        for decl in &match_rule.rule.declarations {
            let prop_name = decl.name.clone();
            let is_important = decl.important;
            let current_weight = match (match_rule.priority.origin, is_important) {
                (CascadeOrigin::UserAgent, false) => 1,
                (CascadeOrigin::Author, false) | (CascadeOrigin::AuthorMedia, false) => 2,
                (CascadeOrigin::Author, true) | (CascadeOrigin::AuthorMedia, true) => 3,
                (CascadeOrigin::UserAgent, true) => 4,
            };
            let prev_weight = *property_importance.get(&prop_name).unwrap_or(&0);
            if current_weight >= prev_weight {
                apply_single_declaration(style, decl, style.font_size, root_font_size, false);
                property_importance.insert(prop_name, current_weight);
            }
        }
    }
}
