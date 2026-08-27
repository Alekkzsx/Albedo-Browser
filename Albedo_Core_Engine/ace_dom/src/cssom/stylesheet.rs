//! # CSSStyleSheet & Regras de Estilo (WHATWG CSSOM §6.2)
//!
//! Representação de folhas de estilo em cascata associadas a tags `<style>` e `<link rel="stylesheet">`,
//! com parsing balanceado de chaves para suporte a regras aninhadas (`@media`, `@keyframes`).

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
    /// Regra `@supports` — CSS Conditional Rules Level 3.
    /// Avalia suporte a propriedades/valores CSS antes de aplicar as regras internas.
    Supports {
        condition: SmolStr,
        rules: Vec<CSSRule>,
    },
    /// Regra `@layer` — CSS Cascade Layers.
    /// Agrupa regras em camadas de cascata nomeadas para controle explícito de prioridade.
    Layer {
        /// Nome da camada (pode ser vazia para declaração de layer anônima).
        name: SmolStr,
        rules: Vec<CSSRule>,
    },
    /// Regra `@container` — CSS Container Queries.
    /// Aplica regras condicionalmente baseadas nas dimensões do container pai.
    Container {
        /// Nome do container (opcional) e condição (ex: `sidebar (min-width: 700px)`).
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

    /// Faz o parsing de um bloco de texto CSS em uma `CSSStyleSheet`
    /// utilizando um parser com rastreamento de profundidade de chaves `{ ... }`.
    pub fn parse(css_text: &str) -> Self {
        let mut sheet = Self::new();
        let len = css_text.len();
        let mut cursor = 0;

        while cursor < len {
            cursor = skip_whitespace_and_comments(css_text, cursor);
            if cursor >= len {
                break;
            }

            let remaining = &css_text[cursor..];

            // 1. Trata regra @import (case-insensitive)
            if remaining.len() >= 7 && remaining[..7].eq_ignore_ascii_case("@import") {
                if let Some(semi_pos) = remaining.find(';') {
                    let import_stmt = remaining[..semi_pos].trim();
                    let mut raw_href = if import_stmt.len() >= 7 && import_stmt[..7].eq_ignore_ascii_case("@import") {
                        import_stmt[7..].trim()
                    } else {
                        import_stmt.trim()
                    };
                    if raw_href.len() >= 4 && raw_href[..4].eq_ignore_ascii_case("url(") && raw_href.ends_with(')') {
                        raw_href = raw_href[4..raw_href.len() - 1].trim();
                    }
                    let href = raw_href.trim_matches('\'').trim_matches('"').trim();
                    sheet.rules.push(CSSRule::Import {
                        href: SmolStr::new(href),
                    });
                    cursor += semi_pos + 1;
                    continue;
                } else {
                    // Sem ponto e vírgula, consome o resto
                    break;
                }
            }

            // 2. Encontra abertura do bloco `{`
            let open_brace_rel = find_top_level_open_brace(remaining);
            if let Some(open_rel) = open_brace_rel {
                let open_idx = cursor + open_rel;
                let preamble = css_text[cursor..open_idx].trim();

                // Encontra fechamento balanceado `}`
                if let Some(close_idx) = find_matching_brace(css_text, open_idx) {
                    let body = &css_text[open_idx + 1..close_idx];

                    if preamble.len() >= 6 && preamble[..6].eq_ignore_ascii_case("@media") {
                        let condition = preamble[6..].trim();
                        let inner_sheet = CSSStyleSheet::parse(body);
                        sheet.rules.push(CSSRule::Media {
                            condition: SmolStr::new(condition),
                            rules: inner_sheet.rules,
                        });
                    } else if preamble.len() >= 9 && preamble[..9].eq_ignore_ascii_case("@supports") {
                        // CSS Conditional Rules Level 3 — @supports (min-width: 768px)
                        let condition = preamble[9..].trim();
                        let inner_sheet = CSSStyleSheet::parse(body);
                        sheet.rules.push(CSSRule::Supports {
                            condition: SmolStr::new(condition),
                            rules: inner_sheet.rules,
                        });
                    } else if preamble.len() >= 6 && preamble[..6].eq_ignore_ascii_case("@layer") {
                        // CSS Cascade Layers — @layer utilities { ... }
                        let name = preamble[6..].trim();
                        let inner_sheet = CSSStyleSheet::parse(body);
                        sheet.rules.push(CSSRule::Layer {
                            name: SmolStr::new(name),
                            rules: inner_sheet.rules,
                        });
                    } else if preamble.len() >= 10 && preamble[..10].eq_ignore_ascii_case("@container") {
                        // CSS Container Queries — @container sidebar (min-width: 700px) { ... }
                        let condition = preamble[10..].trim();
                        let inner_sheet = CSSStyleSheet::parse(body);
                        sheet.rules.push(CSSRule::Container {
                            condition: SmolStr::new(condition),
                            rules: inner_sheet.rules,
                        });
                    } else if (preamble.len() >= 10 && preamble[..10].eq_ignore_ascii_case("@keyframes"))
                        || (preamble.len() >= 18 && preamble[..18].eq_ignore_ascii_case("@-webkit-keyframes"))
                    {
                        let name = preamble
                            .split_whitespace()
                            .nth(1)
                            .unwrap_or("")
                            .trim();
                        sheet.rules.push(CSSRule::Keyframes {
                            name: SmolStr::new(name),
                            css_text: SmolStr::new(body),
                        });
                    } else if !preamble.is_empty() {
                        let mut parsed_selectors = Vec::new();
                        for sel_part in split_top_level_selectors(preamble) {
                            let trimmed_sel = sel_part.trim();
                            if !trimmed_sel.is_empty() {
                                if let Some(sel) = ComplexSelector::parse(trimmed_sel) {
                                    parsed_selectors.push(sel);
                                }
                            }
                        }

                        let style_decl = CSSStyleDeclaration::parse(body);
                        sheet.rules.push(CSSRule::Style(Box::new(CSSStyleRule {
                            selector_text: SmolStr::new(preamble),
                            selectors: parsed_selectors,
                            style: style_decl,
                        })));
                    }

                    cursor = close_idx + 1;
                    continue;
                } else {
                    // Chave sem fechamento
                    break;
                }
            } else {
                // Nenhuma chave encontrada
                break;
            }
        }

        sheet
    }

    /// Adiciona uma regra de estilo à folha.
    pub fn insert_rule(&mut self, rule: CSSRule) {
        self.rules.push(rule);
    }
}

/// Avança o cursor pulando espaços em branco e comentários `/* ... */`.
fn skip_whitespace_and_comments(s: &str, mut idx: usize) -> usize {
    let bytes = s.as_bytes();
    let len = bytes.len();
    while idx < len {
        if bytes[idx].is_ascii_whitespace() {
            idx += 1;
        } else if idx + 1 < len && bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
            if let Some(end) = s[idx + 2..].find("*/") {
                idx = idx + 2 + end + 2;
            } else {
                idx = len;
            }
        } else {
            break;
        }
    }
    idx
}

/// Encontra a primeira chave de abertura `{` no nível 0 (fora de aspas e comentários).
fn find_top_level_open_brace(s: &str) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let mut in_comment = false;
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let (byte_offset, ch) = chars[i];
        if in_comment {
            if ch == '*' && i + 1 < len && chars[i + 1].1 == '/' {
                in_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }

        if ch == '/' && i + 1 < len && chars[i + 1].1 == '*' && !in_single && !in_double {
            in_comment = true;
            i += 2;
            continue;
        }

        if ch == '\\' && (in_single || in_double) && i + 1 < len {
            i += 2;
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
        } else if ch == '"' && !in_single {
            in_double = !in_double;
        } else if !in_single && !in_double && ch == '{' {
            return Some(byte_offset);
        }
        i += 1;
    }
    None
}

/// Encontra a chave de fechamento correspondente `}` para uma chave aberta em `open_idx`.
fn find_matching_brace(s: &str, open_idx: usize) -> Option<usize> {
    let chars: Vec<(usize, char)> = s[open_idx + 1..].char_indices().collect();
    let mut depth: usize = 1;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_comment = false;
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let (byte_offset, ch) = chars[i];
        if in_comment {
            if ch == '*' && i + 1 < len && chars[i + 1].1 == '/' {
                in_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }

        if ch == '/' && i + 1 < len && chars[i + 1].1 == '*' && !in_single && !in_double {
            in_comment = true;
            i += 2;
            continue;
        }

        if ch == '\\' && (in_single || in_double) && i + 1 < len {
            i += 2;
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
        } else if ch == '"' && !in_single {
            in_double = !in_double;
        } else if !in_single && !in_double {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    return Some(open_idx + 1 + byte_offset);
                }
            }
        }
        i += 1;
    }
    None
}

/// Divide a lista de seletores por vírgula no nível 0 (fora de parênteses como `:not(...)` e aspas).
fn split_top_level_selectors(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut paren_depth: usize = 0;
    let mut in_single = false;
    let mut in_double = false;

    for (idx, ch) in s.char_indices() {
        if ch == '\'' && !in_double {
            in_single = !in_single;
        } else if ch == '"' && !in_single {
            in_double = !in_double;
        } else if !in_single && !in_double {
            if ch == '(' || ch == '[' {
                paren_depth += 1;
            } else if ch == ')' || ch == ']' {
                paren_depth = paren_depth.saturating_sub(1);
            } else if ch == ',' && paren_depth == 0 {
                result.push(&s[start..idx]);
                start = idx + 1;
            }
        }
    }
    if start <= s.len() {
        result.push(&s[start..]);
    }
    result
}
