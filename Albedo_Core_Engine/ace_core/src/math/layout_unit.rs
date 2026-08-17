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
        Self(self.0 / rhs)
    }
}

impl DivAssign<i32> for LayoutUnit {
    #[inline]
    fn div_assign(&mut self, rhs: i32) {
        self.0 /= rhs;
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
