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





pub(crate) fn parse_simple(source: &str) -> Stylesheet {
    let mut stylesheet = Stylesheet {
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
    };
    let mut input = cssparser::ParserInput::new(source);
    let mut parser = cssparser::Parser::new(&mut input);
    let mut rule_parser_instance = AceStyleRuleParser;
    let mut rule_parser = cssparser::StyleSheetParser::new(&mut parser, &mut rule_parser_instance);
    while let Some(result) = rule_parser.next() {
        match result {
            Ok(mut rule) => {
                tracing::debug!(decl_count = rule.declarations.len(), "Block parsed OK");
                rule.order = stylesheet.rules.len();
                stylesheet.rules.push(rule);
            }
            Err((e, _)) => {
                tracing::debug!(?e, "Rule failed");
            }
        }
    }

    stylesheet.build_rule_maps();
    stylesheet
}
