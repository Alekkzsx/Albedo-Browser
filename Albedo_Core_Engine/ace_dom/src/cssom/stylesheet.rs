//! # CSSStyleSheet & Regras de Estilo (WHATWG CSSOM §6.2)
//!
//! Representação de folhas de estilo em cascata associadas a tags `<style>` e `<link rel="stylesheet">`.

use crate::cssom::declaration::CSSStyleDeclaration;
use crate::query::selector::ComplexSelector;
use ace_core::id::NodeId;
use smol_str::SmolStr;

/// Uma regra de estilo CSS (ex: `.card, div.box { color: blue; padding: 8px; }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSSStyleRule {
    pub selector_text: SmolStr,
    pub selectors: Vec<ComplexSelector>,
    pub style: CSSStyleDeclaration,
}

/// Tipos de regras suportadas em uma folha de estilo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CSSRule {
    Style(Box<CSSStyleRule>),
    Media {
        condition: SmolStr,
        rules: Vec<CSSRule>,
    },
    Keyframes {
        name: SmolStr,
        css_text: SmolStr,
    },
    Import {
        href: SmolStr,
    },
}

/// Uma folha de estilo CSS associada a um documento ou raiz de sombra.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CSSStyleSheet {
    pub rules: Vec<CSSRule>,
    pub disabled: bool,
    pub href: Option<SmolStr>,
    pub title: Option<SmolStr>,
    pub owner_node: Option<NodeId>,
}

impl CSSStyleSheet {
    /// Cria uma nova folha de estilo vazia.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            disabled: false,
            href: None,
            title: None,
            owner_node: None,
        }
    }

    /// Faz o parsing de um bloco de texto CSS em uma `CSSStyleSheet`.
    pub fn parse(css_text: &str) -> Self {
        let mut sheet = Self::new();
        let mut chars = css_text.char_indices().peekable();

        while let Some(&(start_idx, ch)) = chars.peek() {
            if ch.is_whitespace() {
                chars.next();
                continue;
            }

            // Encontra o bloco `{ ... }`
            let remaining = &css_text[start_idx..];
            if let Some(open_brace) = remaining.find('{') {
                if let Some(close_brace) = remaining[open_brace + 1..].find('}') {
                    let selector_str = remaining[..open_brace].trim();
                    let body_str = &remaining[open_brace + 1..open_brace + 1 + close_brace];

                    if selector_str.starts_with("@import") {
                        let href = selector_str.trim_start_matches("@import").trim().trim_matches('\'').trim_matches('"');
                        sheet.rules.push(CSSRule::Import { href: SmolStr::new(href) });
                    } else if !selector_str.is_empty() {
                        let mut parsed_selectors = Vec::new();
                        for sel_part in selector_str.split(',') {
                            let trimmed_sel = sel_part.trim();
                            if let Some(sel) = ComplexSelector::parse(trimmed_sel) {
                                parsed_selectors.push(sel);
                            }
                        }

                        let style_decl = CSSStyleDeclaration::parse(body_str);
                        sheet.rules.push(CSSRule::Style(Box::new(CSSStyleRule {
                            selector_text: SmolStr::new(selector_str),
                            selectors: parsed_selectors,
                            style: style_decl,
                        })));
                    }

                    let consumed_len = open_brace + 1 + close_brace + 1;
                    for _ in 0..consumed_len {
                        chars.next();
                    }
                    continue;
                }
            }
            chars.next();
        }

        sheet
    }

    /// Adiciona uma regra de estilo.
    pub fn insert_rule(&mut self, rule: CSSRule) {
        self.rules.push(rule);
    }
}
