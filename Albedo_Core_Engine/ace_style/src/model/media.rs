//! # Consultas de Mídia CSS (`@media`) e Contexto do Dispositivo
//!
//! Avaliação de media queries contra viewport, orientação e preferências do sistema.

use crate::model::stylesheet::CSSRule;

/// Tipo de mídia alvo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MediaType {
    #[default]
    All,
    Screen,
    Print,
}

impl MediaType {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "all" => Some(Self::All),
            "screen" => Some(Self::Screen),
            "print" => Some(Self::Print),
            _ => None,
        }
    }
}

/// Orientação da viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Orientation {
    #[default]
    Landscape,
    Portrait,
}

/// Esquema de cores preferido pelo usuário (`prefers-color-scheme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
}

/// Contexto de mídia do ambiente de renderização (viewport, tela, tema).
#[derive(Debug, Clone, PartialEq)]
pub struct MediaContext {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub orientation: Orientation,
    pub color_scheme: ColorScheme,
    pub media_type: MediaType,
    pub device_pixel_ratio: f32,
}

impl Default for MediaContext {
    fn default() -> Self {
        Self {
            viewport_width: 1280.0,
            viewport_height: 720.0,
            orientation: Orientation::Landscape,
            color_scheme: ColorScheme::Light,
            media_type: MediaType::Screen,
            device_pixel_ratio: 1.0,
        }
    }
}

/// Expressão atômica de consulta de mídia.
#[derive(Debug, Clone, PartialEq)]
pub enum MediaFeature {
    MinWidth(f32),
    MaxWidth(f32),
    MinHeight(f32),
    MaxHeight(f32),
    Orientation(Orientation),
    PrefersColorScheme(ColorScheme),
}

impl MediaFeature {
    pub fn matches(&self, ctx: &MediaContext) -> bool {
        match self {
            Self::MinWidth(w) => ctx.viewport_width >= *w,
            Self::MaxWidth(w) => ctx.viewport_width <= *w,
            Self::MinHeight(h) => ctx.viewport_height >= *h,
            Self::MaxHeight(h) => ctx.viewport_height <= *h,
            Self::Orientation(o) => ctx.orientation == *o,
            Self::PrefersColorScheme(c) => ctx.color_scheme == *c,
        }
    }

    pub fn parse(feature_str: &str) -> Option<Self> {
        let trimmed = feature_str.trim().trim_start_matches('(').trim_end_matches(')');
        let (name, val) = trimmed.split_once(':')?;
        let name = name.trim().to_ascii_lowercase();
        let val = val.trim();

        match name.as_str() {
            "min-width" => parse_px(val).map(Self::MinWidth),
            "max-width" => parse_px(val).map(Self::MaxWidth),
            "min-height" => parse_px(val).map(Self::MinHeight),
            "max-height" => parse_px(val).map(Self::MaxHeight),
            "orientation" => match val.to_ascii_lowercase().as_str() {
                "portrait" => Some(Self::Orientation(Orientation::Portrait)),
                "landscape" => Some(Self::Orientation(Orientation::Landscape)),
                _ => None,
            },
            "prefers-color-scheme" => match val.to_ascii_lowercase().as_str() {
                "dark" => Some(Self::PrefersColorScheme(ColorScheme::Dark)),
                "light" => Some(Self::PrefersColorScheme(ColorScheme::Light)),
                _ => None,
            },
            _ => None,
        }
    }
}

fn parse_px(s: &str) -> Option<f32> {
    let lower = s.trim().to_ascii_lowercase();
    if let Some(num) = lower.strip_suffix("px") {
        num.trim().parse::<f32>().ok()
    } else {
        lower.parse::<f32>().ok()
    }
}

/// Representação de uma cláusula de consulta de mídia (`@media`).
#[derive(Debug, Clone, PartialEq)]
pub struct MediaCondition {
    pub negated: bool,
    pub media_type: Option<MediaType>,
    pub features: Vec<MediaFeature>,
}

impl MediaCondition {
    pub fn matches(&self, ctx: &MediaContext) -> bool {
        let mut result = true;

        if let Some(mt) = self.media_type {
            if mt != MediaType::All && ctx.media_type != MediaType::All && mt != ctx.media_type {
                result = false;
            }
        }

        if result {
            for f in &self.features {
                if !f.matches(ctx) {
                    result = false;
                    break;
                }
            }
        }

        if self.negated {
            !result
        } else {
            result
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        let mut negated = false;
        let mut rest = trimmed;

        if let Some(r) = rest.strip_prefix("not ") {
            negated = true;
            rest = r.trim();
        } else if let Some(r) = rest.strip_prefix("only ") {
            rest = r.trim();
        }

        let mut media_type = None;
        let mut features = Vec::new();

        for part in rest.split("and") {
            let p = part.trim();
            if p.starts_with('(') && p.ends_with(')') {
                if let Some(f) = MediaFeature::parse(p) {
                    features.push(f);
                }
            } else if let Some(mt) = MediaType::parse(p) {
                media_type = Some(mt);
            }
        }

        Some(Self {
            negated,
            media_type,
            features,
        })
    }
}

/// Consulta completa `@media` que pode conter múltiplas condições separadas por vírgula (OR).
#[derive(Debug, Clone, PartialEq)]
pub struct MediaQuery {
    pub conditions: Vec<MediaCondition>,
}

impl MediaQuery {
    pub fn matches(&self, ctx: &MediaContext) -> bool {
        if self.conditions.is_empty() {
            return true;
        }
        self.conditions.iter().any(|c| c.matches(ctx))
    }

    pub fn parse(s: &str) -> Self {
        let conditions: Vec<MediaCondition> = s
            .split(',')
            .filter_map(|part| MediaCondition::parse(part.trim()))
            .collect();
        Self { conditions }
    }
}

/// Regra `@media <query> { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaRule {
    pub query: MediaQuery,
    pub rules: Vec<CSSRule>,
}
