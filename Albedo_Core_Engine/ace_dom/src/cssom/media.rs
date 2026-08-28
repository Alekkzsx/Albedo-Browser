//! # W3C Media Queries Level 4 & Level 5
//!
//! Estruturas de contexto de mídia (`MediaContext`) e avaliador normativo de consultas `@media`
//! com suporte a tipos (`screen`, `print`, `all`), recursos de dimensão (`min-width`, `max-width`, `min-height`, `max-height`, `width`, `height`),
//! preferências (`prefers-color-scheme`), orientação (`portrait`, `landscape`) e operadores lógicos (`and`, `not`, `,`).

/// Tipos de mídia CSS normativos (W3C Media Queries Level 4 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MediaType {
    #[default]
    Screen,
    Print,
    Speech,
    All,
}

/// Preferência de esquema de cores do usuário (W3C Media Queries Level 5 §11.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
    NoPreference,
}

/// Orientação da viewport (W3C Media Queries Level 4 §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

/// Contexto de avaliação de `@media` queries.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaContext {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub media_type: MediaType,
    pub color_scheme: ColorScheme,
    pub orientation: Orientation,
    pub device_pixel_ratio: f32,
}

impl Default for MediaContext {
    fn default() -> Self {
        Self {
            viewport_width: 1280.0,
            viewport_height: 800.0,
            media_type: MediaType::Screen,
            color_scheme: ColorScheme::Light,
            orientation: Orientation::Landscape,
            device_pixel_ratio: 1.0,
        }
    }
}

impl MediaContext {
    /// Cria um novo contexto de mídia com valores padrão.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a largura e altura da viewport.
    pub fn with_viewport(mut self, width: f32, height: f32) -> Self {
        self.viewport_width = width;
        self.viewport_height = height;
        self.orientation = if width >= height {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        };
        self
    }

    /// Define o esquema de cores.
    pub fn with_color_scheme(mut self, color_scheme: ColorScheme) -> Self {
        self.color_scheme = color_scheme;
        self
    }

    /// Define o tipo de mídia.
    pub fn with_media_type(mut self, media_type: MediaType) -> Self {
        self.media_type = media_type;
        self
    }

    /// Define a razão de aspecto de pixels do dispositivo.
    pub fn with_device_pixel_ratio(mut self, dpr: f32) -> Self {
        self.device_pixel_ratio = dpr;
        self
    }
}

/// Avalia uma condição de `@media` (ex: `screen and (min-width: 600px), (prefers-color-scheme: dark)`)
/// contra um determinado `MediaContext`.
pub fn evaluate_media_query(condition: &str, ctx: &MediaContext) -> bool {
    let trimmed = condition.trim();
    if trimmed.is_empty() {
        return true;
    }

    // Lista de consultas separadas por vírgula (Disjunção / OR lógico)
    for single_query in split_top_level(trimmed, ',') {
        let single_trimmed = single_query.trim();
        if single_trimmed.is_empty() {
            continue;
        }
        if evaluate_single_query(single_trimmed, ctx) {
            return true;
        }
    }

    false
}

/// Avalia uma única consulta de mídia (ex: `not screen and (min-width: 600px)`).
fn evaluate_single_query(query: &str, ctx: &MediaContext) -> bool {
    let mut q = query.trim();
    if q.is_empty() {
        return true;
    }

    let mut is_not = false;
    if q.len() >= 4 && q[..4].eq_ignore_ascii_case("not ") {
        is_not = true;
        q = q[4..].trim();
    } else if q.len() >= 5 && q[..5].eq_ignore_ascii_case("only ") {
        q = q[5..].trim();
    }

    // Divide por 'and' no nível superior (Conjunção / AND lógico)
    let parts = split_and_terms(q);
    let mut result = true;

    for part in parts {
        let part_trimmed = part.trim();
        if part_trimmed.is_empty() {
            continue;
        }
        if !evaluate_media_term(part_trimmed, ctx) {
            result = false;
            break;
        }
    }

    if is_not {
        !result
    } else {
        result
    }
}

/// Avalia um termo de consulta de mídia: ou tipo de mídia ou recurso entre parênteses `(...)`.
fn evaluate_media_term(term: &str, ctx: &MediaContext) -> bool {
    let trimmed = term.trim();
    if trimmed.is_empty() {
        return true;
    }

    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inside = trimmed[1..trimmed.len() - 1].trim();
        evaluate_media_feature(inside, ctx)
    } else {
        // Tipo de mídia simples
        match trimmed.to_ascii_lowercase().as_str() {
            "all" => true,
            "screen" => ctx.media_type == MediaType::Screen || ctx.media_type == MediaType::All,
            "print" => ctx.media_type == MediaType::Print || ctx.media_type == MediaType::All,
            "speech" => ctx.media_type == MediaType::Speech || ctx.media_type == MediaType::All,
            _ => false,
        }
    }
}

/// Avalia uma expressão de recurso de mídia (ex: `min-width: 600px`, `prefers-color-scheme: dark`).
fn evaluate_media_feature(feature_expr: &str, ctx: &MediaContext) -> bool {
    let (name, val) = if let Some((n, v)) = feature_expr.split_once(':') {
        (n.trim().to_ascii_lowercase(), Some(v.trim()))
    } else {
        (feature_expr.trim().to_ascii_lowercase(), None)
    };

    match name.as_str() {
        "min-width" => {
            if let Some(val_str) = val {
                if let Some(len) = parse_css_length(val_str) {
                    return ctx.viewport_width >= len - 1e-4;
                }
            }
            false
        }
        "max-width" => {
            if let Some(val_str) = val {
                if let Some(len) = parse_css_length(val_str) {
                    return ctx.viewport_width <= len + 1e-4;
                }
            }
            false
        }
        "width" => {
            if let Some(val_str) = val {
                if let Some(len) = parse_css_length(val_str) {
                    return (ctx.viewport_width - len).abs() <= 1e-3;
                }
            }
            false
        }
        "min-height" => {
            if let Some(val_str) = val {
                if let Some(len) = parse_css_length(val_str) {
                    return ctx.viewport_height >= len - 1e-4;
                }
            }
            false
        }
        "max-height" => {
            if let Some(val_str) = val {
                if let Some(len) = parse_css_length(val_str) {
                    return ctx.viewport_height <= len + 1e-4;
                }
            }
            false
        }
        "height" => {
            if let Some(val_str) = val {
                if let Some(len) = parse_css_length(val_str) {
                    return (ctx.viewport_height - len).abs() <= 1e-3;
                }
            }
            false
        }
        "orientation" => {
            if let Some(val_str) = val {
                if val_str.eq_ignore_ascii_case("portrait") {
                    ctx.viewport_height >= ctx.viewport_width || ctx.orientation == Orientation::Portrait
                } else if val_str.eq_ignore_ascii_case("landscape") {
                    ctx.viewport_width >= ctx.viewport_height || ctx.orientation == Orientation::Landscape
                } else {
                    false
                }
            } else {
                true
            }
        }
        "prefers-color-scheme" => {
            if let Some(val_str) = val {
                if val_str.eq_ignore_ascii_case("dark") {
                    ctx.color_scheme == ColorScheme::Dark
                } else if val_str.eq_ignore_ascii_case("light") {
                    ctx.color_scheme == ColorScheme::Light
                } else if val_str.eq_ignore_ascii_case("no-preference") {
                    ctx.color_scheme == ColorScheme::NoPreference
                } else {
                    false
                }
            } else {
                true
            }
        }
        "min-device-pixel-ratio" | "-webkit-min-device-pixel-ratio" | "min-resolution" => {
            if let Some(val_str) = val {
                if let Some(dpr) = parse_css_resolution(val_str) {
                    return ctx.device_pixel_ratio >= dpr - 1e-4;
                }
            }
            false
        }
        "max-device-pixel-ratio" | "-webkit-max-device-pixel-ratio" | "max-resolution" => {
            if let Some(val_str) = val {
                if let Some(dpr) = parse_css_resolution(val_str) {
                    return ctx.device_pixel_ratio <= dpr + 1e-4;
                }
            }
            false
        }
        "device-pixel-ratio" | "-webkit-device-pixel-ratio" | "resolution" => {
            if let Some(val_str) = val {
                if let Some(dpr) = parse_css_resolution(val_str) {
                    return (ctx.device_pixel_ratio - dpr).abs() <= 1e-3;
                }
            }
            false
        }
        "color" => {
            if let Some(val_str) = val {
                val_str.parse::<f32>().map(|v| v > 0.0).unwrap_or(true)
            } else {
                true
            }
        }
        "hover" | "any-hover" => true,
        _ => false,
    }
}

/// Divide uma string por um delimitador no nível 0 (fora de parênteses e aspas).
fn split_top_level(s: &str, delim: char) -> Vec<&str> {
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
            if ch == '(' {
                paren_depth += 1;
            } else if ch == ')' {
                paren_depth = paren_depth.saturating_sub(1);
            } else if ch == delim && paren_depth == 0 {
                result.push(&s[start..idx]);
                start = idx + delim.len_utf8();
            }
        }
    }
    if start <= s.len() {
        result.push(&s[start..]);
    }
    result
}

/// Divide uma expressão de consulta de mídia por palavras-chave 'and' no nível 0.
fn split_and_terms(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut paren_depth: usize = 0;
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if b == b'(' {
            paren_depth += 1;
            i += 1;
        } else if b == b')' {
            paren_depth = paren_depth.saturating_sub(1);
            i += 1;
        } else if paren_depth == 0 {
            // Verifica se a palavra 'and' está aqui
            if i + 3 <= len && s[i..i + 3].eq_ignore_ascii_case("and") {
                let prev_ok = i == 0 || bytes[i - 1].is_ascii_whitespace() || bytes[i - 1] == b')';
                let next_idx = i + 3;
                let next_ok = next_idx == len
                    || bytes[next_idx].is_ascii_whitespace()
                    || bytes[next_idx] == b'(';
                if prev_ok && next_ok {
                    result.push(&s[start..i]);
                    start = next_idx;
                    i = next_idx;
                    continue;
                }
            }
            i += 1;
        } else {
            i += 1;
        }
    }

    if start <= len {
        result.push(&s[start..]);
    }
    result
}

/// Faz parsing de dimensões CSS com conversão para pixels (`px`).
pub fn parse_css_length(s: &str) -> Option<f32> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    if let Some(val_str) = lower.strip_suffix("px") {
        val_str.trim().parse::<f32>().ok()
    } else if let Some(val_str) = lower.strip_suffix("em") {
        val_str.trim().parse::<f32>().ok().map(|v| v * 16.0)
    } else if let Some(val_str) = lower.strip_suffix("rem") {
        val_str.trim().parse::<f32>().ok().map(|v| v * 16.0)
    } else if let Some(val_str) = lower.strip_suffix("in") {
        val_str.trim().parse::<f32>().ok().map(|v| v * 96.0)
    } else if let Some(val_str) = lower.strip_suffix("cm") {
        val_str.trim().parse::<f32>().ok().map(|v| v * (96.0 / 2.54))
    } else if let Some(val_str) = lower.strip_suffix("mm") {
        val_str.trim().parse::<f32>().ok().map(|v| v * (96.0 / 25.4))
    } else if let Some(val_str) = lower.strip_suffix("pt") {
        val_str.trim().parse::<f32>().ok().map(|v| v * (96.0 / 72.0))
    } else {
        trimmed.parse::<f32>().ok()
    }
}

/// Faz parsing de resoluções CSS (dppx, dpi, dpcm).
fn parse_css_resolution(s: &str) -> Option<f32> {
    let trimmed = s.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(val_str) = lower.strip_suffix("dppx") {
        val_str.trim().parse::<f32>().ok()
    } else if let Some(val_str) = lower.strip_suffix("dpi") {
        val_str.trim().parse::<f32>().ok().map(|v| v / 96.0)
    } else if let Some(val_str) = lower.strip_suffix("dpcm") {
        val_str.trim().parse::<f32>().ok().map(|v| v * 2.54 / 96.0)
    } else {
        trimmed.parse::<f32>().ok()
    }
}
