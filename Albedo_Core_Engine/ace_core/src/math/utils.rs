//! # Utilitários Matemáticos para Geometria e Renderização
//!
//! Funções matemáticas de interpolação, aproximação de ponto flutuante,
//! conversões angulares e alinhamento de pixel (pixel-snapping).

use std::f32::consts::PI;

/// Interpolação linear entre `a` e `b` com peso `t` (0.0 ..= 1.0).
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Restringe um valor dentro do intervalo fechado `[min, max]`.
#[inline]
pub fn clamp(val: f32, min: f32, max: f32) -> f32 {
    val.clamp(min, max)
}

/// Restringe um valor float para a faixa padrão de cores/alfa `[0.0, 1.0]`.
#[inline]
pub fn saturate(val: f32) -> f32 {
    val.clamp(0.0, 1.0)
}

/// Compara se dois floats são aproximadamente iguais dentro de uma tolerância `epsilon`.
#[inline]
pub fn almost_equal(a: f32, b: f32, epsilon: f32) -> bool {
    (a - b).abs() <= epsilon
}

/// Converte um ângulo de graus para radianos.
#[inline]
pub fn deg_to_rad(deg: f32) -> f32 {
    deg * (PI / 180.0)
}

/// Converte um ângulo de radianos para graus.
#[inline]
pub fn rad_to_deg(rad: f32) -> f32 {
    rad * (180.0 / PI)
}

/// Arredonda uma coordenada para o pixel físico mais próximo (Pixel Snapping),
/// evitando bordas borradas e artefatos de subpixel em renderização de layout.
#[inline]
pub fn snap_to_pixel(val: f32, scale: f32) -> f32 {
    if scale == 0.0 {
        val
    } else {
        (val * scale).round() / scale
    }
}

/// Retorna o valor mínimo entre 3 floats.
#[inline]
pub fn min3(a: f32, b: f32, c: f32) -> f32 {
    a.min(b).min(c)
}

/// Retorna o valor máximo entre 3 floats.
#[inline]
pub fn max3(a: f32, b: f32, c: f32) -> f32 {
    a.max(b).max(c)
}

/// Retorna o valor mínimo entre 4 floats.
#[inline]
pub fn min4(a: f32, b: f32, c: f32, d: f32) -> f32 {
    a.min(b).min(c).min(d)
}

/// Retorna o valor máximo entre 4 floats.
#[inline]
pub fn max4(a: f32, b: f32, c: f32, d: f32) -> f32 {
    a.max(b).max(c).max(d)
}
