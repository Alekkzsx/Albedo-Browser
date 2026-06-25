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





impl Stylesheet {
        _current_time: f64,
        vw: f32,
        vh: f32,
        color_scheme: &str,
    ) -> ComputedStyle {
        let node = dom.get_node(node_id).expect("Node missing");
        let _element_data = match &node.node_type {
            AceNodeType::Element(e) => e,
            _ => return ComputedStyle::default(),
        };

        let cache_key = self.compute_style_hash(
            dom, node_id, hovered_element, focused_element,
            active_element, vw, vh, color_scheme,
        );

        if let Ok(cache) = self.style_sharing_cache.read() {
            if let Some(cached_style) = cache.get(&cache_key) {
                return cached_style.clone();
            }
        }

        let mut style = inherit_from_parent(parent_style);

        if let Some(node) = dom.get_node(node_id) {
            if let AceNodeType::Element(el) = &node.node_type {
                let ace_element = AceElement {
                    dom, index: node_id, hovered_element, focused_element, active_element,
                };
                let mut matched_rules = Vec::new();
                self.match_rules_for_element(&ace_element, &mut matched_rules, el, vw, vh, color_scheme);

                let parent_font_size = parent_style.map(|s| s.font_size).unwrap_or(16.0);
                let root_font_size = root_style.map(|s| s.font_size).unwrap_or(16.0);

                apply_matched_rules(&mut style, &matched_rules, parent_font_size, root_font_size);

                if let Some(inline_str) = el.attributes.get("style") {
                    apply_inline_styles(&mut style, inline_str, parent_font_size, root_font_size);
                }

                if focused_element == Some(node_id) && style.outline.is_none() {
                    style.outline = Some(crate::ace::engine::style::css_values::Outline {
                        width: 2.0,
                        color: crate::ace::engine::style::css_values::CssColor::Named("#0066ff".to_string()),
                        style: "solid".to_string(),
                        offset: 2.0,
                    });
                }
            }
        }

        if let Ok(mut cache) = self.style_sharing_cache.write() {
            cache.insert(cache_key, style.clone());
        }

        style
    }

pub(crate) fn compute_style_hash(
        &self,
        dom: &AceDOM,
        node_id: usize,
        hovered_element: Option<usize>,
        focused_element: Option<usize>,
        active_element: Option<usize>,
        vw: f32,
        vh: f32,
        color_scheme: &str,
    ) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        // Hash environment (media queries can change result)
        (vw.to_bits(), vh.to_bits(), color_scheme).hash(&mut hasher);

        let mut current_idx = Some(node_id);
        let mut depth = 0;

        while let Some(idx) = current_idx {
            if let Some(node) = dom.get_node(idx) {
                if let AceNodeType::Element(el) = &node.node_type {
                    // Hash Element Identity
                    el.tag.hash(&mut hasher);

                    if let Some(id) = el.attributes.get("id") {
                        id.hash(&mut hasher);
                    }
                    if let Some(classes) = el.attributes.get("class") {
                        classes.hash(&mut hasher);
                    }

                    // Style attributes have high specificity
                    if let Some(inline_style) = el.attributes.get("style") {
                        inline_style.hash(&mut hasher);
                    }

                    for (k, v) in &el.attributes {
                        if k != "id" && k != "class" && k != "style" {
                            k.hash(&mut hasher);
                            v.hash(&mut hasher);
                        }
                    }

                    // State hashes (crucial for pseudo-classes like :hover)
                    if hovered_element == Some(idx) {
                        1u8.hash(&mut hasher);
                    }
                    if focused_element == Some(idx) {
                        2u8.hash(&mut hasher);
                    }
                    if active_element == Some(idx) {
                        3u8.hash(&mut hasher);
                    }

                    // Hash Sibling Context (for + and ~ selectors)
                    if depth == 0 {
                        let mut prev_idx = node.prev_sibling;
                        while let Some(p_idx) = prev_idx {
                            if let Some(p_node) = dom.get_node(p_idx) {
                                if let AceNodeType::Element(p_el) = &p_node.node_type {
                                    "sibling".hash(&mut hasher);
                                    p_el.tag.hash(&mut hasher);
                                    if let Some(pc) = p_el.attributes.get("class") {
                                        pc.hash(&mut hasher);
                                    }
                                }
                                prev_idx = p_node.prev_sibling;
                            } else {
                                break;
                            }
                        }
                    }
                }

                // Go to parent
                current_idx = node.parent;
                depth += 1;
            } else {
                break;
            }
        }

        hasher.finish()
    }
}
