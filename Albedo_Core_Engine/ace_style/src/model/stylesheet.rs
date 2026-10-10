//! # Modelo de Folhas de Estilo e Regras CSS (CSSOM)
//!
//! Representação tipada de `StyleSheet`, `StyleRule`, `@media`, `@layer`, `@import` e `@keyframes`.

use crate::cascade::origin::StyleSheetOrigin;
use crate::model::layer::{LayerBlockRule, LayerStatementRule};
use crate::model::media::MediaRule;
use crate::parser::declaration_parser::{parse_declarations, ParsedDeclaration};
use ace_core::intern::Atom;
use ace_dom::query::selector::ComplexSelector;
use smol_str::SmolStr;

/// Regra de estilo contendo seletores complexos e bloco de declarações.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleRule {
    /// Lista de seletores combinados por vírgula (`h1, .title, #main`)
    pub selectors: Vec<ComplexSelector>,
    /// Declarações desdobradas e canônicas
    pub declarations: Vec<ParsedDeclaration>,
    /// Camada à qual a regra pertence (`@layer`), se houver
    pub layer: Option<Atom>,
    /// Ordem relativa da regra dentro da folha de estilo
    pub source_order: u32,
}

/// Regra de importação `@import url(...) [layer(...)] [media]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRule {
    pub url: String,
    pub layer: Option<Atom>,
    pub media: Option<String>,
}

/// Regra de animação por quadros-chave `@keyframes <name> { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyframesRule {
    pub name: Atom,
    pub frames: Vec<(SmolStr, Vec<ParsedDeclaration>)>,
}

/// Enumeração de todas as regras suportadas no CSSOM do Albedo.
#[derive(Debug, Clone, PartialEq)]
pub enum CSSRule {
    Style(StyleRule),
    Media(MediaRule),
    LayerBlock(LayerBlockRule),
    LayerStatement(LayerStatementRule),
    Import(ImportRule),
    Keyframes(KeyframesRule),
}

/// Representação de uma folha de estilo CSS completa.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyleSheet {
    /// Origem normativa da folha (User-Agent, Usuário ou Autor)
    pub origin: StyleSheetOrigin,
    /// Regras contidas na folha
    pub rules: Vec<CSSRule>,
    /// URL de origem (para folhas externas)
    pub source_url: Option<String>,
}

impl StyleSheet {
    pub fn new(origin: StyleSheetOrigin) -> Self {
        Self {
            origin,
            rules: Vec::new(),
            source_url: None,
        }
    }

    /// Analisa uma string CSS completa e constrói a árvore de regras da folha de estilo.
    pub fn parse(css: &str, origin: StyleSheetOrigin) -> Self {
        let mut sheet = Self::new(origin);
        parse_stylesheet_into(css, &mut sheet, None, &mut 0);
        sheet
    }
}

/// Parser de alto nível que transforma o texto CSS em regras CSSOM.
fn parse_stylesheet_into(
    css: &str,
    sheet: &mut StyleSheet,
    active_layer: Option<&Atom>,
    order_counter: &mut u32,
) {
    let mut rest = css.trim();

    while !rest.is_empty() {
        // Ignora comentários no nível raiz
        if rest.starts_with("/*") {
            if let Some(end_idx) = rest.find("*/") {
                rest = rest[end_idx + 2..].trim();
                continue;
            } else {
                break;
            }
        }

        // Ignora <!-- e -->
        if rest.starts_with("<!--") {
            rest = rest[4..].trim();
            continue;
        }
        if rest.starts_with("-->") {
            rest = rest[3..].trim();
            continue;
        }

        // At-Rules (@...)
        if rest.starts_with('@') {
            if let Some((rule, remaining)) = parse_at_rule(rest, active_layer, order_counter) {
                sheet.rules.push(rule);
                rest = remaining.trim();
                continue;
            }
        }

        // Regra de Estilo Normal: seletor { declarações }
        if let Some(open_brace) = rest.find('{') {
            let selector_str = rest[..open_brace].trim();

            if let Some(close_brace) = find_matching_brace(&rest[open_brace..]) {
                let body = &rest[open_brace + 1..open_brace + close_brace].trim();
                rest = rest[open_brace + close_brace + 1..].trim();

                let selectors: Vec<ComplexSelector> = selector_str
                    .split(',')
                    .filter_map(|s| ComplexSelector::parse(s.trim()))
                    .collect();

                if !selectors.is_empty() {
                    let declarations = parse_declarations(body);
                    *order_counter += 1;
                    sheet.rules.push(CSSRule::Style(StyleRule {
                        selectors,
                        declarations,
                        layer: active_layer.cloned(),
                        source_order: *order_counter,
                    }));
                }
                continue;
            }
        }

        // Se houver sintaxe malformada sem correspondência, avança até o próximo ponto e vírgula ou fecha chave
        if let Some(idx) = rest.find(|c| c == ';' || c == '}') {
            rest = rest[idx + 1..].trim();
        } else {
            break;
        }
    }
}

fn parse_at_rule<'a>(
    input: &'a str,
    active_layer: Option<&Atom>,
    order_counter: &mut u32,
) -> Option<(CSSRule, &'a str)> {
    if input.starts_with("@media") {
        let rest = input["@media".len()..].trim();
        let open_brace = rest.find('{')?;
        let query_str = rest[..open_brace].trim();
        let close_brace = find_matching_brace(&rest[open_brace..])?;
        let body = &rest[open_brace + 1..open_brace + close_brace];
        let remaining = &rest[open_brace + close_brace + 1..];

        let mut sub_sheet = StyleSheet::new(StyleSheetOrigin::Author);
        parse_stylesheet_into(body, &mut sub_sheet, active_layer, order_counter);

        let media_rule = MediaRule {
            query: crate::model::media::MediaQuery::parse(query_str),
            rules: sub_sheet.rules,
        };
        return Some((CSSRule::Media(media_rule), remaining));
    }

    if input.starts_with("@layer") {
        let rest = input["@layer".len()..].trim();
        // Pode ser declaração @layer a, b; ou bloco @layer name { ... }
        if let Some(semi_idx) = rest.find(';') {
            let brace_idx = rest.find('{');
            if brace_idx.is_none() || semi_idx < brace_idx.unwrap() {
                let stmt_str = rest[..semi_idx].trim();
                let remaining = &rest[semi_idx + 1..];
                let names: Vec<Atom> = stmt_str
                    .split(',')
                    .map(|s| Atom::new(s.trim()))
                    .filter(|a| !a.as_str().is_empty())
                    .collect();
                return Some((CSSRule::LayerStatement(LayerStatementRule { names }), remaining));
            }
        }

        if let Some(open_brace) = rest.find('{') {
            let name_str = rest[..open_brace].trim();
            let close_brace = find_matching_brace(&rest[open_brace..])?;
            let body = &rest[open_brace + 1..open_brace + close_brace];
            let remaining = &rest[open_brace + close_brace + 1..];

            let layer_atom = if name_str.is_empty() {
                Atom::new("__anonymous_layer__")
            } else {
                Atom::new(name_str)
            };

            let mut sub_sheet = StyleSheet::new(StyleSheetOrigin::Author);
            parse_stylesheet_into(body, &mut sub_sheet, Some(&layer_atom), order_counter);

            return Some((
                CSSRule::LayerBlock(LayerBlockRule {
                    name: layer_atom,
                    rules: sub_sheet.rules,
                }),
                remaining,
            ));
        }
    }

    if input.starts_with("@import") {
        let rest = input["@import".len()..].trim();
        let semi_idx = rest.find(';')?;
        let stmt = rest[..semi_idx].trim();
        let remaining = &rest[semi_idx + 1..];

        let url = stmt
            .trim_start_matches("url(")
            .trim_end_matches(')')
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();

        return Some((
            CSSRule::Import(ImportRule {
                url,
                layer: None,
                media: None,
            }),
            remaining,
        ));
    }

    if input.starts_with("@keyframes") {
        let rest = input["@keyframes".len()..].trim();
        let open_brace = rest.find('{')?;
        let name_str = rest[..open_brace].trim();
        let close_brace = find_matching_brace(&rest[open_brace..])?;
        let body = &rest[open_brace + 1..open_brace + close_brace];
        let remaining = &rest[open_brace + close_brace + 1..];

        let mut frames = Vec::new();
        let mut frame_rest = body.trim();
        while !frame_rest.is_empty() {
            if let Some(f_open) = frame_rest.find('{') {
                let f_sel = frame_rest[..f_open].trim();
                if let Some(f_close) = find_matching_brace(&frame_rest[f_open..]) {
                    let f_body = &frame_rest[f_open + 1..f_open + f_close];
                    frame_rest = frame_rest[f_open + f_close + 1..].trim();
                    let decls = parse_declarations(f_body);
                    frames.push((SmolStr::new(f_sel), decls));
                    continue;
                }
            }
            break;
        }

        return Some((
            CSSRule::Keyframes(KeyframesRule {
                name: Atom::new(name_str),
                frames,
            }),
            remaining,
        ));
    }

    None
}

/// Encontra a posição correspondente de fechamento de chave `{ ... }`, considerando aninhamento.
fn find_matching_brace(s: &str) -> Option<usize> {
    if !s.starts_with('{') {
        return None;
    }
    let mut depth = 0;
    let mut in_quote: Option<char> = None;

    for (idx, ch) in s.char_indices() {
        match ch {
            '"' | '\'' => {
                if in_quote == Some(ch) {
                    in_quote = None;
                } else if in_quote.is_none() {
                    in_quote = Some(ch);
                }
            }
            '{' if in_quote.is_none() => {
                depth += 1;
            }
            '}' if in_quote.is_none() => {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}
