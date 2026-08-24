//! # Composição Porter-Duff e Modos de Mesclagem CSS Compositing Level 1
//!
//! Implementação completa dos 12 operadores clássicos de composição com alfa pré-multiplicado
//! e das 11 funções analíticas de mesclagem separáveis do padrão W3C CSS Compositing and Blending Module Level 1.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorRgba {
    pub r: f32, // [0.0, 1.0]
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRgba {
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn clamp(&self) -> Self {
        Self {
            r: self.r.clamp(0.0, 1.0),
            g: self.g.clamp(0.0, 1.0),
            b: self.b.clamp(0.0, 1.0),
            a: self.a.clamp(0.0, 1.0),
        }
    }
}

/// Os 12 operadores canônicos de composição gráfica de Thomas Porter e Tom Duff (1984).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PorterDuffOperator {
    Clear,
    Copy,
    Dst,
    SrcOver,
    DstOver,
    SrcIn,
    DstIn,
    SrcOut,
    DstOut,
    SrcAtop,
    DstAtop,
    Xor,
}

/// Os 11 modos de mesclagem separáveis do W3C CSS Compositing Level 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CssBlendMode {
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
}

/// Executa a composição Porter-Duff com alfa pré-multiplicado.
pub fn composite_porter_duff(
    src: ColorRgba,
    dst: ColorRgba,
    op: PorterDuffOperator,
) -> ColorRgba {
    let (cs_r, cs_g, cs_b, as_) = (src.r * src.a, src.g * src.a, src.b * src.a, src.a);
    let (cb_r, cb_g, cb_b, ab) = (dst.r * dst.a, dst.g * dst.a, dst.b * dst.a, dst.a);

    let (fs, fb) = match op {
        PorterDuffOperator::Clear => (0.0, 0.0),
        PorterDuffOperator::Copy => (1.0, 0.0),
        PorterDuffOperator::Dst => (0.0, 1.0),
        PorterDuffOperator::SrcOver => (1.0, 1.0 - as_),
        PorterDuffOperator::DstOver => (1.0 - ab, 1.0),
        PorterDuffOperator::SrcIn => (ab, 0.0),
        PorterDuffOperator::DstIn => (0.0, as_),
        PorterDuffOperator::SrcOut => (1.0 - ab, 0.0),
        PorterDuffOperator::DstOut => (0.0, 1.0 - as_),
        PorterDuffOperator::SrcAtop => (ab, 1.0 - as_),
        PorterDuffOperator::DstAtop => (1.0 - ab, as_),
        PorterDuffOperator::Xor => (1.0 - ab, 1.0 - as_),
    };

    let out_a = (as_ * fs + ab * fb).clamp(0.0, 1.0);
    if out_a <= 1e-7 {
        return ColorRgba::TRANSPARENT;
    }

    let out_r = (cs_r * fs + cb_r * fb) / out_a;
    let out_g = (cs_g * fs + cb_g * fb) / out_a;
    let out_b = (cs_b * fs + cb_b * fb) / out_a;

    ColorRgba::new(out_r, out_g, out_b, out_a).clamp()
}

/// Função de mesclagem analítica W3C por canal de cor B(Cb, Cs).
fn blend_channel(cb: f32, cs: f32, mode: CssBlendMode) -> f32 {
    match mode {
        CssBlendMode::Multiply => cb * cs,
        CssBlendMode::Screen => cb + cs - cb * cs,
        CssBlendMode::Darken => cb.min(cs),
        CssBlendMode::Lighten => cb.max(cs),
        CssBlendMode::Difference => (cb - cs).abs(),
        CssBlendMode::Exclusion => cb + cs - 2.0 * cb * cs,
        CssBlendMode::HardLight => {
            if cs <= 0.5 {
                2.0 * cb * cs
            } else {
                1.0 - 2.0 * (1.0 - cb) * (1.0 - cs)
            }
        }
        CssBlendMode::Overlay => {
            if cb <= 0.5 {
                2.0 * cb * cs
            } else {
                1.0 - 2.0 * (1.0 - cb) * (1.0 - cs)
            }
        }
        CssBlendMode::ColorDodge => {
            if cb == 0.0 {
                0.0
            } else if cs >= 1.0 {
                1.0
            } else {
                (cb / (1.0 - cs)).min(1.0)
            }
        }
        CssBlendMode::ColorBurn => {
            if cb >= 1.0 {
                1.0
            } else if cs == 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - cb) / cs).min(1.0)
            }
        }
        CssBlendMode::SoftLight => {
            if cs <= 0.5 {
                cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
            } else {
                let d_cb = if cb <= 0.25 {
                    ((16.0 * cb - 12.0) * cb + 4.0) * cb
                } else {
                    cb.sqrt()
                };
                cb + (2.0 * cs - 1.0) * (d_cb - cb)
            }
        }
    }
}

/// Executa a mesclagem CSS Compositing & Blending Level 1.
pub fn composite_blend(src: ColorRgba, dst: ColorRgba, mode: CssBlendMode) -> ColorRgba {
    let as_ = src.a;
    let ab = dst.a;
    let out_a = as_ + ab * (1.0 - as_);

    if out_a <= 1e-7 {
        return ColorRgba::TRANSPARENT;
    }

    let blend_r = blend_channel(dst.r, src.r, mode);
    let blend_g = blend_channel(dst.g, src.g, mode);
    let blend_b = blend_channel(dst.b, src.b, mode);

    let out_r =
        ((1.0 - ab) * as_ * src.r + (1.0 - as_) * ab * dst.r + as_ * ab * blend_r) / out_a;
    let out_g =
        ((1.0 - ab) * as_ * src.g + (1.0 - as_) * ab * dst.g + as_ * ab * blend_g) / out_a;
    let out_b =
        ((1.0 - ab) * as_ * src.b + (1.0 - as_) * ab * dst.b + as_ * ab * blend_b) / out_a;

    ColorRgba::new(out_r, out_g, out_b, out_a).clamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_porter_duff_srcover_blending() {
        let src = ColorRgba::new(1.0, 0.0, 0.0, 0.5);
        let dst = ColorRgba::new(0.0, 0.0, 1.0, 1.0);
        let res = composite_porter_duff(src, dst, PorterDuffOperator::SrcOver);
        assert_eq!(res.a, 1.0);
        assert!((res.r - 0.5).abs() < 1e-5);
        assert!((res.b - 0.5).abs() < 1e-5);
    }
}
