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





#[derive(Default, Clone)]
pub struct RuleMap {
    pub id_rules: HashMap<String, Vec<AceRule>>,
    pub class_rules: HashMap<String, Vec<AceRule>>,
    pub tag_rules: HashMap<String, Vec<AceRule>>,
    pub universal_rules: Vec<AceRule>,
}

impl RuleMap {
    /// TODO: add docs
    pub fn add_rule(&mut self, rule: AceRule) {
        for selector in rule.selectors.slice().iter() {
            let mut indexed = false;
            // Iterate in matching order (right-to-left) to find the most specific clue
            for component in selector.iter_raw_match_order() {
                match component {
                    selectors::parser::Component::ID(id) => {
                        self.id_rules
                            .entry(id.0.clone())
                            .or_default()
                            .push(rule.clone());
                        indexed = true;
                        break;
                    }
                    selectors::parser::Component::Class(class) => {
                        self.class_rules
                            .entry(class.0.clone())
                            .or_default()
                            .push(rule.clone());
                        indexed = true;
                        break;
                    }
                    selectors::parser::Component::LocalName(name) => {
                        self.tag_rules
                            .entry(name.name.0.clone())
                            .or_default()
                            .push(rule.clone());
                        indexed = true;
                        break;
                    }
                    _ => {}
                }
            }
            if !indexed {
                self.universal_rules.push(rule.clone());
            }
        }
    }

    /// TODO: add docs
    pub fn match_element<'a>(
        &'a self,
        element: &AceElement,
        matched_rules: &mut Vec<MatchedRule<'a>>,
        origin: CascadeOrigin,
        base_order: usize,
    ) {
        let mut try_match = |rule: &'a AceRule| {
            for selector in rule.selectors.slice() {
                let mut caches = selectors::matching::SelectorCaches::default();
                let mut context = MatchingContext::new(
                    MatchingMode::Normal,
                    None,
                    &mut caches,
                    selectors::matching::QuirksMode::NoQuirks,
                    selectors::matching::NeedsSelectorFlags::No,
                    selectors::matching::MatchingForInvalidation::No,
                );
                if selectors::matching::matches_selector(selector, 0, None, element, &mut context) {
                    matched_rules.push(MatchedRule {
                        priority: CascadePriority {
                            origin: origin.clone(),
                            important: false,
                            specificity: selector.specificity(),
                            order: base_order + rule.order,
                        },
                        rule,
                    });
                }
            }
        };

        // 1. Check Universal rules
        for rule in &self.universal_rules {
            try_match(rule);
        }

        if let AceNodeType::Element(el) = &element.dom.get_node(element.index).expect("Node missing").node_type {
            // 2. Check Tag rules
            if let Some(rules) = self.tag_rules.get(el.tag_name()) {
                for rule in rules {
                    try_match(rule);
                }
            }
            // 3. Check ID rules
            if let Some(id) = el.attributes.get("id") {
                if let Some(rules) = self.id_rules.get(id) {
                    for rule in rules {
                        try_match(rule);
                    }
                }
            }
            // 4. Check Class rules
            if let Some(class_attr) = el.attributes.get("class") {
                for class in class_attr.split_whitespace() {
                    if let Some(rules) = self.class_rules.get(class) {
                        for rule in rules {
                            try_match(rule);
                        }
                    }
                }
            }
        }
    }
}
