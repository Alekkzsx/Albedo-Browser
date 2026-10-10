//! # Parser de Declarações CSS e Expansão de Shorthands
//!
//! Extração de pares `propriedade: valor [!important]` e desdobramento canônico de shorthands.

use ace_core::intern::Atom;
use smol_str::SmolStr;

/// Representação de uma declaração CSS parseada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDeclaration {
    /// Nome canônico da propriedade
    pub property: Atom,
    /// Valor textual da declaração
    pub value: SmolStr,
    /// Sinalizador de precedência `!important`
    pub important: bool,
}

impl ParsedDeclaration {
    pub fn new(property: impl AsRef<str>, value: impl AsRef<str>, important: bool) -> Self {
        let lower = property.as_ref().trim().to_ascii_lowercase();
        Self {
            property: Atom::new(&lower),
            value: SmolStr::new(value.as_ref().trim()),
            important,
        }
    }
}

/// Analisa um bloco de declarações CSS (conteúdo dentro de `{ ... }` ou atributo `style="..."`).
pub fn parse_declarations(input: &str) -> Vec<ParsedDeclaration> {
    let mut declarations = Vec::new();
    let mut curr_chunk = String::new();
    let mut in_quote: Option<char> = None;
    let mut paren_depth = 0;

    for ch in input.chars() {
        match ch {
            '\n' | '\r' => {
                in_quote = None;
                curr_chunk.push(ch);
            }
            '"' | '\'' => {
                if in_quote == Some(ch) {
                    in_quote = None;
                } else if in_quote.is_none() {
                    in_quote = Some(ch);
                }
                curr_chunk.push(ch);
            }
            '(' if in_quote.is_none() => {
                paren_depth += 1;
                curr_chunk.push(ch);
            }
            ')' if in_quote.is_none() => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
                curr_chunk.push(ch);
            }
            ';' if in_quote.is_none() && paren_depth == 0 => {
                let trimmed = curr_chunk.trim();
                if !trimmed.is_empty() {
                    parse_single_declaration(trimmed, &mut declarations);
                }
                curr_chunk.clear();
            }
            _ => {
                curr_chunk.push(ch);
            }
        }
    }

    let trimmed = curr_chunk.trim();
    if !trimmed.is_empty() {
        parse_single_declaration(trimmed, &mut declarations);
    }

    declarations
}

fn parse_single_declaration(decl_str: &str, output: &mut Vec<ParsedDeclaration>) {
    let Some((prop_raw, mut val_raw)) = decl_str.split_once(':') else {
        return;
    };

    let prop = prop_raw.trim().to_ascii_lowercase();
    if prop.is_empty() {
        return;
    }

    let mut important = false;
    let trimmed_val = val_raw.trim();

    // Detecção e remoção de !important
    if let Some(pos) = find_important_flag(trimmed_val) {
        important = true;
        val_raw = &trimmed_val[..pos];
    } else {
        val_raw = trimmed_val;
    }

    let val = val_raw.trim();
    if val.is_empty() {
        return;
    }

    // Se for custom property (--*), preserva exatamente como está (case-sensitive)
    if prop.starts_with("--") {
        output.push(ParsedDeclaration {
            property: Atom::new(&prop),
            value: SmolStr::new(val),
            important,
        });
        return;
    }

    // Expansão de shorthands conhecidos
    expand_shorthand(&prop, val, important, output);
}

fn find_important_flag(s: &str) -> Option<usize> {
    let lower = s.to_ascii_lowercase();
    if let Some(idx) = lower.rfind("!important") {
        let after = &lower[idx + 10..].trim();
        if after.is_empty() {
            return Some(idx);
        }
    }
    None
}

/// Expande shorthands CSS em propriedades canônicas primitivas.
fn expand_shorthand(
    prop: &str,
    val: &str,
    important: bool,
    output: &mut Vec<ParsedDeclaration>,
) {
    match prop {
        "margin" => {
            let parts = split_whitespace_tokens(val);
            match parts.len() {
                1 => {
                    output.push(ParsedDeclaration::new("margin-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("margin-right", &parts[0], important));
                    output.push(ParsedDeclaration::new("margin-bottom", &parts[0], important));
                    output.push(ParsedDeclaration::new("margin-left", &parts[0], important));
                }
                2 => {
                    output.push(ParsedDeclaration::new("margin-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("margin-right", &parts[1], important));
                    output.push(ParsedDeclaration::new("margin-bottom", &parts[0], important));
                    output.push(ParsedDeclaration::new("margin-left", &parts[1], important));
                }
                3 => {
                    output.push(ParsedDeclaration::new("margin-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("margin-right", &parts[1], important));
                    output.push(ParsedDeclaration::new("margin-bottom", &parts[2], important));
                    output.push(ParsedDeclaration::new("margin-left", &parts[1], important));
                }
                4 => {
                    output.push(ParsedDeclaration::new("margin-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("margin-right", &parts[1], important));
                    output.push(ParsedDeclaration::new("margin-bottom", &parts[2], important));
                    output.push(ParsedDeclaration::new("margin-left", &parts[3], important));
                }
                _ => {
                    output.push(ParsedDeclaration::new(prop, val, important));
                }
            }
        }
        "padding" => {
            let parts = split_whitespace_tokens(val);
            match parts.len() {
                1 => {
                    output.push(ParsedDeclaration::new("padding-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("padding-right", &parts[0], important));
                    output.push(ParsedDeclaration::new("padding-bottom", &parts[0], important));
                    output.push(ParsedDeclaration::new("padding-left", &parts[0], important));
                }
                2 => {
                    output.push(ParsedDeclaration::new("padding-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("padding-right", &parts[1], important));
                    output.push(ParsedDeclaration::new("padding-bottom", &parts[0], important));
                    output.push(ParsedDeclaration::new("padding-left", &parts[1], important));
                }
                3 => {
                    output.push(ParsedDeclaration::new("padding-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("padding-right", &parts[1], important));
                    output.push(ParsedDeclaration::new("padding-bottom", &parts[2], important));
                    output.push(ParsedDeclaration::new("padding-left", &parts[1], important));
                }
                4 => {
                    output.push(ParsedDeclaration::new("padding-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("padding-right", &parts[1], important));
                    output.push(ParsedDeclaration::new("padding-bottom", &parts[2], important));
                    output.push(ParsedDeclaration::new("padding-left", &parts[3], important));
                }
                _ => {
                    output.push(ParsedDeclaration::new(prop, val, important));
                }
            }
        }
        "border" => {
            let (w, s, c) = parse_border_components(val);
            if let Some(width) = w {
                output.push(ParsedDeclaration::new("border-top-width", &width, important));
                output.push(ParsedDeclaration::new("border-right-width", &width, important));
                output.push(ParsedDeclaration::new("border-bottom-width", &width, important));
                output.push(ParsedDeclaration::new("border-left-width", &width, important));
            }
            if let Some(style) = s {
                output.push(ParsedDeclaration::new("border-top-style", &style, important));
                output.push(ParsedDeclaration::new("border-right-style", &style, important));
                output.push(ParsedDeclaration::new("border-bottom-style", &style, important));
                output.push(ParsedDeclaration::new("border-left-style", &style, important));
            }
            if let Some(color) = c {
                output.push(ParsedDeclaration::new("border-top-color", &color, important));
                output.push(ParsedDeclaration::new("border-right-color", &color, important));
                output.push(ParsedDeclaration::new("border-bottom-color", &color, important));
                output.push(ParsedDeclaration::new("border-left-color", &color, important));
            }
        }
        "border-top" | "border-right" | "border-bottom" | "border-left" => {
            let side = prop.strip_prefix("border-").unwrap();
            let (w, s, c) = parse_border_components(val);
            if let Some(width) = w {
                output.push(ParsedDeclaration::new(format!("border-{}-width", side), &width, important));
            }
            if let Some(style) = s {
                output.push(ParsedDeclaration::new(format!("border-{}-style", side), &style, important));
            }
            if let Some(color) = c {
                output.push(ParsedDeclaration::new(format!("border-{}-color", side), &color, important));
            }
        }
        "flex" => {
            let parts = split_whitespace_tokens(val);
            if parts.len() == 1 {
                if parts[0] == "none" {
                    output.push(ParsedDeclaration::new("flex-grow", "0", important));
                    output.push(ParsedDeclaration::new("flex-shrink", "0", important));
                    output.push(ParsedDeclaration::new("flex-basis", "auto", important));
                } else if parts[0] == "auto" {
                    output.push(ParsedDeclaration::new("flex-grow", "1", important));
                    output.push(ParsedDeclaration::new("flex-shrink", "1", important));
                    output.push(ParsedDeclaration::new("flex-basis", "auto", important));
                } else if parts[0] == "initial" {
                    output.push(ParsedDeclaration::new("flex-grow", "0", important));
                    output.push(ParsedDeclaration::new("flex-shrink", "1", important));
                    output.push(ParsedDeclaration::new("flex-basis", "auto", important));
                } else if parts[0].parse::<f32>().is_ok() {
                    output.push(ParsedDeclaration::new("flex-grow", &parts[0], important));
                    output.push(ParsedDeclaration::new("flex-shrink", "1", important));
                    output.push(ParsedDeclaration::new("flex-basis", "0px", important));
                } else {
                    output.push(ParsedDeclaration::new("flex-grow", "1", important));
                    output.push(ParsedDeclaration::new("flex-shrink", "1", important));
                    output.push(ParsedDeclaration::new("flex-basis", &parts[0], important));
                }
            } else if parts.len() == 2 {
                output.push(ParsedDeclaration::new("flex-grow", &parts[0], important));
                if parts[1].parse::<f32>().is_ok() {
                    output.push(ParsedDeclaration::new("flex-shrink", &parts[1], important));
                    output.push(ParsedDeclaration::new("flex-basis", "0px", important));
                } else {
                    output.push(ParsedDeclaration::new("flex-shrink", "1", important));
                    output.push(ParsedDeclaration::new("flex-basis", &parts[1], important));
                }
            } else if parts.len() >= 3 {
                output.push(ParsedDeclaration::new("flex-grow", &parts[0], important));
                output.push(ParsedDeclaration::new("flex-shrink", &parts[1], important));
                output.push(ParsedDeclaration::new("flex-basis", &parts[2], important));
            }
        }
        "gap" => {
            let parts = split_whitespace_tokens(val);
            if parts.len() == 1 {
                output.push(ParsedDeclaration::new("row-gap", &parts[0], important));
                output.push(ParsedDeclaration::new("column-gap", &parts[0], important));
            } else if parts.len() >= 2 {
                output.push(ParsedDeclaration::new("row-gap", &parts[0], important));
                output.push(ParsedDeclaration::new("column-gap", &parts[1], important));
            }
        }
        "overflow" => {
            let parts = split_whitespace_tokens(val);
            if parts.len() == 1 {
                output.push(ParsedDeclaration::new("overflow-x", &parts[0], important));
                output.push(ParsedDeclaration::new("overflow-y", &parts[0], important));
            } else if parts.len() >= 2 {
                output.push(ParsedDeclaration::new("overflow-x", &parts[0], important));
                output.push(ParsedDeclaration::new("overflow-y", &parts[1], important));
            }
        }
        _ => {
            output.push(ParsedDeclaration::new(prop, val, important));
        }
    }
}

fn split_whitespace_tokens(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut curr = String::new();
    let mut paren_depth = 0;

    for ch in s.chars() {
        if ch == '(' {
            paren_depth += 1;
            curr.push(ch);
        } else if ch == ')' {
            if paren_depth > 0 {
                paren_depth -= 1;
            }
            curr.push(ch);
        } else if ch.is_whitespace() && paren_depth == 0 {
            if !curr.is_empty() {
                tokens.push(curr.clone());
                curr.clear();
            }
        } else {
            curr.push(ch);
        }
    }
    if !curr.is_empty() {
        tokens.push(curr);
    }
    tokens
}

fn parse_border_components(s: &str) -> (Option<String>, Option<String>, Option<String>) {
    let tokens = split_whitespace_tokens(s);
    let mut width = None;
    let mut style = None;
    let mut color = None;

    let styles = [
        "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset", "outset",
    ];

    for tok in tokens {
        let lower = tok.to_ascii_lowercase();
        if styles.contains(&lower.as_str()) {
            style = Some(tok);
        } else if lower.ends_with("px")
            || lower.ends_with("em")
            || lower.ends_with("rem")
            || lower.ends_with("pt")
            || lower == "thin"
            || lower == "medium"
            || lower == "thick"
            || lower == "0"
        {
            width = Some(tok);
        } else {
            color = Some(tok);
        }
    }

    (width, style, color)
}
