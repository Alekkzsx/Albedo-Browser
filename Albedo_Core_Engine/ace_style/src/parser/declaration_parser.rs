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
            '{' if in_quote.is_none() => {
                // Descarta qualquer prefixo de seletor (ex: "p {")
                curr_chunk.clear();
            }
            '}' if in_quote.is_none() => {
                let trimmed = curr_chunk.trim();
                if !trimmed.is_empty() {
                    parse_single_declaration(trimmed, &mut declarations);
                }
                curr_chunk.clear();
            }
            '\n' | '\r' => {
                if in_quote.is_some() {
                    // W3C CSS Syntax L3: nova linha encerra string como bad-string e invalida a declaração
                    in_quote = None;
                    curr_chunk.clear();
                } else {
                    curr_chunk.push(ch);
                }
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
fn expand_shorthand(prop: &str, val: &str, important: bool, output: &mut Vec<ParsedDeclaration>) {
    match prop {
        "margin" => {
            let parts = split_whitespace_tokens(val);
            match parts.len() {
                1 => {
                    output.push(ParsedDeclaration::new("margin-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("margin-right", &parts[0], important));
                    output.push(ParsedDeclaration::new(
                        "margin-bottom",
                        &parts[0],
                        important,
                    ));
                    output.push(ParsedDeclaration::new("margin-left", &parts[0], important));
                }
                2 => {
                    output.push(ParsedDeclaration::new("margin-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("margin-right", &parts[1], important));
                    output.push(ParsedDeclaration::new(
                        "margin-bottom",
                        &parts[0],
                        important,
                    ));
                    output.push(ParsedDeclaration::new("margin-left", &parts[1], important));
                }
                3 => {
                    output.push(ParsedDeclaration::new("margin-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("margin-right", &parts[1], important));
                    output.push(ParsedDeclaration::new(
                        "margin-bottom",
                        &parts[2],
                        important,
                    ));
                    output.push(ParsedDeclaration::new("margin-left", &parts[1], important));
                }
                4 => {
                    output.push(ParsedDeclaration::new("margin-top", &parts[0], important));
                    output.push(ParsedDeclaration::new("margin-right", &parts[1], important));
                    output.push(ParsedDeclaration::new(
                        "margin-bottom",
                        &parts[2],
                        important,
                    ));
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
                    output.push(ParsedDeclaration::new(
                        "padding-right",
                        &parts[0],
                        important,
                    ));
                    output.push(ParsedDeclaration::new(
                        "padding-bottom",
                        &parts[0],
                        important,
                    ));
                    output.push(ParsedDeclaration::new("padding-left", &parts[0], important));
                }
                2 => {
                    output.push(ParsedDeclaration::new("padding-top", &parts[0], important));
                    output.push(ParsedDeclaration::new(
                        "padding-right",
                        &parts[1],
                        important,
                    ));
                    output.push(ParsedDeclaration::new(
                        "padding-bottom",
                        &parts[0],
                        important,
                    ));
                    output.push(ParsedDeclaration::new("padding-left", &parts[1], important));
                }
                3 => {
                    output.push(ParsedDeclaration::new("padding-top", &parts[0], important));
                    output.push(ParsedDeclaration::new(
                        "padding-right",
                        &parts[1],
                        important,
                    ));
                    output.push(ParsedDeclaration::new(
                        "padding-bottom",
                        &parts[2],
                        important,
                    ));
                    output.push(ParsedDeclaration::new("padding-left", &parts[1], important));
                }
                4 => {
                    output.push(ParsedDeclaration::new("padding-top", &parts[0], important));
                    output.push(ParsedDeclaration::new(
                        "padding-right",
                        &parts[1],
                        important,
                    ));
                    output.push(ParsedDeclaration::new(
                        "padding-bottom",
                        &parts[2],
                        important,
                    ));
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
                output.push(ParsedDeclaration::new(
                    "border-top-width",
                    &width,
                    important,
                ));
                output.push(ParsedDeclaration::new(
                    "border-right-width",
                    &width,
                    important,
                ));
                output.push(ParsedDeclaration::new(
                    "border-bottom-width",
                    &width,
                    important,
                ));
                output.push(ParsedDeclaration::new(
                    "border-left-width",
                    &width,
                    important,
                ));
            }
            if let Some(style) = s {
                output.push(ParsedDeclaration::new(
                    "border-top-style",
                    &style,
                    important,
                ));
                output.push(ParsedDeclaration::new(
                    "border-right-style",
                    &style,
                    important,
                ));
                output.push(ParsedDeclaration::new(
                    "border-bottom-style",
                    &style,
                    important,
                ));
                output.push(ParsedDeclaration::new(
                    "border-left-style",
                    &style,
                    important,
                ));
            }
            if let Some(color) = c {
                output.push(ParsedDeclaration::new(
                    "border-top-color",
                    &color,
                    important,
                ));
                output.push(ParsedDeclaration::new(
                    "border-right-color",
                    &color,
                    important,
                ));
                output.push(ParsedDeclaration::new(
                    "border-bottom-color",
                    &color,
                    important,
                ));
                output.push(ParsedDeclaration::new(
                    "border-left-color",
                    &color,
                    important,
                ));
            }
        }
        "border-top" | "border-right" | "border-bottom" | "border-left" => {
            let side = prop.strip_prefix("border-").unwrap();
            let (w, s, c) = parse_border_components(val);
            if let Some(width) = w {
                output.push(ParsedDeclaration::new(
                    format!("border-{}-width", side),
                    &width,
                    important,
                ));
            }
            if let Some(style) = s {
                output.push(ParsedDeclaration::new(
                    format!("border-{}-style", side),
                    &style,
                    important,
                ));
            }
            if let Some(color) = c {
                output.push(ParsedDeclaration::new(
                    format!("border-{}-color", side),
                    &color,
                    important,
                ));
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
        "font" => {
            parse_font_shorthand(val, important, output);
        }
        "margin-inline" => {
            let parts = split_whitespace_tokens(val);
            if parts.len() == 1 {
                output.push(ParsedDeclaration::new("margin-inline-start", &parts[0], important));
                output.push(ParsedDeclaration::new("margin-inline-end", &parts[0], important));
            } else if parts.len() >= 2 {
                output.push(ParsedDeclaration::new("margin-inline-start", &parts[0], important));
                output.push(ParsedDeclaration::new("margin-inline-end", &parts[1], important));
            }
        }
        "margin-block" => {
            let parts = split_whitespace_tokens(val);
            if parts.len() == 1 {
                output.push(ParsedDeclaration::new("margin-block-start", &parts[0], important));
                output.push(ParsedDeclaration::new("margin-block-end", &parts[0], important));
            } else if parts.len() >= 2 {
                output.push(ParsedDeclaration::new("margin-block-start", &parts[0], important));
                output.push(ParsedDeclaration::new("margin-block-end", &parts[1], important));
            }
        }
        "padding-inline" => {
            let parts = split_whitespace_tokens(val);
            if parts.len() == 1 {
                output.push(ParsedDeclaration::new("padding-inline-start", &parts[0], important));
                output.push(ParsedDeclaration::new("padding-inline-end", &parts[0], important));
            } else if parts.len() >= 2 {
                output.push(ParsedDeclaration::new("padding-inline-start", &parts[0], important));
                output.push(ParsedDeclaration::new("padding-inline-end", &parts[1], important));
            }
        }
        "padding-block" => {
            let parts = split_whitespace_tokens(val);
            if parts.len() == 1 {
                output.push(ParsedDeclaration::new("padding-block-start", &parts[0], important));
                output.push(ParsedDeclaration::new("padding-block-end", &parts[0], important));
            } else if parts.len() >= 2 {
                output.push(ParsedDeclaration::new("padding-block-start", &parts[0], important));
                output.push(ParsedDeclaration::new("padding-block-end", &parts[1], important));
            }
        }
        "inset" => {
            let parts = split_whitespace_tokens(val);
            match parts.len() {
                1 => {
                    output.push(ParsedDeclaration::new("top", &parts[0], important));
                    output.push(ParsedDeclaration::new("right", &parts[0], important));
                    output.push(ParsedDeclaration::new("bottom", &parts[0], important));
                    output.push(ParsedDeclaration::new("left", &parts[0], important));
                }
                2 => {
                    output.push(ParsedDeclaration::new("top", &parts[0], important));
                    output.push(ParsedDeclaration::new("right", &parts[1], important));
                    output.push(ParsedDeclaration::new("bottom", &parts[0], important));
                    output.push(ParsedDeclaration::new("left", &parts[1], important));
                }
                3 => {
                    output.push(ParsedDeclaration::new("top", &parts[0], important));
                    output.push(ParsedDeclaration::new("right", &parts[1], important));
                    output.push(ParsedDeclaration::new("bottom", &parts[2], important));
                    output.push(ParsedDeclaration::new("left", &parts[1], important));
                }
                4 => {
                    output.push(ParsedDeclaration::new("top", &parts[0], important));
                    output.push(ParsedDeclaration::new("right", &parts[1], important));
                    output.push(ParsedDeclaration::new("bottom", &parts[2], important));
                    output.push(ParsedDeclaration::new("left", &parts[3], important));
                }
                _ => {
                    output.push(ParsedDeclaration::new(prop, val, important));
                }
            }
        }
        "inset-inline" => {
            let parts = split_whitespace_tokens(val);
            if parts.len() == 1 {
                output.push(ParsedDeclaration::new("inset-inline-start", &parts[0], important));
                output.push(ParsedDeclaration::new("inset-inline-end", &parts[0], important));
            } else if parts.len() >= 2 {
                output.push(ParsedDeclaration::new("inset-inline-start", &parts[0], important));
                output.push(ParsedDeclaration::new("inset-inline-end", &parts[1], important));
            }
        }
        "inset-block" => {
            let parts = split_whitespace_tokens(val);
            if parts.len() == 1 {
                output.push(ParsedDeclaration::new("inset-block-start", &parts[0], important));
                output.push(ParsedDeclaration::new("inset-block-end", &parts[0], important));
            } else if parts.len() >= 2 {
                output.push(ParsedDeclaration::new("inset-block-start", &parts[0], important));
                output.push(ParsedDeclaration::new("inset-block-end", &parts[1], important));
            }
        }
        _ => {
            output.push(ParsedDeclaration::new(prop, val, important));
        }
    }
}

fn parse_font_shorthand(val: &str, important: bool, output: &mut Vec<ParsedDeclaration>) {
    let tokens = split_whitespace_tokens(val);
    if tokens.is_empty() {
        return;
    }

    let mut style = None;
    let mut variant = None;
    let mut weight = None;
    let mut size = None;
    let mut line_height = None;
    let mut family_tokens = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        let lower = tok.to_ascii_lowercase();

        // Se já encontramos o size, todo o restante é family
        if size.is_some() {
            family_tokens.push(tok.clone());
            i += 1;
            continue;
        }

        // Verifica se é size (contém '/' ou é formato de tamanho)
        if is_font_size_token(&lower) {
            if let Some((s, lh)) = tok.split_once('/') {
                size = Some(s.to_string());
                line_height = Some(lh.to_string());
            } else if i + 2 < tokens.len() && tokens[i + 1] == "/" {
                size = Some(tok.clone());
                line_height = Some(tokens[i + 2].clone());
                i += 2;
            } else {
                size = Some(tok.clone());
            }
            i += 1;
            continue;
        }

        // Tenta style
        if (lower == "italic" || lower == "oblique") && style.is_none() {
            style = Some(tok.clone());
            i += 1;
            continue;
        }

        // Tenta variant
        if lower == "small-caps" && variant.is_none() {
            variant = Some(tok.clone());
            i += 1;
            continue;
        }

        // Tenta weight
        if (lower == "bold"
            || lower == "bolder"
            || lower == "lighter"
            || lower == "100"
            || lower == "200"
            || lower == "300"
            || lower == "400"
            || lower == "500"
            || lower == "600"
            || lower == "700"
            || lower == "800"
            || lower == "900")
            && weight.is_none()
        {
            weight = Some(tok.clone());
            i += 1;
            continue;
        }

        if lower == "normal" {
            if style.is_none() {
                style = Some(tok.clone());
            } else if variant.is_none() {
                variant = Some(tok.clone());
            } else if weight.is_none() {
                weight = Some(tok.clone());
            }
            i += 1;
            continue;
        }

        // Fallback: se não bateu com nenhum dos anteriores, assume size
        if let Some((s, lh)) = tok.split_once('/') {
            size = Some(s.to_string());
            line_height = Some(lh.to_string());
        } else {
            size = Some(tok.clone());
        }
        i += 1;
    }

    if let Some(s) = size {
        output.push(ParsedDeclaration::new("font-size", &s, important));
    }
    if let Some(lh) = line_height {
        output.push(ParsedDeclaration::new("line-height", &lh, important));
    }
    output.push(ParsedDeclaration::new(
        "font-style",
        style.as_deref().unwrap_or("normal"),
        important,
    ));
    output.push(ParsedDeclaration::new(
        "font-variant",
        variant.as_deref().unwrap_or("normal"),
        important,
    ));
    output.push(ParsedDeclaration::new(
        "font-weight",
        weight.as_deref().unwrap_or("normal"),
        important,
    ));
    if !family_tokens.is_empty() {
        output.push(ParsedDeclaration::new(
            "font-family",
            family_tokens.join(" "),
            important,
        ));
    }
}

fn is_font_size_token(s: &str) -> bool {
    if s.contains('/') {
        return true;
    }
    s.ends_with("px")
        || s.ends_with("em")
        || s.ends_with("rem")
        || s.ends_with('%')
        || s.ends_with("pt")
        || s.ends_with("vh")
        || s.ends_with("vw")
        || s.ends_with("ch")
        || s.starts_with("calc(")
        || s.starts_with("min(")
        || s.starts_with("max(")
        || s.starts_with("clamp(")
        || matches!(
            s,
            "xx-small"
                | "x-small"
                | "small"
                | "medium"
                | "large"
                | "x-large"
                | "xx-large"
                | "smaller"
                | "larger"
        )
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
        "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset",
        "outset",
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
