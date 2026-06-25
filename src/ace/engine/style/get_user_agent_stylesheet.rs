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





/// TODO: add docs
pub fn get_user_agent_stylesheet() -> Stylesheet {
    let ua_css = "
        html, body, header, footer, main, section, article, nav, aside, 
        details, summary, figure, figcaption, h1, h2, h3, h4, 
        h5, h6, p, ul, ol, li, div, blockquote, address, form { display: block; }
        
        template, script, style, head, meta, link, title { display: none; }
        
        h1 { font-size: 2em; font-weight: bold; margin: 0.67em 0; }
        h2 { font-size: 1.5em; font-weight: bold; margin: 0.83em 0; }
        h3 { font-size: 1.17em; font-weight: bold; margin: 1em 0; }
        h4 { font-size: 1em; font-weight: bold; margin: 1.33em 0; }
        h5 { font-size: 0.83em; font-weight: bold; margin: 1.67em 0; }
        h6 { font-size: 0.67em; font-weight: bold; margin: 2.33em 0; }
        
        b, strong { font-weight: bold; }
        i, em { font-style: italic; }
        
        ul { list-style-type: disc; margin: 1em 0; padding-left: 40px; }
        ol { list-style-type: decimal; margin: 1em 0; padding-left: 40px; }
        
        button, input, select, textarea { 
            display: inline-block; 
            margin: 0; 
            font: inherit; 
            box-sizing: border-box;
        }
        button { background-color: #efefef; border: 1px solid #767676; padding: 1px 6px; }
        input[type=\"text\"], input[type=\"password\"], input[type=\"email\"], input[type=\"number\"], 
        input[type=\"date\"], input[type=\"time\"] { 
            background-color: white; 
            border: 1px solid #767676; 
            padding: 1px 2px; 
            min-height: 1.2em;
        }
        input[type=\"color\"] {
            width: 44px;
            height: 23px;
            padding: 1px 2px;
            background-color: white;
            border: 1px solid #767676;
        }
        input[type=\"range\"] {
            width: 129px;
            height: 21px;
            background: transparent;
        }
        textarea {
            background-color: white; 
            border: 1px solid #767676; 
            padding: 2px;
            min-height: 2em;
        }
        select {
            background-color: white; 
            border: 1px solid #767676; 
            padding: 1px 2px;
        }

        /* Table Default Styles */
        table { display: table; border-collapse: separate; border-spacing: 2px; border-color: gray; }
        thead { display: table-header-group; vertical-align: middle; border-color: inherit; }
        tbody { display: table-row-group; vertical-align: middle; border-color: inherit; }
        tfoot { display: table-footer-group; vertical-align: middle; border-color: inherit; }
        tr { display: table-row; vertical-align: inherit; border-color: inherit; }
        td, th { display: table-cell; vertical-align: inherit; }
        th { font-weight: bold; text-align: center; }

        /* Dialog Default Styles */
        dialog { display: none; position: fixed; background: white; color: black;
                 border: 1px solid rgba(0,0,0,0.3); padding: 1em; margin: auto;
                 max-width: 80vw; max-height: 80vh; overflow: auto; }
        dialog[open] { display: block; }

        /* Summary pointer cursor */
        summary { cursor: pointer; }
    ";

    let mut ss = parse_simple(ua_css);
    // Move rules to user_agent_rules
    let rules = std::mem::take(&mut ss.rules);
    ss.user_agent_rules = rules;
    ss.keyframes = HashMap::new();
    ss.supports_rules = Vec::new();
    ss.container_rules = Vec::new();
    ss.font_faces = Vec::new();

    // Build optimization maps
    ss.build_rule_maps();

    ss
}
