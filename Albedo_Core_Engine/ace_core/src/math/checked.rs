//! # Aritmética Segura Contra Overflow (WebKit CheckedArithmetic & Chromium CheckedNumeric)
//!
//! Wrapper genérico para cálculos numéricos seguros com propagação de erros (*Sticky Error*),
//! prevenindo vulnerabilidades de *Integer Overflow* e *Buffer Overflow* ao lidar com dados da web.

use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

/// Wrapper para cálculos numéricos com semântica de *Sticky Error*.
///
/// Se qualquer operação aritmética sofrer overflow, underflow ou divisão por zero,
/// o estado interno se torna `None` e todas as operações subsequentes mantêm o estado inválido.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Checked<T> {
    value: Option<T>,
}

impl<T> Checked<T> {
    /// Cria um novo valor checked válido.
    #[inline]
    pub const fn new(val: T) -> Self {
        Self { value: Some(val) }
    }

    /// Cria um estado inválido explicitamente (resultado de overflow prévio).
    #[inline]
    pub const fn invalid() -> Self {
        Self { value: None }
    }

    /// Retorna `true` se o cálculo foi concluído com sucesso sem nenhum overflow ou divisão inválida.
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.value.is_some()
    }

    /// Extrai o valor interno como `Option<T>`.
    #[inline]
    pub const fn value(&self) -> Option<T>
    where
        T: Copy,
    {
        self.value
    }

    /// Retorna o valor ou um padrão caso tenha ocorrido overflow.
    #[inline]
    pub fn value_or(&self, default: T) -> T
    where
        T: Copy,
    {
        self.value.unwrap_or(default)
    }
}

impl Checked<usize> {
    /// Retorna o valor ou `usize::MAX` em caso de overflow (útil para alocações seguras).
    #[inline]
    pub fn value_or_max(&self) -> usize {
        self.value.unwrap_or(usize::MAX)
    }
}

impl<T: Copy> Checked<T> {
    /// Converte o tipo interno verificando se o valor cabe no tipo de destino `U`.
    pub fn cast<U: TryFrom<T>>(self) -> Checked<U> {
        match self.value {
            Some(val) => match U::try_from(val) {
                Ok(converted) => Checked::new(converted),
                Err(_) => Checked::invalid(),
            },
            None => Checked::invalid(),
        }
    }
}

// Macro para implementar operações aritméticas checked em tipos primitivos
macro_rules! impl_checked_ops {
    ($($t:ty),*) => {
        $(
            impl Add for Checked<$t> {
                type Output = Self;

                #[inline]
                fn add(self, rhs: Self) -> Self::Output {
                    match (self.value, rhs.value) {
                        (Some(a), Some(b)) => match a.checked_add(b) {
                            Some(res) => Self::new(res),
                            None => Self::invalid(),
                        },
                        _ => Self::invalid(),
                    }
                }
            }

            impl Add<$t> for Checked<$t> {
                type Output = Self;

                #[inline]
                fn add(self, rhs: $t) -> Self::Output {
                    self + Checked::new(rhs)
                }
            }

            impl Sub for Checked<$t> {
                type Output = Self;

                #[inline]
                fn sub(self, rhs: Self) -> Self::Output {
                    match (self.value, rhs.value) {
                        (Some(a), Some(b)) => match a.checked_sub(b) {
                            Some(res) => Self::new(res),
                            None => Self::invalid(),
                        },
                        _ => Self::invalid(),
                    }
                }
            }

            impl Sub<$t> for Checked<$t> {
                type Output = Self;

                #[inline]
                fn sub(self, rhs: $t) -> Self::Output {
                    self - Checked::new(rhs)
                }
            }

            impl Mul for Checked<$t> {
                type Output = Self;

                #[inline]
                fn mul(self, rhs: Self) -> Self::Output {
                    match (self.value, rhs.value) {
                        (Some(a), Some(b)) => match a.checked_mul(b) {
                            Some(res) => Self::new(res),
                            None => Self::invalid(),
                        },
                        _ => Self::invalid(),
                    }
                }
            }

            impl Mul<$t> for Checked<$t> {
                type Output = Self;

                #[inline]
                fn mul(self, rhs: $t) -> Self::Output {
                    self * Checked::new(rhs)
                }
            }

            impl Div for Checked<$t> {
                type Output = Self;

                #[inline]
                fn div(self, rhs: Self) -> Self::Output {
                    match (self.value, rhs.value) {
                        (Some(a), Some(b)) => {
                            if b == 0 {
                                Self::invalid()
                            } else {
                                match a.checked_div(b) {
                                    Some(res) => Self::new(res),
                                    None => Self::invalid(),
                                }
                            }
                        }
                        _ => Self::invalid(),
                    }
                }
            }

            impl Div<$t> for Checked<$t> {
                type Output = Self;

                #[inline]
                fn div(self, rhs: $t) -> Self::Output {
                    self / Checked::new(rhs)
                }
            }

            impl From<$t> for Checked<$t> {
                #[inline]
                fn from(val: $t) -> Self {
                    Self::new(val)
                }
            }
        )*
    };
}

impl_checked_ops!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

/// Alias para cálculos de alocação de buffer e capacidade em bytes.
pub type CheckedSize = Checked<usize>;

impl<T: fmt::Debug> fmt::Debug for Checked<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            Some(v) => write!(f, "Checked({:?})", v),
            None => write!(f, "Checked(Invalid)"),
        }
    }
}

impl<T: fmt::Display> fmt::Display for Checked<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            Some(v) => write!(f, "{}", v),
            None => write!(f, "Invalid"),
        }
    }
}
