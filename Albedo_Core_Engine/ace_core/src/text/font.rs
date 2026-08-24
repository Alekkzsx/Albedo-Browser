//! # Primitivas de Tipografia e Fontes (CSS Fonts Module Level 4)
//!
//! Tipos e enums fortemente tipados para seleção, correspondência e resolução de fontes no CSSOM.

use smol_str::SmolStr;
use std::fmt;

/// Famílias de fontes genéricas padronizadas pelo W3C (CSS Fonts Module Level 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GenericFontFamily {
    /// Fontes com serifa (ex: Times New Roman, Georgia).
    Serif,
    /// Fontes sem serifa (ex: Arial, Helvetica, Inter).
    #[default]
    SansSerif,
    /// Fontes de largura fixa / monoespaçadas (ex: Courier, Consolas, Fira Code).
    Monospace,
    /// Fontes cursivas / manuscritas.
    Cursive,
    /// Fontes decorativas / artísticas.
    Fantasy,
    /// Fonte padrão da interface do sistema operacional (ex: San Francisco, Segoe UI, Roboto).
    SystemUi,
    /// Fonte com serifa do sistema operacional.
    UiSerif,
    /// Fonte sem serifa do sistema operacional.
    UiSansSerif,
    /// Fonte monoespaçada do sistema operacional.
    UiMonospace,
    /// Fonte com cantos arredondados do sistema operacional.
    UiRounded,
    /// Fonte especializada para renderização de emojis.
    Emoji,
    /// Fonte para fórmulas matemáticas (OpenType MATH table).
    Math,
    /// Estilo chinês tradicional Fangsong.
    Fangsong,
}

impl GenericFontFamily {
    /// Converte uma palavra-chave CSS no `GenericFontFamily` correspondente.
    pub fn from_css_keyword(keyword: &str) -> Option<Self> {
        match keyword.trim().to_ascii_lowercase().as_str() {
            "serif" => Some(Self::Serif),
            "sans-serif" => Some(Self::SansSerif),
            "monospace" => Some(Self::Monospace),
            "cursive" => Some(Self::Cursive),
            "fantasy" => Some(Self::Fantasy),
            "system-ui" => Some(Self::SystemUi),
            "ui-serif" => Some(Self::UiSerif),
            "ui-sans-serif" => Some(Self::UiSansSerif),
            "ui-monospace" => Some(Self::UiMonospace),
            "ui-rounded" => Some(Self::UiRounded),
            "emoji" => Some(Self::Emoji),
            "math" => Some(Self::Math),
            "fangsong" => Some(Self::Fangsong),
            _ => None,
        }
    }

    /// Retorna a palavra-chave CSS canônica correspondente.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Serif => "serif",
            Self::SansSerif => "sans-serif",
            Self::Monospace => "monospace",
            Self::Cursive => "cursive",
            Self::Fantasy => "fantasy",
            Self::SystemUi => "system-ui",
            Self::UiSerif => "ui-serif",
            Self::UiSansSerif => "ui-sans-serif",
            Self::UiMonospace => "ui-monospace",
            Self::UiRounded => "ui-rounded",
            Self::Emoji => "emoji",
            Self::Math => "math",
            Self::Fangsong => "fangsong",
        }
    }
}

/// Peso de uma fonte tipográfica ($1 \dots 1000$).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const EXTRA_LIGHT: Self = Self(200);
    pub const LIGHT: Self = Self(300);
    pub const NORMAL: Self = Self(400);
    pub const REGULAR: Self = Self(400);
    pub const MEDIUM: Self = Self(500);
    pub const SEMI_BOLD: Self = Self(600);
    pub const BOLD: Self = Self(700);
    pub const EXTRA_BOLD: Self = Self(800);
    pub const BLACK: Self = Self(900);

    /// Cria um novo `FontWeight` garantindo saturação dentro dos limites $[1, 1000]$.
    #[inline]
    pub const fn new(weight: u16) -> Self {
        let clamped = if weight < 1 {
            1
        } else if weight > 1000 {
            1000
        } else {
            weight
        };
        Self(clamped)
    }

    /// Analisa uma propriedade CSS `font-weight` (valor numérico ou palavra-chave).
    pub fn from_css_value(value: &str) -> Option<Self> {
        let trimmed = value.trim().to_ascii_lowercase();
        match trimmed.as_str() {
            "normal" => Some(Self::NORMAL),
            "bold" => Some(Self::BOLD),
            _ => trimmed.parse::<u16>().ok().map(Self::new),
        }
    }

    /// Retorna o valor numérico do peso.
    #[inline]
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Retorna `true` se o peso for considerado negrito ($\ge 600$).
    #[inline]
    pub const fn is_bold(self) -> bool {
        self.0 >= 600
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Estilo de inclinação de uma fonte tipográfica (`font-style`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FontStyle {
    /// Fonte romana normal vertical.
    #[default]
    Normal,
    /// Fonte itálica desenhada manualmente.
    Italic,
    /// Inclinação oblíqua com ângulo opcional em graus (padrão: 14°).
    Oblique(Option<f32>),
}

/// Extensão e condensação de glifos (`font-stretch`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    #[default]
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

impl FontStretch {
    /// Analisa uma palavra-chave CSS `font-stretch`.
    pub fn from_css_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ultra-condensed" => Some(Self::UltraCondensed),
            "extra-condensed" => Some(Self::ExtraCondensed),
            "condensed" => Some(Self::Condensed),
            "semi-condensed" => Some(Self::SemiCondensed),
            "normal" => Some(Self::Normal),
            "semi-expanded" => Some(Self::SemiExpanded),
            "expanded" => Some(Self::Expanded),
            "extra-expanded" => Some(Self::ExtraExpanded),
            "ultra-expanded" => Some(Self::UltraExpanded),
            _ => None,
        }
    }
}

/// Descritor agregado de seleção de fontes no CSSOM.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FontDescriptor {
    /// Nome da família de fontes requerida (ex: `"Inter"`, `"Helvetica"`).
    pub family: SmolStr,
    /// Família genérica de fallback se aplicável.
    pub generic_family: Option<GenericFontFamily>,
    /// Peso da fonte.
    pub weight: FontWeight,
    /// Estilo de inclinação.
    pub style: FontStyle,
    /// Nível de expansão dos caracteres.
    pub stretch: FontStretch,
    /// Tamanho em pixels CSS computados.
    pub size_px: f32,
}

impl FontDescriptor {
    /// Cria um novo descritor de fonte padrão com tamanho explícito em pixels.
    pub fn new(family: impl Into<SmolStr>, size_px: f32) -> Self {
        let fam = family.into();
        let generic = GenericFontFamily::from_css_keyword(&fam);
        Self {
            family: fam,
            generic_family: generic,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
            stretch: FontStretch::Normal,
            size_px,
        }
    }
}

impl fmt::Display for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
