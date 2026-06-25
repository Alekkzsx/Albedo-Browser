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

    /// TODO: add docs
    pub fn calculate_pseudo_style(
        &self,
        dom: &AceDOM,
        node_id: usize,
        pseudo: &AcePseudoElement,
        hovered_element: Option<usize>,
        focused_element: Option<usize>,
        active_element: Option<usize>,
        vw: f32,
        vh: f32,
        _color_scheme: &str,
    ) -> ComputedStyle {
        let mut style = ComputedStyle::default();

        // Default display for pseudo-elements is inline
        style.display = CssDisplay::Inline;

        if let Some(node) = dom.get_node(node_id) {
            if let AceNodeType::Element(_) = &node.node_type {
                let ace_element = AceElement {
                    dom,
                    index: node_id,
                    hovered_element,
                    focused_element,
                    active_element,
                };
                let mut matched_rules = Vec::new();

                // Injetar lógicas de cálculo com viewport aqui se necessário,
                // mas o principal é passar adiante para resolve_length.

                // ... rest of matching ... (simplified for this edit)

                // 1. Process User Agent Rules
                for (order, rule) in self.user_agent_rules.iter().enumerate() {
                    for selector in rule.selectors.slice() {
                        if selector.pseudo_element() == Some(pseudo) {
                            let mut caches = selectors::matching::SelectorCaches::default();
                            let mut context = MatchingContext::new(
                                MatchingMode::Normal,
                                None,
                                &mut caches,
                                selectors::matching::QuirksMode::NoQuirks,
                                selectors::matching::NeedsSelectorFlags::No,
                                selectors::matching::MatchingForInvalidation::No,
                            );
                            if selectors::matching::matches_selector(
                                selector,
                                0,
                                None,
                                &ace_element,
                                &mut context,
                            ) {
                                matched_rules.push(MatchedRule {
                                    priority: CascadePriority {
                                        origin: CascadeOrigin::UserAgent,
                                        important: false,
                                        specificity: selector.specificity(),
                                        order,
                                    },
                                    rule,
                                });
                            }
                        }
                    }
                }

                // 2. Process User Rules
                for (order, rule) in self.rules.iter().enumerate() {
                    for selector in rule.selectors.slice() {
                        if selector.pseudo_element() == Some(pseudo) {
                            let mut caches = selectors::matching::SelectorCaches::default();
                            let mut context = MatchingContext::new(
                                MatchingMode::Normal,
                                None,
                                &mut caches,
                                selectors::matching::QuirksMode::NoQuirks,
                                selectors::matching::NeedsSelectorFlags::No,
                                selectors::matching::MatchingForInvalidation::No,
                            );
                            if selectors::matching::matches_selector(
                                selector,
                                0,
                                None,
                                &ace_element,
                                &mut context,
                            ) {
                                matched_rules.push(MatchedRule {
                                    priority: CascadePriority {
                                        origin: CascadeOrigin::Author,
                                        important: false,
                                        specificity: selector.specificity(),
                                        order: order + 1000,
                                    },
                                    rule,
                                });
                            }
                        }
                    }
                }

                matched_rules.sort_by(|a, b| a.priority.cmp(&b.priority));

                for match_rule in &matched_rules {
                    for decl in &match_rule.rule.declarations {
                        let value = decl.value.trim();
                        match decl.name.as_str() {
                            "content" => style.content = parse_content(val),
                            "color" => style.color = parse_color(val),
                            "font-size" => {
                                style.font_size =
                                    resolve_length(&parse_length(val), 16.0, 16.0, vw, vh)
                            }
                            "display" => {
                                style.display = match val {
                                    "block" => CssDisplay::Block,
                                    "flex" => CssDisplay::Flex,
                                    "grid" => CssDisplay::Grid,
                                    "none" => CssDisplay::None,
                                    _ => CssDisplay::Inline,
                                };
                            }
                            "background-color" => style.background_color = parse_color(val),
                            "width" => style.width = parse_length(val),
                            "height" => style.height = parse_length(val),
                            "opacity" => {
                                if let Ok(n) = val.parse::<f32>() {
                                    style.opacity = n.clamp(0.0, 1.0);
                                }
                            }
                            "z-index" => {
                                if let Ok(n) = val.parse::<i32>() {
                                    style.z_index = n;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        style
    }
}
