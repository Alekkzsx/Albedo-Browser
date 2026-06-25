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




pub struct Stylesheet {
    pub user_agent_rules: Vec<AceRule>,
    pub rules: Vec<AceRule>,
    pub media_rules: Vec<AceMediaRule>,
    pub supports_rules: Vec<AceSupportsRule>,
    pub container_rules: Vec<AceContainerRule>,
    pub font_faces: Vec<std::collections::HashMap<String, String>>,
    pub keyframes: HashMap<String, Vec<self::css_values::CssKeyframe>>,
    pub user_agent_rule_map: RuleMap,
    pub author_rule_map: RuleMap,
    pub style_sharing_cache:
        std::sync::Arc<std::sync::RwLock<std::collections::HashMap<u64, ComputedStyle>>>,
}

impl Stylesheet {
    /// TODO: add docs
    pub fn new() -> Self {
        Self {
            user_agent_rules: Vec::new(),
            rules: Vec::new(),
            media_rules: Vec::new(),
            supports_rules: Vec::new(),
            container_rules: Vec::new(),
            font_faces: Vec::new(),
            keyframes: HashMap::new(),
            user_agent_rule_map: RuleMap::default(),
            author_rule_map: RuleMap::default(),
            style_sharing_cache: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    /// TODO: add docs
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.style_sharing_cache.write() {
            cache.clear();
        }
    }

    /// TODO: add docs
    pub fn build_rule_maps(&mut self) {
        let mut ua_map = RuleMap::default();
        for rule in &self.user_agent_rules {
            ua_map.add_rule(rule.clone());
        }
        self.user_agent_rule_map = ua_map;

        let mut author_map = RuleMap::default();
        for rule in &self.rules {
            author_map.add_rule(rule.clone());
        }
        self.author_rule_map = author_map;
    }
}
