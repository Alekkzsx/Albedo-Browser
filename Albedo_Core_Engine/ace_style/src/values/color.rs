//! # Cores CSS e Palavras-chave Especiais
//!
//! Integração com `ace_core::math::Color` e resolução de `currentColor`.

pub use ace_core::math::Color;

/// Representação de uma cor no CSSOM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CssColor {
    /// Cor absoluta resolvida (RGB, HSL, Hex, Oklab, etc.)
    Absolute(Color),
    /// Palavra-chave `currentColor` (herda a cor do texto do elemento)
    CurrentColor,
}

impl Default for CssColor {
    fn default() -> Self {
        Self::Absolute(Color::BLACK)
    }
}

impl CssColor {
    /// Resolve a cor para uma instância concreta de `ace_core::math::Color`.
    #[inline]
    pub fn resolve(&self, current_color: Color) -> Color {
        match self {
            Self::Absolute(c) => *c,
            Self::CurrentColor => current_color,
        }
    }

    /// Analisa uma cor a partir de uma string CSS.
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.eq_ignore_ascii_case("currentcolor") {
            return Some(Self::CurrentColor);
        }

        if trimmed.eq_ignore_ascii_case("transparent") {
            return Some(Self::Absolute(Color::from_rgba(0, 0, 0, 0)));
        }

        // Tenta fazer o parse via Color::parse do ace_core
        if let Ok(c) = Color::parse(trimmed) {
            return Some(Self::Absolute(c));
        }

        None
    }
}
