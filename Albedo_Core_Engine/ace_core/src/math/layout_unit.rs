//! # Unidade de Layout em Ponto Fixo (Subpixel Arithmetic)
//!
//! Implementa a primitiva `LayoutUnit` (base 60), inspirada no `Au` do Gecko/Servo e `LayoutUnit` do Blink.
//!
//! 1 pixel CSS equivale a **60 LayoutUnits**. Isso permite divisões inteiras exatas por
//! 2, 3, 4, 5, 6, 10, 12, 15, 20, 30 e 60, prevenindo deriva de arredondamento cumulativa
//! em algoritmos de layout (Flexbox, CSS Grid, Quebra de Texto).

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Número de unidades de layout por pixel CSS.
pub const UNITS_PER_PIXEL: i32 = 60;

/// Valor mínimo representável por um `LayoutUnit`.
pub const MIN: LayoutUnit = LayoutUnit(i32::MIN);

/// Valor máximo representável por um `LayoutUnit`.
pub const MAX: LayoutUnit = LayoutUnit(i32::MAX);

/// Zero em `LayoutUnit`.
pub const ZERO: LayoutUnit = LayoutUnit(0);

/// Uma unidade geométrica inteira em ponto fixo (1/60 px) para layout determinístico.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LayoutUnit(pub i32);

impl LayoutUnit {
    /// Valor mínimo representável.
    pub const MIN: Self = MIN;
    /// Valor máximo representável.
    pub const MAX: Self = MAX;
    /// Zero em unidades de layout.
    pub const ZERO: Self = ZERO;

    /// Cria um `LayoutUnit` a partir de um valor bruto de unidades inteiras (1/60 px).
    #[inline]
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    /// Cria um `LayoutUnit` a partir de pixels inteiros.
    #[inline]
    pub const fn from_px(px: i32) -> Self {
        Self(px.saturating_mul(UNITS_PER_PIXEL))
    }

    /// Cria um `LayoutUnit` a partir de um valor em ponto flutuante (`f32`).
    #[inline]
    pub fn from_f32_px(px: f32) -> Self {
        let raw = (px * UNITS_PER_PIXEL as f32).round();
        if raw >= i32::MAX as f32 {
            MAX
        } else if raw <= i32::MIN as f32 {
            MIN
        } else {
            Self(raw as i32)
        }
    }

    /// Retorna o valor bruto em unidades inteiras (1/60 px).
    #[inline]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Converte para pixels em ponto flutuante (`f32`).
    #[inline]
    pub fn to_f32_px(self) -> f32 {
        self.0 as f32 / UNITS_PER_PIXEL as f32
    }

    /// Arredonda para baixo e retorna a quantidade inteira de pixels CSS.
    #[inline]
    pub fn floor_px(self) -> i32 {
        if self.0 >= 0 {
            self.0 / UNITS_PER_PIXEL
        } else {
            (self.0 - (UNITS_PER_PIXEL - 1)) / UNITS_PER_PIXEL
        }
    }

    /// Arredonda para cima e retorna a quantidade inteira de pixels CSS.
    #[inline]
    pub fn ceil_px(self) -> i32 {
        if self.0 >= 0 {
            (self.0 + (UNITS_PER_PIXEL - 1)) / UNITS_PER_PIXEL
        } else {
            self.0 / UNITS_PER_PIXEL
        }
    }

    /// Arredonda para o pixel inteiro mais próximo.
    #[inline]
    pub fn round_px(self) -> i32 {
        if self.0 >= 0 {
            (self.0 + UNITS_PER_PIXEL / 2) / UNITS_PER_PIXEL
        } else {
            (self.0 - UNITS_PER_PIXEL / 2) / UNITS_PER_PIXEL
        }
    }

    /// Retorna o valor absoluto.
    #[inline]
    pub const fn abs(self) -> Self {
        Self(self.0.saturating_abs())
    }

    /// Retorna o menor entre dois valores.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    /// Retorna o maior entre dois valores.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    /// Restringe o valor ao intervalo `[min, max]`.
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self(self.0.clamp(min.0, max.0))
    }

    /// Adição saturada protegida contra overflow.
    #[inline]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Subtração saturada protegida contra underflow.
    #[inline]
    pub const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    /// Multiplicação inteira saturada.
    #[inline]
    pub const fn saturating_mul(self, factor: i32) -> Self {
        Self(self.0.saturating_mul(factor))
    }

    /// Multiplicação fracionária segura com acumulador temporário `i64`:
    /// calcula `(self * numerator) / denominator` sem estouro intermediário de 32 bits.
    #[inline]
    pub fn mul_div(self, numerator: i32, denominator: i32) -> Self {
        if denominator == 0 {
            return if (self.0 >= 0) == (numerator >= 0) {
                MAX
            } else {
                MIN
            };
        }
        let result = (self.0 as i64 * numerator as i64) / denominator as i64;
        let clamped = result.clamp(i32::MIN as i64, i32::MAX as i64);
        Self(clamped as i32)
    }

    /// Multiplicação fracionária segura por uma fração (numerador, denominador).
    #[inline]
    pub fn mul_fraction(self, num: i32, den: i32) -> Self {
        self.mul_div(num, den)
    }

    /// Multiplica duas grandezas `LayoutUnit` preservando a escala (dividindo pela base `UNITS_PER_PIXEL`).
    #[inline]
    pub fn mul_layout_unit(self, other: Self) -> Self {
        let result = (self.0 as i64 * other.0 as i64) / UNITS_PER_PIXEL as i64;
        let clamped = result.clamp(i32::MIN as i64, i32::MAX as i64);
        Self(clamped as i32)
    }

    /// Divide duas grandezas `LayoutUnit` retornando a razão como `f32`.
    #[inline]
    pub fn div_layout_unit(self, other: Self) -> f32 {
        if other.0 == 0 {
            if self.0 == 0 {
                f32::NAN
            } else if self.0 > 0 {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            }
        } else {
            self.0 as f32 / other.0 as f32
        }
    }

    /// Divisão inteira saturada protegida contra divisão por zero e overflow (`i32::MIN / -1`).
    #[inline]
    pub const fn saturating_div(self, rhs: i32) -> Self {
        if rhs == 0 {
            if self.0 >= 0 {
                MAX
            } else {
                MIN
            }
        } else if self.0 == i32::MIN && rhs == -1 {
            MAX
        } else {
            Self(self.0 / rhs)
        }
    }

    /// Adição com verificação de overflow (retorna `None` se estourar).
    #[inline]
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.0.checked_add(rhs.0) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Subtração com verificação de underflow (retorna `None` se estourar).
    #[inline]
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.0.checked_sub(rhs.0) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Multiplicação com verificação de overflow (retorna `None` se estourar).
    #[inline]
    pub const fn checked_mul(self, factor: i32) -> Option<Self> {
        match self.0.checked_mul(factor) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Divisão com verificação de divisão por zero e overflow.
    #[inline]
    pub const fn checked_div(self, rhs: i32) -> Option<Self> {
        if rhs == 0 || (self.0 == i32::MIN && rhs == -1) {
            None
        } else {
            Some(Self(self.0 / rhs))
        }
    }

    /// Executa o Box Snapping (arredondamento de caixa) com eliminação de fendas (pixel cracking).
    ///
    /// Garante algebricamente que para caixas adjacentes B1=(origem, tam1) e B2=(origem+tam1, tam2),
    /// a borda direita de B1 coincidirá exatamente com a borda esquerda de B2 (`right_1 == left_2`).
    ///
    /// Retorna `(snapped_origin, snapped_size)`.
    #[inline]
    pub fn snap_box(self, size: Self) -> (i32, i32) {
        snap_box(self, size)
    }
}

/// Executa o Box Snapping de uma dimensão 1D (origem e tamanho) garantindo zero pixel cracking.
///
/// Fórmula:
/// - `snapped_origin = round(origin)`
/// - `snapped_size = round(origin + size) - round(origin)`
#[inline]
pub fn snap_box(origin: LayoutUnit, size: LayoutUnit) -> (i32, i32) {
    let snapped_origin = origin.round_px();
    let snapped_end = (origin + size).round_px();
    let snapped_size = snapped_end - snapped_origin;
    (snapped_origin, snapped_size)
}

impl Add for LayoutUnit {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self.saturating_add(rhs)
    }
}

impl AddAssign for LayoutUnit {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for LayoutUnit {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.saturating_sub(rhs)
    }
}

impl SubAssign for LayoutUnit {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<i32> for LayoutUnit {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: i32) -> Self::Output {
        self.saturating_mul(rhs)
    }
}

impl MulAssign<i32> for LayoutUnit {
    #[inline]
    fn mul_assign(&mut self, rhs: i32) {
        *self = *self * rhs;
    }
}

impl Div<i32> for LayoutUnit {
    type Output = Self;

    #[inline]
    fn div(self, rhs: i32) -> Self::Output {
        self.saturating_div(rhs)
    }
}

impl DivAssign<i32> for LayoutUnit {
    #[inline]
    fn div_assign(&mut self, rhs: i32) {
        *self = self.saturating_div(rhs);
    }
}

impl Neg for LayoutUnit {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self(self.0.saturating_neg())
    }
}

impl Sum for LayoutUnit {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(ZERO, |acc, x| acc + x)
    }
}

impl fmt::Debug for LayoutUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LayoutUnit({:.2}px [{}au])", self.to_f32_px(), self.0)
    }
}

impl fmt::Display for LayoutUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}px", self.to_f32_px())
    }
}

impl From<i32> for LayoutUnit {
    #[inline]
    fn from(v: i32) -> Self {
        Self::from_px(v)
    }
}

impl From<f32> for LayoutUnit {
    #[inline]
    fn from(v: f32) -> Self {
        Self::from_f32_px(v)
    }
}

impl From<LayoutUnit> for f32 {
    #[inline]
    fn from(v: LayoutUnit) -> Self {
        v.to_f32_px()
    }
}

impl From<LayoutUnit> for i32 {
    #[inline]
    fn from(v: LayoutUnit) -> Self {
        v.round_px()
    }
}
