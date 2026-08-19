//! # Representação de Cores e Algoritmos de Blending CSS
//!
//! Implementação completa de cores para o CSSOM e Pipeline de Pintura GPU,
//! incluindo parsing de especificações CSS3/CSS4 e composição alfa Porter-Duff.

#![allow(clippy::excessive_precision)]

use crate::error::AceError;
use std::fmt;

/// Representa uma cor no espaço de cores sRGB com canal alfa de 8 bits por componente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    /// Componente Vermelho (0-255).
    pub r: u8,
    /// Componente Verde (0-255).
    pub g: u8,
    /// Componente Azul (0-255).
    pub b: u8,
    /// Canal Alfa de transparência (0 = Totalmente Transparente, 255 = Totalmente Opaco).
    pub a: u8,
}

/// Espaços de cores suportados conforme as especificações CSS Color Module Level 4 e Level 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    /// sRGB padrão da Web.
    Srgb,
    /// sRGB linear (sem correção gamma).
    SrgbLinear,
    /// Display-P3 (Wide-Gamut D65).
    DisplayP3,
    /// Adobe RGB (1998).
    A98Rgb,
    /// ProPhoto RGB.
    ProPhotoRgb,
    /// ITU-R BT.2020.
    Rec2020,
    /// Espaço perceptual cartesiano Oklab.
    Oklab,
    /// Espaço perceptual polar Oklch.
    Oklch,
    /// CIE L*a*b* (D50).
    Lab,
    /// CIE L*C*h (D50).
    Lch,
    /// HSL (Hue, Saturation, Lightness).
    Hsl,
    /// HWB (Hue, Whiteness, Blackness).
    Hwb,
}

impl ColorSpace {
    /// Analisa o nome de um espaço de cor CSS.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "srgb" => Some(Self::Srgb),
            "srgb-linear" => Some(Self::SrgbLinear),
            "display-p3" => Some(Self::DisplayP3),
            "a98-rgb" => Some(Self::A98Rgb),
            "prophoto-rgb" => Some(Self::ProPhotoRgb),
            "rec2020" => Some(Self::Rec2020),
            "oklab" => Some(Self::Oklab),
            "oklch" => Some(Self::Oklch),
            "lab" => Some(Self::Lab),
            "lch" => Some(Self::Lch),
            "hsl" => Some(Self::Hsl),
            "hwb" => Some(Self::Hwb),
            _ => None,
        }
    }
}

/// Métodos de interpolação de matiz (ângulo polar) conforme CSS Color Module Level 4 §12.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HueInterpolation {
    /// Arco mais curto no círculo trigonométrico (padrão do CSS Color 4).
    #[default]
    Shorter,
    /// Arco mais longo no círculo trigonométrico.
    Longer,
    /// Sentido anti-horário (ângulos crescentes).
    Increasing,
    /// Sentido horário (ângulos decrescentes).
    Decreasing,
}

impl HueInterpolation {
    /// Analisa o nome de um método de interpolação de matiz CSS.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "shorter" | "shorter hue" => Some(Self::Shorter),
            "longer" | "longer hue" => Some(Self::Longer),
            "increasing" | "increasing hue" => Some(Self::Increasing),
            "decreasing" | "decreasing hue" => Some(Self::Decreasing),
            _ => None,
        }
    }
}

impl Color {
    pub const TRANSPARENT: Self = Self::from_rgba(0, 0, 0, 0);
    pub const BLACK: Self = Self::from_rgb(0, 0, 0);
    pub const WHITE: Self = Self::from_rgb(255, 255, 255);
    pub const RED: Self = Self::from_rgb(255, 0, 0);
    pub const GREEN: Self = Self::from_rgb(0, 128, 0);
    pub const LIME: Self = Self::from_rgb(0, 255, 0);
    pub const BLUE: Self = Self::from_rgb(0, 0, 255);
    pub const YELLOW: Self = Self::from_rgb(255, 255, 0);
    pub const CYAN: Self = Self::from_rgb(0, 255, 255);
    pub const MAGENTA: Self = Self::from_rgb(255, 0, 255);
    pub const GRAY: Self = Self::from_rgb(128, 128, 128);
    pub const SILVER: Self = Self::from_rgb(192, 192, 192);

    /// Cria uma cor a partir de valores inteiros RGBA (0-255).
    #[inline]
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Cria uma cor opaca a partir de valores inteiros RGB (0-255).
    #[inline]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Cria uma cor a partir de floats normalizados (0.0 ..= 1.0).
    #[inline]
    pub fn from_rgba_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: (r.clamp(0.0, 1.0) * 255.0).round() as u8,
            g: (g.clamp(0.0, 1.0) * 255.0).round() as u8,
            b: (b.clamp(0.0, 1.0) * 255.0).round() as u8,
            a: (a.clamp(0.0, 1.0) * 255.0).round() as u8,
        }
    }

    /// Retorna os componentes normalizados como floats (0.0 ..= 1.0).
    #[inline]
    pub fn to_rgba_f32(self) -> (f32, f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }

    /// Retorna os componentes com canal alfa pré-multiplicado (necessário para Shaders GPU
    /// para evitar halos escuros em texturas semi-transparentes).
    #[inline]
    pub fn to_premultiplied_f32(self) -> (f32, f32, f32, f32) {
        let (r, g, b, a) = self.to_rgba_f32();
        (r * a, g * a, b * a, a)
    }

    /// Cria uma cor a partir do modelo HSLA:
    /// * `h`: Matiz (Hue em graus 0.0 ..= 360.0).
    /// * `s`: Saturação (0.0 ..= 1.0).
    /// * `l`: Luminosidade (0.0 ..= 1.0).
    /// * `a`: Alfa (0.0 ..= 1.0).
    pub fn from_hsla(h: f32, s: f32, l: f32, a: f32) -> Self {
        let s = s.clamp(0.0, 1.0);
        let l = l.clamp(0.0, 1.0);
        let a = a.clamp(0.0, 1.0);

        let h_norm = (h % 360.0 + 360.0) % 360.0;
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h_norm / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r_prime, g_prime, b_prime) = match (h_norm / 60.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Self::from_rgba_f32(r_prime + m, g_prime + m, b_prime + m, a)
    }

    /// Retorna a representação HSLA desta cor `(h, s, l, a)`:
    /// * `h`: Matiz em graus (0.0 ..= 360.0).
    /// * `s`: Saturação (0.0 ..= 1.0).
    /// * `l`: Luminosidade (0.0 ..= 1.0).
    /// * `a`: Alfa (0.0 ..= 1.0).
    pub fn to_hsla(self) -> (f32, f32, f32, f32) {
        let (r, g, b, a) = self.to_rgba_f32();
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let l = (max + min) / 2.0;

        if delta < 1e-5 {
            return (0.0, 0.0, l, a);
        }

        let s = if l > 0.5 {
            delta / (2.0 - max - min)
        } else {
            delta / (max + min)
        };

        let mut h = if (max - r).abs() < 1e-5 {
            ((g - b) / delta + if g < b { 6.0 } else { 0.0 }) * 60.0
        } else if (max - g).abs() < 1e-5 {
            ((b - r) / delta + 2.0) * 60.0
        } else {
            ((r - g) / delta + 4.0) * 60.0
        };

        if h < 0.0 {
            h += 360.0;
        } else if h >= 360.0 {
            h -= 360.0;
        }

        (h, s, l, a)
    }

    /// Parser de strings hexadecimais (`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`).
    pub fn from_hex(hex: &str) -> Result<Self, AceError> {
        let s = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
        let len = s.len();

        match len {
            3 => {
                // #RGB -> #RRGGBB
                let r = u8::from_str_radix(&s[0..1], 16).map_err(|_| invalid_color(hex))?;
                let g = u8::from_str_radix(&s[1..2], 16).map_err(|_| invalid_color(hex))?;
                let b = u8::from_str_radix(&s[2..3], 16).map_err(|_| invalid_color(hex))?;
                Ok(Self::from_rgba(r * 17, g * 17, b * 17, 255))
            }
            4 => {
                // #RGBA -> #RRGGBBAA
                let r = u8::from_str_radix(&s[0..1], 16).map_err(|_| invalid_color(hex))?;
                let g = u8::from_str_radix(&s[1..2], 16).map_err(|_| invalid_color(hex))?;
                let b = u8::from_str_radix(&s[2..3], 16).map_err(|_| invalid_color(hex))?;
                let a = u8::from_str_radix(&s[3..4], 16).map_err(|_| invalid_color(hex))?;
                Ok(Self::from_rgba(r * 17, g * 17, b * 17, a * 17))
            }
            6 => {
                // #RRGGBB
                let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| invalid_color(hex))?;
                let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| invalid_color(hex))?;
                let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| invalid_color(hex))?;
                Ok(Self::from_rgba(r, g, b, 255))
            }
            8 => {
                // #RRGGBBAA
                let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| invalid_color(hex))?;
                let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| invalid_color(hex))?;
                let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| invalid_color(hex))?;
                let a = u8::from_str_radix(&s[6..8], 16).map_err(|_| invalid_color(hex))?;
                Ok(Self::from_rgba(r, g, b, a))
            }
            _ => Err(invalid_color(hex)),
        }
    }

    /// Parser universal de especificações de cores CSS (hex, `rgb()`, `rgba()`, `hsl()`, `hsla()`, `hwb()`, `oklab()`, `oklch()`, `color()`, `color-mix()`, `light-dark()` e cores nomeadas).
    pub fn parse_css(input: &str) -> Result<Self, AceError> {
        let s = input.trim();
        let s_lower = s.to_ascii_lowercase();

        if s_lower.starts_with('#') {
            return Self::from_hex(&s_lower);
        }

        if let Some(color) = named_color(&s_lower) {
            return Ok(color);
        }

        if s_lower.starts_with("rgb(") || s_lower.starts_with("rgba(") {
            return parse_rgb_functional(&s_lower);
        }

        if s_lower.starts_with("hsl(") || s_lower.starts_with("hsla(") {
            return parse_hsl_functional(&s_lower);
        }

        if s_lower.starts_with("hwb(") {
            return parse_hwb_functional(&s_lower);
        }

        if s_lower.starts_with("oklab(") {
            return parse_oklab_functional(&s_lower);
        }

        if s_lower.starts_with("oklch(") {
            return parse_oklch_functional(&s_lower);
        }

        if s_lower.starts_with("lab(") {
            return parse_lab_functional(&s_lower);
        }

        if s_lower.starts_with("lch(") {
            return parse_lch_functional(&s_lower);
        }

        if s_lower.starts_with("color(") {
            return parse_color_functional(&s_lower);
        }

        if s_lower.starts_with("color-mix(") {
            return parse_color_mix_functional(&s_lower);
        }

        if s_lower.starts_with("light-dark(") {
            return parse_light_dark_functional(&s_lower);
        }

        Err(invalid_color(input))
    }

    /// Alias conveniente para `parse_css`.
    #[inline]
    pub fn parse(input: &str) -> Result<Self, AceError> {
        Self::parse_css(input)
    }

    /// Converte esta cor sRGB para o espaço de cor perceptual Oklab.
    #[inline]
    pub fn to_oklab(self) -> super::oklab::Oklab {
        let (r, g, b, a) = self.to_rgba_f32();
        super::oklab::Oklab::from_srgb(r, g, b, a)
    }

    /// Cria uma cor a partir de uma representação Oklab.
    #[inline]
    pub fn from_oklab(oklab: super::oklab::Oklab) -> Self {
        let (r, g, b, a) = oklab.to_srgb();
        Self::from_rgba_f32(r, g, b, a)
    }

    /// Converte esta cor sRGB para o espaço de cor cilíndrico Oklch.
    #[inline]
    pub fn to_oklch(self) -> super::oklab::Oklch {
        let (r, g, b, a) = self.to_rgba_f32();
        super::oklab::Oklch::from_srgb(r, g, b, a)
    }

    /// Cria uma cor a partir de uma representação Oklch.
    #[inline]
    pub fn from_oklch(oklch: super::oklab::Oklch) -> Self {
        let (r, g, b, a) = oklch.to_srgb();
        Self::from_rgba_f32(r, g, b, a)
    }

    /// Converte esta cor sRGB para o espaço CIE L*a*b* (D50) via Adaptação Cromática Bradford.
    #[inline]
    pub fn to_lab(self) -> (f32, f32, f32, f32) {
        let (r, g, b, a) = self.to_rgba_f32();
        let r_lin = srgb_to_linear(r);
        let g_lin = srgb_to_linear(g);
        let b_lin = srgb_to_linear(b);
        let (x, y, z) = linear_srgb_to_xyz_d50(r_lin, g_lin, b_lin);
        let (l, a_val, b_val) = xyz_d50_to_lab(x, y, z);
        (l, a_val, b_val, a)
    }

    /// Cria uma cor a partir do espaço CIE L*a*b* (D50) via Adaptação Cromática Bradford inversa.
    #[inline]
    pub fn from_lab(l: f32, a: f32, b: f32, alpha: f32) -> Self {
        let (x, y, z) = lab_to_xyz_d50(l, a, b);
        let (r_lin, g_lin, b_lin) = xyz_d50_to_linear_srgb(x, y, z);
        let r = linear_to_srgb(r_lin).clamp(0.0, 1.0);
        let g = linear_to_srgb(g_lin).clamp(0.0, 1.0);
        let b = linear_to_srgb(b_lin).clamp(0.0, 1.0);
        Self::from_rgba_f32(r, g, b, alpha.clamp(0.0, 1.0))
    }

    /// Converte esta cor sRGB para o espaço CIE L*C*h (D50).
    #[inline]
    pub fn to_lch(self) -> (f32, f32, f32, f32) {
        let (l, a, b, alpha) = self.to_lab();
        let (l, c, h) = lab_to_lch(l, a, b);
        (l, c, h, alpha)
    }

    /// Cria uma cor a partir do espaço CIE L*C*h (D50).
    #[inline]
    pub fn from_lch(l: f32, c: f32, h: f32, alpha: f32) -> Self {
        let (l, a, b) = lch_to_lab(l, c, h);
        Self::from_lab(l, a, b, alpha)
    }

    /// Calcula a diferença de cor perceptual CIE Delta E (CIE76) entre duas cores no espaço Lab.
    #[inline]
    pub fn delta_e_76(self, other: Self) -> f32 {
        let (l1, a1, b1, _) = self.to_lab();
        let (l2, a2, b2, _) = other.to_lab();
        let dl = l1 - l2;
        let da = a1 - a2;
        let db = b1 - b2;
        (dl * dl + da * da + db * db).sqrt()
    }

    /// Composição alfa padrão Porter-Duff (`source-over`): mistura `self` (fonte) sobre `dst` (fundo).
    #[inline]
    pub fn blend_source_over(self, dst: Self) -> Self {
        let (src_r, src_g, src_b, src_a) = self.to_rgba_f32();
        let (dst_r, dst_g, dst_b, dst_a) = dst.to_rgba_f32();

        let out_a = src_a + dst_a * (1.0 - src_a);
        if out_a <= 0.0 {
            return Self::TRANSPARENT;
        }

        let out_r = (src_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a;
        let out_g = (src_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a;
        let out_b = (src_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a;

        Self::from_rgba_f32(out_r, out_g, out_b, out_a)
    }

    /// Interpolação linear sRGB padrão entre duas cores (`t` entre 0.0 e 1.0).
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let (r1, g1, b1, a1) = self.to_rgba_f32();
        let (r2, g2, b2, a2) = other.to_rgba_f32();

        Self::from_rgba_f32(
            r1 + (r2 - r1) * t,
            g1 + (g2 - g1) * t,
            b1 + (b2 - b1) * t,
            a1 + (a2 - a1) * t,
        )
    }

    /// Interpolação linear no espaço cartesiano CIE Lab com premultiplicação alfa.
    pub fn lerp_lab(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let (l1, a1, b1, alpha1) = self.to_lab();
        let (l2, a2, b2, alpha2) = other.to_lab();

        let alpha1 = alpha1.clamp(0.0, 1.0);
        let alpha2 = alpha2.clamp(0.0, 1.0);
        let out_a = alpha1 * (1.0 - t) + alpha2 * t;
        if out_a <= 1e-6 {
            return Self::TRANSPARENT;
        }

        let l = (l1 * alpha1 * (1.0 - t) + l2 * alpha2 * t) / out_a;
        let a = (a1 * alpha1 * (1.0 - t) + a2 * alpha2 * t) / out_a;
        let b = (b1 * alpha1 * (1.0 - t) + b2 * alpha2 * t) / out_a;

        Self::from_lab(l, a, b, out_a)
    }

    /// Interpolação perceptual uniforme no espaço Oklab com premultiplicação alfa.
    pub fn lerp_oklab(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let lab1 = self.to_oklab();
        let lab2 = other.to_oklab();

        let a1 = lab1.alpha.clamp(0.0, 1.0);
        let a2 = lab2.alpha.clamp(0.0, 1.0);
        let out_a = a1 * (1.0 - t) + a2 * t;
        if out_a <= 1e-6 {
            return Self::TRANSPARENT;
        }

        let l = (lab1.l * a1 * (1.0 - t) + lab2.l * a2 * t) / out_a;
        let a = (lab1.a * a1 * (1.0 - t) + lab2.a * a2 * t) / out_a;
        let b = (lab1.b * a1 * (1.0 - t) + lab2.b * a2 * t) / out_a;

        Self::from_oklab(super::oklab::Oklab::new(l, a, b, out_a))
    }

    /// Interpolação perceptual no espaço polar Oklch (método padrão shorter hue).
    #[inline]
    pub fn lerp_oklch(self, other: Self, t: f32) -> Self {
        self.interpolate_oklch(other, t, HueInterpolation::Shorter)
    }

    /// Interpolação perceptual no espaço polar Oklch com suporte aos 4 métodos de matiz, powerless e premultiplicação alfa.
    pub fn interpolate_oklch(self, other: Self, t: f32, method: HueInterpolation) -> Self {
        let t = t.clamp(0.0, 1.0);
        let lch1 = self.to_oklch();
        let lch2 = other.to_oklch();

        let a1 = lch1.alpha.clamp(0.0, 1.0);
        let a2 = lch2.alpha.clamp(0.0, 1.0);
        let out_a = a1 * (1.0 - t) + a2 * t;
        if out_a <= 1e-6 {
            return Self::TRANSPARENT;
        }

        let l = (lch1.l * a1 * (1.0 - t) + lch2.l * a2 * t) / out_a;
        let c = (lch1.c * a1 * (1.0 - t) + lch2.c * a2 * t) / out_a;

        let h1_powerless = lch1.c < 1e-4;
        let h2_powerless = lch2.c < 1e-4;
        let (h1, h2) = match (h1_powerless, h2_powerless) {
            (true, false) => (lch2.h, lch2.h),
            (false, true) => (lch1.h, lch1.h),
            (true, true) => (0.0, 0.0),
            (false, false) => (lch1.h, lch2.h),
        };

        let h = interpolate_hue(h1, h2, t, method);
        Self::from_oklch(crate::math::oklab::Oklch::new(l, c, h, out_a))
    }

    /// Interpolação no espaço polar CIE Lch com suporte aos 4 métodos de matiz, powerless e premultiplicação alfa.
    pub fn interpolate_lch(self, other: Self, t: f32, method: HueInterpolation) -> Self {
        let t = t.clamp(0.0, 1.0);
        let (l1, c1, h1_raw, a1) = self.to_lch();
        let (l2, c2, h2_raw, a2) = other.to_lch();

        let a1 = a1.clamp(0.0, 1.0);
        let a2 = a2.clamp(0.0, 1.0);
        let out_a = a1 * (1.0 - t) + a2 * t;
        if out_a <= 1e-6 {
            return Self::TRANSPARENT;
        }

        let l = (l1 * a1 * (1.0 - t) + l2 * a2 * t) / out_a;
        let c = (c1 * a1 * (1.0 - t) + c2 * a2 * t) / out_a;

        let h1_powerless = c1 < 1e-4;
        let h2_powerless = c2 < 1e-4;
        let (h1, h2) = match (h1_powerless, h2_powerless) {
            (true, false) => (h2_raw, h2_raw),
            (false, true) => (h1_raw, h1_raw),
            (true, true) => (0.0, 0.0),
            (false, false) => (h1_raw, h2_raw),
        };

        let h = interpolate_hue(h1, h2, t, method);
        Self::from_lch(l, c, h, out_a)
    }

    /// Interpolação no espaço polar HSL com suporte aos 4 métodos de matiz, powerless e premultiplicação alfa.
    pub fn interpolate_hsl(self, other: Self, t: f32, method: HueInterpolation) -> Self {
        let t = t.clamp(0.0, 1.0);
        let (h1_raw, s1, l1, a1) = self.to_hsla();
        let (h2_raw, s2, l2, a2) = other.to_hsla();

        let a1 = a1.clamp(0.0, 1.0);
        let a2 = a2.clamp(0.0, 1.0);
        let out_a = a1 * (1.0 - t) + a2 * t;
        if out_a <= 1e-6 {
            return Self::TRANSPARENT;
        }

        let s = (s1 * a1 * (1.0 - t) + s2 * a2 * t) / out_a;
        let l = (l1 * a1 * (1.0 - t) + l2 * a2 * t) / out_a;

        let h1_powerless = s1 < 1e-4;
        let h2_powerless = s2 < 1e-4;
        let (h1, h2) = match (h1_powerless, h2_powerless) {
            (true, false) => (h2_raw, h2_raw),
            (false, true) => (h1_raw, h1_raw),
            (true, true) => (0.0, 0.0),
            (false, false) => (h1_raw, h2_raw),
        };

        let h = interpolate_hue(h1, h2, t, method);
        Self::from_hsla(h, s, l, out_a)
    }

    /// Interpolação polar genérica (Oklch, Lch, Hsl) no espaço e método especificados.
    #[inline]
    pub fn interpolate_polar(self, other: Self, t: f32, space: ColorSpace, method: HueInterpolation) -> Self {
        match space {
            ColorSpace::Oklch => self.interpolate_oklch(other, t, method),
            ColorSpace::Lch => self.interpolate_lch(other, t, method),
            ColorSpace::Hsl => self.interpolate_hsl(other, t, method),
            _ => self.interpolate_oklch(other, t, method),
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
        } else {
            write!(
                f,
                "rgba({}, {}, {}, {:.3})",
                self.r,
                self.g,
                self.b,
                self.a as f32 / 255.0
            )
        }
    }
}

fn invalid_color(s: &str) -> AceError {
    AceError::style(format!("Valor de cor CSS inválido: '{}'", s))
}

fn parse_rgb_functional(s: &str) -> Result<Color, AceError> {
    let inner = s
        .trim_start_matches("rgba(")
        .trim_start_matches("rgb(")
        .trim_end_matches(')')
        .trim();

    // Suporta delimitador por vírgula ou espaço/barra (CSS Colors Level 4)
    let parts: Vec<&str> = if inner.contains(',') {
        inner.split(',').map(str::trim).collect()
    } else {
        inner.split_whitespace().filter(|p| *p != "/").collect()
    };

    if parts.len() < 3 || parts.len() > 4 {
        return Err(invalid_color(s));
    }

    let r = parse_component(parts[0], 255.0)?;
    let g = parse_component(parts[1], 255.0)?;
    let b = parse_component(parts[2], 255.0)?;
    let a = if parts.len() == 4 {
        parse_alpha(parts[3])?
    } else {
        1.0
    };

    Ok(Color::from_rgba_f32(r / 255.0, g / 255.0, b / 255.0, a))
}

fn parse_hsl_functional(s: &str) -> Result<Color, AceError> {
    let inner = s
        .trim_start_matches("hsla(")
        .trim_start_matches("hsl(")
        .trim_end_matches(')')
        .trim();

    let parts: Vec<&str> = if inner.contains(',') {
        inner.split(',').map(str::trim).collect()
    } else {
        inner.split_whitespace().filter(|p| *p != "/").collect()
    };

    if parts.len() < 3 || parts.len() > 4 {
        return Err(invalid_color(s));
    }

    let h: f32 = parts[0]
        .trim_end_matches("deg")
        .parse()
        .map_err(|_| invalid_color(s))?;
    let s_val = parse_percentage(parts[1])?;
    let l_val = parse_percentage(parts[2])?;
    let a = if parts.len() == 4 {
        parse_alpha(parts[3])?
    } else {
        1.0
    };

    Ok(Color::from_hsla(h, s_val, l_val, a))
}

fn parse_oklab_functional(s: &str) -> Result<Color, AceError> {
    let inner = s.trim_start_matches("oklab(").trim_end_matches(')').trim();

    let parts: Vec<&str> = if inner.contains('/') {
        let mut split = inner.split('/');
        let main = split.next().unwrap_or("");
        let alpha = split.next().unwrap_or("");
        let mut p: Vec<&str> = main.split_whitespace().collect();
        if !alpha.trim().is_empty() {
            p.push(alpha.trim());
        }
        p
    } else {
        inner.split_whitespace().collect()
    };

    if parts.len() < 3 || parts.len() > 4 {
        return Err(invalid_color(s));
    }

    let l = parse_percentage(parts[0])?;
    let a: f32 = parts[1].parse().map_err(|_| invalid_color(s))?;
    let b: f32 = parts[2].parse().map_err(|_| invalid_color(s))?;
    let alpha = if parts.len() == 4 {
        parse_alpha(parts[3])?
    } else {
        1.0
    };

    Ok(Color::from_oklab(super::oklab::Oklab::new(l, a, b, alpha)))
}

fn parse_oklch_functional(s: &str) -> Result<Color, AceError> {
    let inner = s.trim_start_matches("oklch(").trim_end_matches(')').trim();

    let parts: Vec<&str> = if inner.contains('/') {
        let mut split = inner.split('/');
        let main = split.next().unwrap_or("");
        let alpha = split.next().unwrap_or("");
        let mut p: Vec<&str> = main.split_whitespace().collect();
        if !alpha.trim().is_empty() {
            p.push(alpha.trim());
        }
        p
    } else if inner.contains(',') {
        inner.split(',').map(str::trim).collect()
    } else {
        inner.split_whitespace().collect()
    };

    if parts.len() < 3 || parts.len() > 4 {
        return Err(invalid_color(s));
    }

    let l = parse_percentage(parts[0])?;
    let c: f32 = parts[1].parse().map_err(|_| invalid_color(s))?;
    let h = parse_hue_angle(parts[2])?;
    let alpha = if parts.len() == 4 {
        parse_alpha(parts[3])?
    } else {
        1.0
    };

    Ok(Color::from_oklch(super::oklab::Oklch::new(l, c, h, alpha)))
}

fn parse_lab_functional(s: &str) -> Result<Color, AceError> {
    let inner = s.trim_start_matches("lab(").trim_end_matches(')').trim();

    let parts: Vec<&str> = if inner.contains('/') {
        let mut split = inner.split('/');
        let main = split.next().unwrap_or("");
        let alpha = split.next().unwrap_or("");
        let mut p: Vec<&str> = main.split_whitespace().collect();
        if !alpha.trim().is_empty() {
            p.push(alpha.trim());
        }
        p
    } else if inner.contains(',') {
        inner.split(',').map(str::trim).collect()
    } else {
        inner.split_whitespace().collect()
    };

    if parts.len() < 3 || parts.len() > 4 {
        return Err(invalid_color(s));
    }

    let l = parse_lab_percentage(parts[0], 100.0)?;
    let a = parse_lab_percentage(parts[1], 125.0)?;
    let b = parse_lab_percentage(parts[2], 125.0)?;
    let alpha = if parts.len() == 4 {
        parse_alpha(parts[3])?
    } else {
        1.0
    };

    Ok(Color::from_lab(l, a, b, alpha))
}

fn parse_lch_functional(s: &str) -> Result<Color, AceError> {
    let inner = s.trim_start_matches("lch(").trim_end_matches(')').trim();

    let parts: Vec<&str> = if inner.contains('/') {
        let mut split = inner.split('/');
        let main = split.next().unwrap_or("");
        let alpha = split.next().unwrap_or("");
        let mut p: Vec<&str> = main.split_whitespace().collect();
        if !alpha.trim().is_empty() {
            p.push(alpha.trim());
        }
        p
    } else if inner.contains(',') {
        inner.split(',').map(str::trim).collect()
    } else {
        inner.split_whitespace().collect()
    };

    if parts.len() < 3 || parts.len() > 4 {
        return Err(invalid_color(s));
    }

    let l = parse_lab_percentage(parts[0], 100.0)?;
    let c = parse_lab_percentage(parts[1], 150.0)?;
    let h = parse_hue_angle(parts[2])?;
    let alpha = if parts.len() == 4 {
        parse_alpha(parts[3])?
    } else {
        1.0
    };

    Ok(Color::from_lch(l, c, h, alpha))
}

fn parse_hwb_functional(s: &str) -> Result<Color, AceError> {
    let inner = s.trim_start_matches("hwb(").trim_end_matches(')').trim();

    let parts: Vec<&str> = if inner.contains('/') {
        let mut split = inner.split('/');
        let main = split.next().unwrap_or("");
        let alpha = split.next().unwrap_or("");
        let mut p: Vec<&str> = main.split_whitespace().collect();
        if !alpha.trim().is_empty() {
            p.push(alpha.trim());
        }
        p
    } else if inner.contains(',') {
        inner.split(',').map(str::trim).collect()
    } else {
        inner.split_whitespace().collect()
    };

    if parts.len() < 3 || parts.len() > 4 {
        return Err(invalid_color(s));
    }

    let h = parse_hue_angle(parts[0])?;
    let w = parse_percentage(parts[1])?;
    let b = parse_percentage(parts[2])?;
    let a = if parts.len() == 4 {
        parse_alpha(parts[3])?
    } else {
        1.0
    };

    let (r, g, b_val, a_val) = hwb_to_srgb(h, w, b, a);
    Ok(Color::from_rgba_f32(r, g, b_val, a_val))
}

fn parse_color_functional(s: &str) -> Result<Color, AceError> {
    let inner = s.trim_start_matches("color(").trim_end_matches(')').trim();

    let (main, alpha_str) = if let Some((m, a)) = inner.split_once('/') {
        (m.trim(), Some(a.trim()))
    } else {
        (inner, None)
    };

    let parts: Vec<&str> = main.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(invalid_color(s));
    }

    let space_str = parts[0];
    let space = ColorSpace::parse(space_str).ok_or_else(|| invalid_color(s))?;

    let r = parse_percentage(parts[1])?;
    let g = parse_percentage(parts[2])?;
    let b = parse_percentage(parts[3])?;
    let a = if let Some(a_str) = alpha_str {
        parse_alpha(a_str)?
    } else {
        1.0
    };

    match space {
        ColorSpace::DisplayP3 => {
            let (sr, sg, sb, sa) = display_p3_to_srgb(r, g, b, a);
            Ok(Color::from_rgba_f32(sr, sg, sb, sa))
        }
        ColorSpace::Srgb => Ok(Color::from_rgba_f32(r, g, b, a)),
        ColorSpace::SrgbLinear => Ok(Color::from_rgba_f32(
            linear_to_srgb(r),
            linear_to_srgb(g),
            linear_to_srgb(b),
            a,
        )),
        _ => Ok(Color::from_rgba_f32(r, g, b, a)),
    }
}

fn parse_light_dark_functional(s: &str) -> Result<Color, AceError> {
    let inner = s
        .trim_start_matches("light-dark(")
        .trim_end_matches(')')
        .trim();
    let parts = split_top_level_commas(inner);
    if parts.len() != 2 {
        return Err(invalid_color(s));
    }
    // Avalia o ramo claro por padrão
    Color::parse_css(parts[0])
}

fn parse_color_mix_functional(s: &str) -> Result<Color, AceError> {
    let inner = s
        .trim_start_matches("color-mix(")
        .trim_end_matches(')')
        .trim();
    let parts = split_top_level_commas(inner);
    if parts.len() != 3 {
        return Err(invalid_color(s));
    }

    let in_part = parts[0].trim();
    let header = in_part
        .strip_prefix("in ")
        .ok_or_else(|| invalid_color(s))?
        .trim();

    let (space, hue_method) = parse_mix_space_and_hue(header).ok_or_else(|| invalid_color(s))?;

    let (c1, p1) = parse_color_with_percentage(parts[1])?;
    let (c2, p2) = parse_color_with_percentage(parts[2])?;

    mix_colors_with_hue(space, hue_method, c1, p1, c2, p2)
}

fn parse_mix_space_and_hue(s: &str) -> Option<(ColorSpace, HueInterpolation)> {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let space = ColorSpace::parse(tokens[0])?;
    let hue_method = if tokens.len() >= 2 {
        let hue_str = tokens[1..].join(" ");
        HueInterpolation::parse(&hue_str).unwrap_or(HueInterpolation::Shorter)
    } else {
        HueInterpolation::Shorter
    };
    Some((space, hue_method))
}

fn parse_color_with_percentage(s: &str) -> Result<(Color, Option<f32>), AceError> {
    let trimmed = s.trim();
    if let Some(last_space_idx) = trimmed.rfind(' ') {
        let (color_part, pct_part) = (
            trimmed[..last_space_idx].trim(),
            trimmed[last_space_idx + 1..].trim(),
        );
        if pct_part.ends_with('%') {
            if let Ok(pct) = parse_percentage(pct_part) {
                if let Ok(color) = Color::parse_css(color_part) {
                    return Ok((color, Some(pct)));
                }
            }
        }
    }
    let color = Color::parse_css(trimmed)?;
    Ok((color, None))
}

fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;

    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            ',' if depth == 0 => {
                parts.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        let remainder = s[start..].trim();
        if !remainder.is_empty() {
            parts.push(remainder);
        }
    }
    parts
}

/// Converte Display-P3 (R, G, B floats 0..1 com curva sRGB) para sRGB (R, G, B, A floats 0..1).
pub fn display_p3_to_srgb(r: f32, g: f32, b: f32, a: f32) -> (f32, f32, f32, f32) {
    let r_lin = srgb_to_linear(r);
    let g_lin = srgb_to_linear(g);
    let b_lin = srgb_to_linear(b);

    let r_srgb_lin = 1.22494018 * r_lin - 0.22474048 * g_lin;
    let g_srgb_lin = -0.04205696 * r_lin + 1.04205696 * g_lin;
    let b_srgb_lin = -0.01963756 * r_lin - 0.07863605 * g_lin + 1.09827360 * b_lin;

    (
        linear_to_srgb(r_srgb_lin).clamp(0.0, 1.0),
        linear_to_srgb(g_srgb_lin).clamp(0.0, 1.0),
        linear_to_srgb(b_srgb_lin).clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    )
}

/// Converte sRGB (R, G, B floats 0..1) para Display-P3 (R, G, B, A floats 0..1).
pub fn srgb_to_display_p3(r: f32, g: f32, b: f32, a: f32) -> (f32, f32, f32, f32) {
    let r_lin = srgb_to_linear(r);
    let g_lin = srgb_to_linear(g);
    let b_lin = srgb_to_linear(b);

    let r_p3_lin = 0.822462 * r_lin + 0.177538 * g_lin;
    let g_p3_lin = 0.033194 * r_lin + 0.966806 * g_lin;
    let b_p3_lin = 0.017083 * r_lin + 0.072397 * g_lin + 0.910520 * b_lin;

    (
        linear_to_srgb(r_p3_lin).clamp(0.0, 1.0),
        linear_to_srgb(g_p3_lin).clamp(0.0, 1.0),
        linear_to_srgb(b_p3_lin).clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    )
}

/// Converte HWB para sRGB.
pub fn hwb_to_srgb(h: f32, w: f32, b: f32, a: f32) -> (f32, f32, f32, f32) {
    let w = w.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);
    if w + b >= 1.0 {
        let gray = w / (w + b);
        return (gray, gray, gray, a.clamp(0.0, 1.0));
    }
    let (r, g, b_val, _) = Color::from_hsla(h, 1.0, 0.5, 1.0).to_rgba_f32();
    let factor = 1.0 - w - b;
    (
        (r * factor + w).clamp(0.0, 1.0),
        (g * factor + w).clamp(0.0, 1.0),
        (b_val * factor + w).clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    )
}

/// Resolve `color-mix()` no espaço de cor solicitado com suporte a interpolação polar de matiz e premultiplicação alfa.
pub fn mix_colors_with_hue(
    space: ColorSpace,
    hue_method: HueInterpolation,
    c1: Color,
    p1: Option<f32>,
    c2: Color,
    p2: Option<f32>,
) -> Result<Color, AceError> {
    let (w1, w2, alpha_mult) = match (p1, p2) {
        (Some(p1), Some(p2)) => {
            let sum = p1 + p2;
            if sum <= 0.0 {
                return Ok(Color::TRANSPARENT);
            }
            (p1 / sum, p2 / sum, sum.clamp(0.0, 1.0))
        }
        (Some(p1), None) => {
            let p1 = p1.clamp(0.0, 1.0);
            (p1, 1.0 - p1, 1.0)
        }
        (None, Some(p2)) => {
            let p2 = p2.clamp(0.0, 1.0);
            (1.0 - p2, p2, 1.0)
        }
        (None, None) => (0.5, 0.5, 1.0),
    };

    let (_, _, _, a1) = c1.to_rgba_f32();
    let (_, _, _, a2) = c2.to_rgba_f32();
    let out_a_premul = a1 * w1 + a2 * w2;
    if out_a_premul <= 1e-6 {
        return Ok(Color::TRANSPARENT);
    }
    let final_alpha = (out_a_premul * alpha_mult).clamp(0.0, 1.0);

    let mixed = match space {
        ColorSpace::Oklab => {
            let lab1 = c1.to_oklab();
            let lab2 = c2.to_oklab();
            let l = (lab1.l * a1 * w1 + lab2.l * a2 * w2) / out_a_premul;
            let a = (lab1.a * a1 * w1 + lab2.a * a2 * w2) / out_a_premul;
            let b = (lab1.b * a1 * w1 + lab2.b * a2 * w2) / out_a_premul;
            Color::from_oklab(crate::math::oklab::Oklab::new(l, a, b, final_alpha))
        }
        ColorSpace::Oklch => {
            let lch1 = c1.to_oklch();
            let lch2 = c2.to_oklch();
            let l = (lch1.l * a1 * w1 + lch2.l * a2 * w2) / out_a_premul;
            let c = (lch1.c * a1 * w1 + lch2.c * a2 * w2) / out_a_premul;

            let h1_powerless = lch1.c < 1e-4;
            let h2_powerless = lch2.c < 1e-4;
            let (h1, h2) = match (h1_powerless, h2_powerless) {
                (true, false) => (lch2.h, lch2.h),
                (false, true) => (lch1.h, lch1.h),
                (true, true) => (0.0, 0.0),
                (false, false) => (lch1.h, lch2.h),
            };
            let h = interpolate_hue(h1, h2, w2, hue_method);
            Color::from_oklch(crate::math::oklab::Oklch::new(l, c, h, final_alpha))
        }
        ColorSpace::Lab => {
            let (l1, a1_val, b1_val, _) = c1.to_lab();
            let (l2, a2_val, b2_val, _) = c2.to_lab();
            let l = (l1 * a1 * w1 + l2 * a2 * w2) / out_a_premul;
            let a = (a1_val * a1 * w1 + a2_val * a2 * w2) / out_a_premul;
            let b = (b1_val * a1 * w1 + b2_val * a2 * w2) / out_a_premul;
            Color::from_lab(l, a, b, final_alpha)
        }
        ColorSpace::Lch => {
            let (l1, c1_val, h1_raw, _) = c1.to_lch();
            let (l2, c2_val, h2_raw, _) = c2.to_lch();
            let l = (l1 * a1 * w1 + l2 * a2 * w2) / out_a_premul;
            let c = (c1_val * a1 * w1 + c2_val * a2 * w2) / out_a_premul;

            let h1_powerless = c1_val < 1e-4;
            let h2_powerless = c2_val < 1e-4;
            let (h1, h2) = match (h1_powerless, h2_powerless) {
                (true, false) => (h2_raw, h2_raw),
                (false, true) => (h1_raw, h1_raw),
                (true, true) => (0.0, 0.0),
                (false, false) => (h1_raw, h2_raw),
            };
            let h = interpolate_hue(h1, h2, w2, hue_method);
            Color::from_lch(l, c, h, final_alpha)
        }
        ColorSpace::Hsl => {
            let (h1_raw, s1, l1, _) = c1.to_hsla();
            let (h2_raw, s2, l2, _) = c2.to_hsla();
            let s = (s1 * a1 * w1 + s2 * a2 * w2) / out_a_premul;
            let l = (l1 * a1 * w1 + l2 * a2 * w2) / out_a_premul;

            let h1_powerless = s1 < 1e-4;
            let h2_powerless = s2 < 1e-4;
            let (h1, h2) = match (h1_powerless, h2_powerless) {
                (true, false) => (h2_raw, h2_raw),
                (false, true) => (h1_raw, h1_raw),
                (true, true) => (0.0, 0.0),
                (false, false) => (h1_raw, h2_raw),
            };
            let h = interpolate_hue(h1, h2, w2, hue_method);
            Color::from_hsla(h, s, l, final_alpha)
        }
        ColorSpace::SrgbLinear => {
            let (r1, g1, b1, _) = c1.to_rgba_f32();
            let (r2, g2, b2, _) = c2.to_rgba_f32();
            let r_lin = (srgb_to_linear(r1) * a1 * w1 + srgb_to_linear(r2) * a2 * w2) / out_a_premul;
            let g_lin = (srgb_to_linear(g1) * a1 * w1 + srgb_to_linear(g2) * a2 * w2) / out_a_premul;
            let b_lin = (srgb_to_linear(b1) * a1 * w1 + srgb_to_linear(b2) * a2 * w2) / out_a_premul;
            Color::from_rgba_f32(linear_to_srgb(r_lin), linear_to_srgb(g_lin), linear_to_srgb(b_lin), final_alpha)
        }
        ColorSpace::DisplayP3 => {
            let (r1, g1, b1, _) = c1.to_rgba_f32();
            let (r2, g2, b2, _) = c2.to_rgba_f32();
            let (p1_r, p1_g, p1_b, _) = srgb_to_display_p3(r1, g1, b1, a1);
            let (p2_r, p2_g, p2_b, _) = srgb_to_display_p3(r2, g2, b2, a2);
            let r = (p1_r * a1 * w1 + p2_r * a2 * w2) / out_a_premul;
            let g = (p1_g * a1 * w1 + p2_g * a2 * w2) / out_a_premul;
            let b = (p1_b * a1 * w1 + p2_b * a2 * w2) / out_a_premul;
            let (sr, sg, sb, _) = display_p3_to_srgb(r, g, b, 1.0);
            Color::from_rgba_f32(sr, sg, sb, final_alpha)
        }
        _ => {
            let (r1, g1, b1, _) = c1.to_rgba_f32();
            let (r2, g2, b2, _) = c2.to_rgba_f32();
            let r = (r1 * a1 * w1 + r2 * a2 * w2) / out_a_premul;
            let g = (g1 * a1 * w1 + g2 * a2 * w2) / out_a_premul;
            let b = (b1 * a1 * w1 + b2 * a2 * w2) / out_a_premul;
            Color::from_rgba_f32(r, g, b, final_alpha)
        }
    };
    Ok(mixed)
}

/// Resolve `color-mix()` no espaço de cor solicitado com cálculo preciso de pesos.
pub fn mix_colors(
    space: ColorSpace,
    c1: Color,
    p1: Option<f32>,
    c2: Color,
    p2: Option<f32>,
) -> Result<Color, AceError> {
    mix_colors_with_hue(space, HueInterpolation::Shorter, c1, p1, c2, p2)
}

/// Interpola dois ângulos de matiz em graus conforme CSS Color 4 §12.4.
pub fn interpolate_hue(h1: f32, h2: f32, t: f32, method: HueInterpolation) -> f32 {
    let mut h1 = (h1 % 360.0 + 360.0) % 360.0;
    let mut h2 = (h2 % 360.0 + 360.0) % 360.0;

    match method {
        HueInterpolation::Shorter => {
            let diff = h2 - h1;
            if diff > 180.0 {
                h1 += 360.0;
            } else if diff < -180.0 {
                h2 += 360.0;
            }
        }
        HueInterpolation::Longer => {
            let diff = h2 - h1;
            if diff > 0.0 && diff < 180.0 {
                h1 += 360.0;
            } else if diff <= 0.0 && diff > -180.0 {
                h2 += 360.0;
            }
        }
        HueInterpolation::Increasing => {
            if h2 < h1 {
                h2 += 360.0;
            }
        }
        HueInterpolation::Decreasing => {
            if h1 < h2 {
                h1 += 360.0;
            }
        }
    }

    let h = h1 + (h2 - h1) * t;
    (h % 360.0 + 360.0) % 360.0
}

/// Converte Linear sRGB (D65) para CIE XYZ (D50) usando a matriz combinada com Adaptação Cromática Bradford (W3C CSS Color 4 §9.2).
#[inline]
pub fn linear_srgb_to_xyz_d50(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let x = 0.4360747 * r + 0.3850649 * g + 0.1430804 * b;
    let y = 0.2225045 * r + 0.7168786 * g + 0.0606169 * b;
    let z = 0.0139322 * r + 0.0971045 * g + 0.7141733 * b;
    (x, y, z)
}

/// Converte CIE XYZ (D50) para Linear sRGB (D65) usando a matriz inversa Bradford (W3C CSS Color 4 §9.2).
#[inline]
pub fn xyz_d50_to_linear_srgb(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    let r = 3.1338561 * x - 1.6168667 * y - 0.4906146 * z;
    let g = -0.9787684 * x + 1.9161415 * y + 0.0334540 * z;
    let b = 0.0719453 * x - 0.2289914 * y + 1.4052427 * z;
    (r, g, b)
}

/// Ponto branco de referência D50 e constantes CIE Lab (W3C CSS Color 4 §9.1).
pub const D50_XN: f32 = 0.96422;
pub const D50_YN: f32 = 1.00000;
pub const D50_ZN: f32 = 0.82521;
pub const CIE_EPSILON: f32 = 216.0 / 24389.0; // ≈ 0.008856451679035631
pub const CIE_KAPPA: f32 = 24389.0 / 27.0;    // ≈ 903.2962962962963

/// Converte CIE XYZ (D50) para CIE L*a*b* (D50).
#[inline]
pub fn xyz_d50_to_lab(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    let xr = x / D50_XN;
    let yr = y / D50_YN;
    let zr = z / D50_ZN;

    let fx = if xr > CIE_EPSILON {
        xr.cbrt()
    } else {
        (CIE_KAPPA * xr + 16.0) / 116.0
    };
    let fy = if yr > CIE_EPSILON {
        yr.cbrt()
    } else {
        (CIE_KAPPA * yr + 16.0) / 116.0
    };
    let fz = if zr > CIE_EPSILON {
        zr.cbrt()
    } else {
        (CIE_KAPPA * zr + 16.0) / 116.0
    };

    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);
    (l, a, b)
}

/// Converte CIE L*a*b* (D50) para CIE XYZ (D50).
#[inline]
pub fn lab_to_xyz_d50(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let fy = (l + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;

    let fx3 = fx * fx * fx;
    let fz3 = fz * fz * fz;

    let xr = if fx3 > CIE_EPSILON {
        fx3
    } else {
        (116.0 * fx - 16.0) / CIE_KAPPA
    };
    let yr = if l > CIE_KAPPA * CIE_EPSILON {
        fy * fy * fy
    } else {
        l / CIE_KAPPA
    };
    let zr = if fz3 > CIE_EPSILON {
        fz3
    } else {
        (116.0 * fz - 16.0) / CIE_KAPPA
    };

    let x = xr * D50_XN;
    let y = yr * D50_YN;
    let z = zr * D50_ZN;
    (x, y, z)
}

/// Converte CIE L*a*b* para CIE L*C*h.
#[inline]
pub fn lab_to_lch(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let c = (a * a + b * b).sqrt();
    let mut h = b.atan2(a) * (180.0 / std::f32::consts::PI);
    if h < 0.0 {
        h += 360.0;
    }
    (l, c, h)
}

/// Converte CIE L*C*h para CIE L*a*b*.
#[inline]
pub fn lch_to_lab(l: f32, c: f32, h: f32) -> (f32, f32, f32) {
    let h_rad = h * (std::f32::consts::PI / 180.0);
    let a = c * h_rad.cos();
    let b = c * h_rad.sin();
    (l, a, b)
}

#[inline]
pub fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
pub fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

fn parse_hue_angle(s: &str) -> Result<f32, AceError> {
    let s = s.trim();
    if let Some(val) = s.strip_suffix("deg") {
        val.parse::<f32>().map_err(|_| invalid_color(s))
    } else if let Some(val) = s.strip_suffix("grad") {
        let g: f32 = val.parse().map_err(|_| invalid_color(s))?;
        Ok(g * 0.9)
    } else if let Some(val) = s.strip_suffix("rad") {
        let r: f32 = val.parse().map_err(|_| invalid_color(s))?;
        Ok(r * (180.0 / std::f32::consts::PI))
    } else if let Some(val) = s.strip_suffix("turn") {
        let t: f32 = val.parse().map_err(|_| invalid_color(s))?;
        Ok(t * 360.0)
    } else {
        s.parse::<f32>().map_err(|_| invalid_color(s))
    }
}

fn parse_lab_percentage(s: &str, scale_for_pct: f32) -> Result<f32, AceError> {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        let val: f32 = pct.parse().map_err(|_| invalid_color(s))?;
        Ok((val / 100.0) * scale_for_pct)
    } else {
        s.parse::<f32>().map_err(|_| invalid_color(s))
    }
}

fn parse_component(s: &str, max: f32) -> Result<f32, AceError> {
    if let Some(pct) = s.strip_suffix('%') {
        let val: f32 = pct.parse().map_err(|_| invalid_color(s))?;
        Ok((val / 100.0) * max)
    } else {
        s.parse::<f32>().map_err(|_| invalid_color(s))
    }
}

fn parse_percentage(s: &str) -> Result<f32, AceError> {
    if let Some(pct) = s.strip_suffix('%') {
        let val: f32 = pct.parse().map_err(|_| invalid_color(s))?;
        Ok(val / 100.0)
    } else {
        s.parse::<f32>().map_err(|_| invalid_color(s))
    }
}

fn parse_alpha(s: &str) -> Result<f32, AceError> {
    if let Some(pct) = s.strip_suffix('%') {
        let val: f32 = pct.parse().map_err(|_| invalid_color(s))?;
        Ok(val / 100.0)
    } else {
        s.parse::<f32>().map_err(|_| invalid_color(s))
    }
}

/// Dicionário de cores CSS nomeadas padrão (CSS Color Module Level 4).
fn named_color(name: &str) -> Option<Color> {
    match name {
        "transparent" => Some(Color::TRANSPARENT),
        "black" => Some(Color::BLACK),
        "white" => Some(Color::WHITE),
        "red" => Some(Color::RED),
        "green" => Some(Color::GREEN),
        "lime" => Some(Color::LIME),
        "blue" => Some(Color::BLUE),
        "yellow" => Some(Color::YELLOW),
        "cyan" | "aqua" => Some(Color::CYAN),
        "magenta" | "fuchsia" => Some(Color::MAGENTA),
        "gray" | "grey" => Some(Color::GRAY),
        "silver" => Some(Color::SILVER),
        "maroon" => Some(Color::from_rgb(128, 0, 0)),
        "olive" => Some(Color::from_rgb(128, 128, 0)),
        "navy" => Some(Color::from_rgb(0, 0, 128)),
        "purple" => Some(Color::from_rgb(128, 0, 128)),
        "teal" => Some(Color::from_rgb(0, 128, 128)),
        "orange" => Some(Color::from_rgb(255, 165, 0)),
        "aliceblue" => Some(Color::from_rgb(240, 248, 255)),
        "antiquewhite" => Some(Color::from_rgb(250, 235, 215)),
        "aquamarine" => Some(Color::from_rgb(127, 255, 212)),
        "azure" => Some(Color::from_rgb(240, 255, 255)),
        "beige" => Some(Color::from_rgb(245, 245, 220)),
        "bisque" => Some(Color::from_rgb(255, 228, 196)),
        "blanchedalmond" => Some(Color::from_rgb(255, 235, 205)),
        "blueviolet" => Some(Color::from_rgb(138, 43, 226)),
        "brown" => Some(Color::from_rgb(165, 42, 42)),
        "burlywood" => Some(Color::from_rgb(222, 184, 135)),
        "cadetblue" => Some(Color::from_rgb(95, 158, 160)),
        "chartreuse" => Some(Color::from_rgb(127, 255, 0)),
        "chocolate" => Some(Color::from_rgb(210, 105, 30)),
        "coral" => Some(Color::from_rgb(255, 127, 80)),
        "cornflowerblue" => Some(Color::from_rgb(100, 149, 237)),
        "cornsilk" => Some(Color::from_rgb(255, 248, 220)),
        "crimson" => Some(Color::from_rgb(220, 20, 60)),
        "darkblue" => Some(Color::from_rgb(0, 0, 139)),
        "darkcyan" => Some(Color::from_rgb(0, 139, 139)),
        "darkgoldenrod" => Some(Color::from_rgb(184, 134, 11)),
        "darkgray" | "darkgrey" => Some(Color::from_rgb(169, 169, 169)),
        "darkgreen" => Some(Color::from_rgb(0, 100, 0)),
        "darkkhaki" => Some(Color::from_rgb(189, 183, 107)),
        "darkmagenta" => Some(Color::from_rgb(139, 0, 139)),
        "darkolivegreen" => Some(Color::from_rgb(85, 107, 47)),
        "darkorange" => Some(Color::from_rgb(255, 140, 0)),
        "darkorchid" => Some(Color::from_rgb(153, 50, 204)),
        "darkred" => Some(Color::from_rgb(139, 0, 0)),
        "darksalmon" => Some(Color::from_rgb(233, 150, 122)),
        "darkseagreen" => Some(Color::from_rgb(143, 188, 143)),
        "darkslateblue" => Some(Color::from_rgb(72, 61, 139)),
        "darkslategray" | "darkslategrey" => Some(Color::from_rgb(47, 79, 79)),
        "darkturquoise" => Some(Color::from_rgb(0, 206, 209)),
        "darkviolet" => Some(Color::from_rgb(148, 0, 211)),
        "deeppink" => Some(Color::from_rgb(255, 20, 147)),
        "deepskyblue" => Some(Color::from_rgb(0, 191, 255)),
        "dimgray" | "dimgrey" => Some(Color::from_rgb(105, 105, 105)),
        "dodgerblue" => Some(Color::from_rgb(30, 144, 255)),
        "firebrick" => Some(Color::from_rgb(178, 34, 34)),
        "floralwhite" => Some(Color::from_rgb(255, 250, 240)),
        "forestgreen" => Some(Color::from_rgb(34, 139, 34)),
        "gainsboro" => Some(Color::from_rgb(220, 220, 220)),
        "ghostwhite" => Some(Color::from_rgb(248, 248, 255)),
        "gold" => Some(Color::from_rgb(255, 215, 0)),
        "goldenrod" => Some(Color::from_rgb(218, 165, 32)),
        "greenyellow" => Some(Color::from_rgb(173, 255, 47)),
        "honeydew" => Some(Color::from_rgb(240, 255, 240)),
        "hotpink" => Some(Color::from_rgb(255, 105, 180)),
        "indianred" => Some(Color::from_rgb(205, 92, 92)),
        "indigo" => Some(Color::from_rgb(75, 0, 130)),
        "ivory" => Some(Color::from_rgb(255, 255, 240)),
        "khaki" => Some(Color::from_rgb(240, 230, 140)),
        "lavender" => Some(Color::from_rgb(230, 230, 250)),
        "lavenderblush" => Some(Color::from_rgb(255, 240, 245)),
        "lawngreen" => Some(Color::from_rgb(124, 252, 0)),
        "lemonchiffon" => Some(Color::from_rgb(255, 250, 205)),
        "lightblue" => Some(Color::from_rgb(173, 216, 230)),
        "lightcoral" => Some(Color::from_rgb(240, 128, 128)),
        "lightcyan" => Some(Color::from_rgb(224, 255, 255)),
        "lightgoldenrodyellow" => Some(Color::from_rgb(250, 250, 210)),
        "lightgray" | "lightgrey" => Some(Color::from_rgb(211, 211, 211)),
        "lightgreen" => Some(Color::from_rgb(144, 238, 144)),
        "lightpink" => Some(Color::from_rgb(255, 182, 193)),
        "lightsalmon" => Some(Color::from_rgb(255, 160, 122)),
        "lightseagreen" => Some(Color::from_rgb(32, 178, 170)),
        "lightskyblue" => Some(Color::from_rgb(135, 206, 250)),
        "lightslategray" | "lightslategrey" => Some(Color::from_rgb(119, 136, 153)),
        "lightsteelblue" => Some(Color::from_rgb(176, 196, 222)),
        "lightyellow" => Some(Color::from_rgb(255, 255, 224)),
        "limegreen" => Some(Color::from_rgb(50, 205, 50)),
        "linen" => Some(Color::from_rgb(250, 240, 230)),
        "mediumaquamarine" => Some(Color::from_rgb(102, 205, 170)),
        "mediumblue" => Some(Color::from_rgb(0, 0, 205)),
        "mediumorchid" => Some(Color::from_rgb(186, 85, 211)),
        "mediumpurple" => Some(Color::from_rgb(147, 112, 219)),
        "mediumseagreen" => Some(Color::from_rgb(60, 179, 113)),
        "mediumslateblue" => Some(Color::from_rgb(123, 104, 238)),
        "mediumspringgreen" => Some(Color::from_rgb(0, 250, 154)),
        "mediumturquoise" => Some(Color::from_rgb(72, 209, 204)),
        "mediumvioletred" => Some(Color::from_rgb(199, 21, 133)),
        "midnightblue" => Some(Color::from_rgb(25, 25, 112)),
        "mintcream" => Some(Color::from_rgb(245, 255, 250)),
        "mistyrose" => Some(Color::from_rgb(255, 228, 225)),
        "moccasin" => Some(Color::from_rgb(255, 228, 181)),
        "navajowhite" => Some(Color::from_rgb(255, 222, 173)),
        "oldlace" => Some(Color::from_rgb(253, 245, 230)),
        "olivedrab" => Some(Color::from_rgb(107, 142, 35)),
        "orangered" => Some(Color::from_rgb(255, 69, 0)),
        "orchid" => Some(Color::from_rgb(218, 112, 214)),
        "palegoldenrod" => Some(Color::from_rgb(238, 232, 170)),
        "palegreen" => Some(Color::from_rgb(152, 251, 152)),
        "paleturquoise" => Some(Color::from_rgb(175, 238, 238)),
        "palevioletred" => Some(Color::from_rgb(219, 112, 147)),
        "papayawhip" => Some(Color::from_rgb(255, 239, 213)),
        "peachpuff" => Some(Color::from_rgb(255, 218, 185)),
        "peru" => Some(Color::from_rgb(205, 133, 63)),
        "pink" => Some(Color::from_rgb(255, 192, 203)),
        "plum" => Some(Color::from_rgb(221, 160, 221)),
        "powderblue" => Some(Color::from_rgb(176, 224, 230)),
        "rebeccapurple" => Some(Color::from_rgb(102, 51, 153)),
        "rosybrown" => Some(Color::from_rgb(188, 143, 143)),
        "royalblue" => Some(Color::from_rgb(65, 105, 225)),
        "saddlebrown" => Some(Color::from_rgb(139, 69, 19)),
        "salmon" => Some(Color::from_rgb(250, 128, 114)),
        "sandybrown" => Some(Color::from_rgb(244, 164, 96)),
        "seagreen" => Some(Color::from_rgb(46, 139, 87)),
        "seashell" => Some(Color::from_rgb(255, 245, 238)),
        "sienna" => Some(Color::from_rgb(160, 82, 45)),
        "skyblue" => Some(Color::from_rgb(135, 206, 235)),
        "slateblue" => Some(Color::from_rgb(106, 90, 205)),
        "slategray" | "slategrey" => Some(Color::from_rgb(112, 128, 144)),
        "snow" => Some(Color::from_rgb(255, 250, 250)),
        "springgreen" => Some(Color::from_rgb(0, 255, 127)),
        "steelblue" => Some(Color::from_rgb(70, 130, 180)),
        "tan" => Some(Color::from_rgb(210, 180, 140)),
        "thistle" => Some(Color::from_rgb(216, 191, 216)),
        "tomato" => Some(Color::from_rgb(255, 99, 71)),
        "turquoise" => Some(Color::from_rgb(64, 224, 208)),
        "violet" => Some(Color::from_rgb(238, 130, 238)),
        "wheat" => Some(Color::from_rgb(245, 222, 179)),
        "whitesmoke" => Some(Color::from_rgb(245, 245, 245)),
        "yellowgreen" => Some(Color::from_rgb(154, 205, 50)),
        _ => None,
    }
}

impl From<(u8, u8, u8)> for Color {
    #[inline]
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::from_rgb(r, g, b)
    }
}

impl From<(u8, u8, u8, u8)> for Color {
    #[inline]
    fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
        Self::from_rgba(r, g, b, a)
    }
}
