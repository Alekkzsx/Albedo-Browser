use std::sync::{Arc, Mutex};
pub mod types;
pub mod parser;
pub mod selector_impl;
pub mod computer;

pub use types::{AlbedoStyle as Style, Color, Length, DisplayMode};
pub use computer::StyleComputer;
pub use types::AlbedoStyle;

use crate::engine::style::parser::RuleParser;
use selectors::parser::SelectorList;
use crate::engine::style::selector_impl::Helper;
use cssparser::QualifiedRuleParser;

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub selectors: SelectorList<Helper>,
    pub declarations: Vec<Declaration>,
}

impl Rule {
    pub fn max_specificity(&self) -> u32 {
        self.selectors.slice().iter()
            .map(|s| s.specificity())
            .max()
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

impl Stylesheet {
    pub fn parse(css: &str) -> Self {
        let mut stylesheet = Stylesheet::default();
        let mut input = cssparser::ParserInput::new(css);
        let mut parser = cssparser::Parser::new(&mut input);
        let mut rule_parser = RuleParser;
        
        while !parser.is_exhausted() {
            let start = parser.state();
            if let Ok(token) = parser.next() {
                match token {
                     cssparser::Token::AtKeyword(_name) => {
                         // Skip At-Rule: consume until semicolon or end of block
                         loop {
                             match parser.next() {
                                 Ok(cssparser::Token::Semicolon) => break,
                                 Ok(cssparser::Token::CurlyBracketBlock) => break, 
                                 Err(_) => break,
                                 _ => {}
                             }
                         }
                    }
                    cssparser::Token::CDO | cssparser::Token::CDC => {}
                    _ => {
                        parser.reset(&start);
                        // Try to parse as Qualified Rule (Style Rule)
                        match rule_parser.parse_prelude(&mut parser) {
                            Ok(prelude) => {
                                match rule_parser.parse_block(prelude, &start, &mut parser) {
                                    Ok(rule) => {
                                        stylesheet.rules.push(rule);
                                    }
                                    Err(_) => {
                                        // Block parse failed, potentially bad declaration
                                        // Consume block to recover?
                                        // parse_block usually consumes block.
                                    }
                                }
                            }
                            Err(_) => {
                                // Prelude failed (e.g. invalid selector)
                                // Skip one token? Or try to recover to next rule?
                                // If we don't advance, infinite loop.
                                // If previous token was not AtKeyword, we reset to 'start'.
                                // So we are at 'start' again. parser.next() will return same token.
                                // We MUST advance.
                                let _ = parser.next();
                                // And maybe consume until ; or { ?
                                // If selector is garbage, usually we skip until { or ;
                                // Same skip logic as At-Rule mostly.
                            }
                        }
                    }
                }
            }
        }
        
        stylesheet
    }
}

pub fn resolve_style(node: &kuchiki::NodeRef, _element: &kuchiki::ElementData, stylesheet: &Stylesheet) -> AlbedoStyle {
    let mut computer = StyleComputer::new(stylesheet);
    computer.compute_style(node)
}
