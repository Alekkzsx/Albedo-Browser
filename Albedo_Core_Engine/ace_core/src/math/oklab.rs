//! # Espaços de Cor Perceptuais Oklab e Oklch (CSS Color Module Level 4)
//!
//! Implementação matemática de alta precisão para os espaços perceptuais uniformes
//! Oklab e Oklch conforme a formulação canônica de Björn Ottosson e especificação W3C.

use std::f32::consts::PI;
use std::fmt;

/// Representação no espaço de cor perceptual Oklab.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklab {
    /// Luminosidade perceptual ($L \in [0.0, 1.0]$).
    pub l: f32,
    /// Eixo verde-vermelho ($a \approx [-0.4, 0.4]$).
    pub a: f32,
    /// Eixo azul-amarelo ($b \approx [-0.4, 0.4]$).
    pub b: f32,
    /// Canal alfa ($[0.0, 1.0]$).
    pub alpha: f32,
}

impl Oklab {
    /// Cria uma nova cor no espaço Oklab.
    #[inline]
    pub const fn new(l: f32, a: f32, b: f32, alpha: f32) -> Self {
        Self { l, a, b, alpha }
    }

    /// Converte valores sRGB não-lineares ($[0.0, 1.0]$) para Oklab.
    pub fn from_srgb(r: f32, g: f32, b: f32, alpha: f32) -> Self {
        let r_lin = srgb_to_linear(r);
        let g_lin = srgb_to_linear(g);
        let b_lin = srgb_to_linear(b);

        let l = 0.4122214708 * r_lin + 0.5363325363 * g_lin + 0.0514459929 * b_lin;
        let m = 0.2119034982 * r_lin + 0.6806995451 * g_lin + 0.1073969566 * b_lin;
        let s = 0.0883024619 * r_lin + 0.2817188376 * g_lin + 0.6299787005 * b_lin;

        let l_ = l.cbrt();
        let m_ = m.cbrt();
        let s_ = s.cbrt();

        Self {
            l: 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
            a: 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
            b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
            alpha,
        }
    }

    /// Converte esta cor Oklab para valores sRGB não-lineares ($[0.0, 1.0]$) com clamping.
    pub fn to_srgb(&self) -> (f32, f32, f32, f32) {
        let l_ = self.l + 0.3963377774 * self.a + 0.2158037573 * self.b;
        let m_ = self.l - 0.1055613458 * self.a - 0.0638541728 * self.b;
        let s_ = self.l - 0.0894841775 * self.a - 1.2914855480 * self.b;

        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        let r_lin = 4.0767434721 * l - 3.3077115913 * m + 0.2309699292 * s;
        let g_lin = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
        let b_lin = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

        let r = linear_to_srgb(r_lin).clamp(0.0, 1.0);
        let g = linear_to_srgb(g_lin).clamp(0.0, 1.0);
        let b = linear_to_srgb(b_lin).clamp(0.0, 1.0);

        (r, g, b, self.alpha.clamp(0.0, 1.0))
    }

    /// Converte esta cor Oklab para a forma cilíndrica Oklch.
    pub fn to_oklch(&self) -> Oklch {
        let chroma = (self.a * self.a + self.b * self.b).sqrt();
        let mut hue = self.b.atan2(self.a) * (180.0 / PI);
        if hue < 0.0 {
            hue += 360.0;
        }

        Oklch {
            l: self.l,
            c: chroma,
            h: hue,
            alpha: self.alpha,
        }
    }
}

/// Representação no espaço de cor perceptual cilíndrico Oklch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    /// Luminosidade perceptual ($L \in [0.0, 1.0]$).
    pub l: f32,
    /// Croma / Saturação ($C \ge 0.0$).
    pub c: f32,
    /// Matiz / Ângulo ($H \in [0.0^\circ, 360.0^\circ)$).
    pub h: f32,
    /// Canal alfa ($[0.0, 1.0]$).
    pub alpha: f32,
}

impl Oklch {
    /// Cria uma nova cor no espaço Oklch.
    #[inline]
    pub const fn new(l: f32, c: f32, h: f32, alpha: f32) -> Self {
        Self { l, c, h, alpha }
    }

    /// Converte esta cor Oklch para a forma cartesiana Oklab.
    pub fn to_oklab(&self) -> Oklab {
        let h_rad = self.h * (PI / 180.0);
        let a = self.c * h_rad.cos();
        let b = self.c * h_rad.sin();

        Oklab {
            l: self.l,
            a,
            b,
            alpha: self.alpha,
        }
    }

    /// Converte valores sRGB não-lineares para Oklch.
    pub fn from_srgb(r: f32, g: f32, b: f32, alpha: f32) -> Self {
        Oklab::from_srgb(r, g, b, alpha).to_oklch()
    }

    /// Converte esta cor Oklch para valores sRGB não-lineares ($[0.0, 1.0]$).
    pub fn to_srgb(&self) -> (f32, f32, f32, f32) {
        self.to_oklab().to_srgb()
    }
}

/// Converte um canal sRGB não-linear para Linear sRGB.
#[inline]
fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Converte um canal Linear sRGB para sRGB não-linear.
#[inline]
fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

impl fmt::Display for Oklab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (self.alpha - 1.0).abs() < 0.001 {
            write!(f, "oklab({:.3} {:.3} {:.3})", self.l, self.a, self.b)
        } else {
            write!(f, "oklab({:.3} {:.3} {:.3} / {:.2})", self.l, self.a, self.b, self.alpha)
        }
    }
}

impl fmt::Display for Oklch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (self.alpha - 1.0).abs() < 0.001 {
            write!(f, "oklch({:.3} {:.3} {:.1}deg)", self.l, self.c, self.h)
        } else {
            write!(f, "oklch({:.3} {:.3} {:.1}deg / {:.2})", self.l, self.c, self.h, self.alpha)
        }
    }
}
