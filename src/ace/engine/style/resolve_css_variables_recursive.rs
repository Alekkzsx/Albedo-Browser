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





pub(crate) fn resolve_css_variables_recursive(val: &str, style: &ComputedStyle, depth: usize) -> String {
    if depth > 16 {
        // Recursion limit
        return val.to_string();
    }

    let mut resolved = val.to_string();
    let _changed = true;
    let mut iteration = 0;

    // Iterative replacement for current level, but recursive for nested vars if needed
    // Actually, simple iterative string replacement is still the easiest way to handle "var(--a, var(--b))" in one string
    // But we need to be careful.

    while let Some(start_idx) = resolved.find("var(") {
        iteration += 1;
        if iteration > 32 {
            break;
        } // Safety break for complex single-string nested vars

        let rest = &resolved[start_idx + 4..];
        let mut brace_count = 1;
        let mut end_idx = 0;
        for (i, c) in rest.chars().enumerate() {
            if c == '(' {
                brace_count += 1;
            } else if c == ')' {
                brace_count -= 1;
            }
            if brace_count == 0 {
                end_idx = i;
                break;
            }
        }

        if end_idx > 0 {
            let inner = rest[..end_idx].trim();
            // Handle fallback: var(--name, fallback)
            let mut parts = inner.splitn(2, ',');
            let var_name = parts.next().unwrap_or("").trim();
            let fallback = parts.next().map(|s| s.trim()).unwrap_or("");

            // Buscar valor da variável
            let replacement = style
                .custom_properties
                .get(var_name)
                .map(|v| resolve_css_variables_recursive(v.as_str(), style, depth + 1)) // Recurse here
                .unwrap_or_else(|| resolve_css_variables_recursive(fallback, style, depth + 1));

            let full_var = &resolved[start_idx..start_idx + 4 + end_idx + 1];
            resolved = resolved.replace(full_var, &replacement);
        } else {
            break;
        }
    }

    resolved
}
