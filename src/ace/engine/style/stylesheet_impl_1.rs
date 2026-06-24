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



impl Stylesheet {
pub(crate) fn match_rules_for_element<'a>(
        &'a self,
        ace_element: &AceElement<'a>,
        matched_rules: &mut Vec<MatchedRule<'a>>,
        el: &crate::ace::engine::dom::AceElementData,
        vw: f32,
        vh: f32,
        color_scheme: &str,
    ) {
        self.user_agent_rule_map.match_element(ace_element, matched_rules, CascadeOrigin::UserAgent, 0);
        self.author_rule_map.match_element(ace_element, matched_rules, CascadeOrigin::Author, 1000);

        if el.tag == "body" || el.tag == "div" {
            tracing::debug!(tag = %el.tag, class = ?el.attributes.get("class"), matched_count = matched_rules.len(), "Element matched rules");
            for mr in matched_rules.iter() {
                tracing::debug!(selectors = ?mr.rule.selectors, "Matched selector");
            }
        }

        for media_rule in &self.media_rules {
            if matches_media_query(&media_rule.media_query, vw, vh, color_scheme) {
                for (order, rule) in media_rule.rules.iter().enumerate() {
                    for selector in rule.selectors.slice() {
                        let mut caches = selectors::matching::SelectorCaches::default();
                        let mut context = MatchingContext::new(
                            MatchingMode::Normal, None, &mut caches,
                            selectors::matching::QuirksMode::NoQuirks,
                            selectors::matching::NeedsSelectorFlags::No,
                            selectors::matching::MatchingForInvalidation::No,
                        );
                        if selectors::matching::matches_selector(selector, 0, None, ace_element, &mut context) {
                            matched_rules.push(MatchedRule {
                                priority: CascadePriority { origin: CascadeOrigin::AuthorMedia, important: false, specificity: selector.specificity(), order: order + 2000 },
                                rule,
                            });
                        }
                    }
                }
            }
        }

        for (i, rule_block) in self.supports_rules.iter().enumerate() {
            if matches_supports(&rule_block.condition) {
                for (order, rule) in rule_block.rules.iter().enumerate() {
                    for selector in rule.selectors.slice() {
                        let mut caches = selectors::matching::SelectorCaches::default();
                        let mut context = MatchingContext::new(
                            MatchingMode::Normal, None, &mut caches,
                            selectors::matching::QuirksMode::NoQuirks,
                            selectors::matching::NeedsSelectorFlags::No,
                            selectors::matching::MatchingForInvalidation::No,
                        );
                        if selectors::matching::matches_selector(selector, 0, None, ace_element, &mut context) {
                            matched_rules.push(MatchedRule {
                                priority: CascadePriority { origin: CascadeOrigin::AuthorMedia, important: false, specificity: selector.specificity(), order: 3000 + i * 100 + order },
                                rule,
                            });
                        }
                    }
                }
            }
        }

        for (i, rule_block) in self.container_rules.iter().enumerate() {
            if matches_container(&rule_block.condition, vw) {
                for (order, rule) in rule_block.rules.iter().enumerate() {
                    for selector in rule.selectors.slice() {
                        let mut caches = selectors::matching::SelectorCaches::default();
                        let mut context = MatchingContext::new(
                            MatchingMode::Normal, None, &mut caches,
                            selectors::matching::QuirksMode::NoQuirks,
                            selectors::matching::NeedsSelectorFlags::No,
                            selectors::matching::MatchingForInvalidation::No,
                        );
                        if selectors::matching::matches_selector(selector, 0, None, ace_element, &mut context) {
                            matched_rules.push(MatchedRule {
                                priority: CascadePriority { origin: CascadeOrigin::AuthorMedia, important: false, specificity: selector.specificity(), order: 4000 + i * 100 + order },
                                rule,
                            });
                        }
                    }
                }
            }
        }

        matched_rules.sort_by(|a, b| a.priority.cmp(&b.priority));
    }

    // Calculate style with inheritance
    pub fn calculate_style(
        &self,
        dom: &AceDOM,
        node_id: usize,
        parent_style: Option<&ComputedStyle>,
        root_style: Option<&ComputedStyle>,
        hovered_element: Option<usize>,
        focused_element: Option<usize>,
        active_element: Option<usize>,
        _animation_manager: Option<&self::animation::AnimationManager>,
}
