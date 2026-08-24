//! # Geometria Lógica e Modos de Escrita (W3C CSS Writing Modes Level 3)
//!
//! Abstrações geométricas em coordenadas de fluxo (*Inline* e *Block*) para suporte
//! completo a textos multidirecionais (LTR, RTL, Vertical CJK, Mongol) e CSS Box Model moderno.

use crate::math::layout_unit::LayoutUnit;
use crate::math::units::LayoutPixel;
use euclid::{Point2D, Rect, SideOffsets2D, Size2D};

use std::fmt;
use std::ops::{Add, Sub};

/// Direção do fluxo em linha (*Inline Base Direction*).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    /// Da esquerda para a direita (padrão em línguas ocidentais / latinas).
    #[default]
    Ltr,
    /// Da direita para a esquerda (árabe, hebraico, farsi).
    Rtl,
}

/// Modo de escrita e orientação de blocos (*CSS Writing Mode*).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WritingMode {
    /// Horizontal de cima para baixo (padrão web).
    #[default]
    HorizontalTb,
    /// Vertical com linhas fluindo da direita para a esquerda (Japonês/Chinês tradicional).
    VerticalRl,
    /// Vertical com linhas fluindo da esquerda para a direita (Mongol).
    VerticalLr,
    /// Rotação lateral de 90° no sentido horário.
    SidewaysRl,
    /// Rotação lateral de 90° no sentido anti-horário.
    SidewaysLr,
}

impl WritingMode {
    /// Retorna `true` se o modo de escrita for horizontal.
    #[inline]
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::HorizontalTb)
    }

    /// Retorna `true` se o modo de escrita for vertical.
    #[inline]
    pub const fn is_vertical(self) -> bool {
        !self.is_horizontal()
    }
}

/// Ponto em coordenadas lógicas (eixo inline e eixo block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LogicalPoint<T> {
    /// Posição ao longo do eixo em linha (*Inline Axis*).
    pub inline: T,
    /// Posição ao longo do eixo de bloco (*Block Axis*).
    pub block: T,
}

impl<T> LogicalPoint<T> {
    /// Cria um novo ponto lógico.
    #[inline]
    pub const fn new(inline: T, block: T) -> Self {
        Self { inline, block }
    }
}

/// Dimensões em coordenadas lógicas (tamanho inline e tamanho block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LogicalSize<T> {
    /// Extensão ao longo do eixo em linha (largura em horizontal, altura em vertical).
    pub inline_size: T,
    /// Extensão ao longo do eixo de bloco (altura em horizontal, largura em vertical).
    pub block_size: T,
}

impl<T> LogicalSize<T> {
    /// Cria uma nova dimensão lógica.
    #[inline]
    pub const fn new(inline_size: T, block_size: T) -> Self {
        Self {
            inline_size,
            block_size,
        }
    }
}

/// Retângulo em coordenadas lógicas (origem e dimensões no fluxo do documento).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LogicalRect<T> {
    /// Início no eixo em linha.
    pub inline_start: T,
    /// Início no eixo de bloco.
    pub block_start: T,
    /// Extensão no eixo em linha.
    pub inline_size: T,
    /// Extensão no eixo de bloco.
    pub block_size: T,
}

impl<T> LogicalRect<T> {
    /// Cria um novo retângulo lógico.
    #[inline]
    pub const fn new(inline_start: T, block_start: T, inline_size: T, block_size: T) -> Self {
        Self {
            inline_start,
            block_start,
            inline_size,
            block_size,
        }
    }

    /// Retorna a origem lógica como `LogicalPoint`.
    #[inline]
    pub fn origin(&self) -> LogicalPoint<T>
    where
        T: Copy,
    {
        LogicalPoint::new(self.inline_start, self.block_start)
    }

    /// Retorna as dimensões lógicas como `LogicalSize`.
    #[inline]
    pub fn size(&self) -> LogicalSize<T>
    where
        T: Copy,
    {
        LogicalSize::new(self.inline_size, self.block_size)
    }
}

/// Espaçamentos dos 4 lados em termos lógicos (margens, bordas, preenchimentos).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LogicalSides<T> {
    pub inline_start: T,
    pub inline_end: T,
    pub block_start: T,
    pub block_end: T,
}

impl<T> LogicalSides<T> {
    /// Cria um conjunto de espaçamentos lógicos com os 4 lados explícitos.
    #[inline]
    pub const fn new(inline_start: T, inline_end: T, block_start: T, block_end: T) -> Self {
        Self {
            inline_start,
            inline_end,
            block_start,
            block_end,
        }
    }

    /// Cria espaçamentos uniformes idênticos em todos os lados.
    #[inline]
    pub fn uniform(value: T) -> Self
    where
        T: Copy,
    {
        Self {
            inline_start: value,
            inline_end: value,
            block_start: value,
            block_end: value,
        }
    }
}

// --- Conversões Bidirecionais: Lógico <-> Físico ---

impl<T: Copy + Add<Output = T> + Sub<Output = T>> LogicalRect<T> {
    /// Converte este retângulo lógico para um retângulo físico (`Rect`), considerando
    /// o modo de escrita, a direção e o tamanho do contêiner pai.
    pub fn to_physical<U>(
        &self,
        wm: WritingMode,
        dir: Direction,
        container_size: Size2D<T, U>,
    ) -> Rect<T, U> {
        match wm {
            WritingMode::HorizontalTb => {
                let x = if dir == Direction::Ltr {
                    self.inline_start
                } else {
                    container_size.width - self.inline_start - self.inline_size
                };
                let y = self.block_start;
                Rect::new(
                    Point2D::new(x, y),
                    Size2D::new(self.inline_size, self.block_size),
                )
            }
            WritingMode::VerticalRl | WritingMode::SidewaysRl => {
                let x = container_size.width - self.block_start - self.block_size;
                let y = if dir == Direction::Ltr {
                    self.inline_start
                } else {
                    container_size.height - self.inline_start - self.inline_size
                };
                Rect::new(
                    Point2D::new(x, y),
                    Size2D::new(self.block_size, self.inline_size),
                )
            }
            WritingMode::VerticalLr | WritingMode::SidewaysLr => {
                let x = self.block_start;
                let y = if dir == Direction::Ltr {
                    self.inline_start
                } else {
                    container_size.height - self.inline_start - self.inline_size
                };
                Rect::new(
                    Point2D::new(x, y),
                    Size2D::new(self.block_size, self.inline_size),
                )
            }
        }
    }

    /// Converte um retângulo físico (`Rect`) para o espaço lógico correspondente.
    pub fn from_physical<U>(
        rect: Rect<T, U>,
        wm: WritingMode,
        dir: Direction,
        container_size: Size2D<T, U>,
    ) -> Self {
        match wm {
            WritingMode::HorizontalTb => {
                let inline_start = if dir == Direction::Ltr {
                    rect.origin.x
                } else {
                    container_size.width - rect.origin.x - rect.size.width
                };
                Self {
                    inline_start,
                    block_start: rect.origin.y,
                    inline_size: rect.size.width,
                    block_size: rect.size.height,
                }
            }
            WritingMode::VerticalRl | WritingMode::SidewaysRl => {
                let block_start = container_size.width - rect.origin.x - rect.size.width;
                let inline_start = if dir == Direction::Ltr {
                    rect.origin.y
                } else {
                    container_size.height - rect.origin.y - rect.size.height
                };
                Self {
                    inline_start,
                    block_start,
                    inline_size: rect.size.height,
                    block_size: rect.size.width,
                }
            }
            WritingMode::VerticalLr | WritingMode::SidewaysLr => {
                let block_start = rect.origin.x;
                let inline_start = if dir == Direction::Ltr {
                    rect.origin.y
                } else {
                    container_size.height - rect.origin.y - rect.size.height
                };
                Self {
                    inline_start,
                    block_start,
                    inline_size: rect.size.height,
                    block_size: rect.size.width,
                }
            }
        }
    }
}

impl<T: Copy> LogicalSides<T> {
    /// Converte espaçamentos lógicos em espaçamentos físicos (`SideOffsets2D`).
    pub fn to_physical<U>(&self, wm: WritingMode, dir: Direction) -> SideOffsets2D<T, U> {
        match wm {
            WritingMode::HorizontalTb => {
                let (left, right) = if dir == Direction::Ltr {
                    (self.inline_start, self.inline_end)
                } else {
                    (self.inline_end, self.inline_start)
                };
                SideOffsets2D::new(self.block_start, right, self.block_end, left)
            }
            WritingMode::VerticalRl | WritingMode::SidewaysRl => {
                let (top, bottom) = if dir == Direction::Ltr {
                    (self.inline_start, self.inline_end)
                } else {
                    (self.inline_end, self.inline_start)
                };
                SideOffsets2D::new(top, self.block_start, bottom, self.block_end)
            }
            WritingMode::VerticalLr | WritingMode::SidewaysLr => {
                let (top, bottom) = if dir == Direction::Ltr {
                    (self.inline_start, self.inline_end)
                } else {
                    (self.inline_end, self.inline_start)
                };
                SideOffsets2D::new(top, self.block_end, bottom, self.block_start)
            }
        }
    }
}

// --- Aliases de Alta Ergonomia para Layout ---

pub type LayoutPoint = Point2D<LayoutUnit, LayoutPixel>;
pub type LayoutSize = Size2D<LayoutUnit, LayoutPixel>;
pub type LayoutRect = Rect<LayoutUnit, LayoutPixel>;
pub type LayoutEdgeInsets = SideOffsets2D<LayoutUnit, LayoutPixel>;

pub type LogicalLayoutPoint = LogicalPoint<LayoutUnit>;
pub type LogicalLayoutSize = LogicalSize<LayoutUnit>;
pub type LogicalLayoutRect = LogicalRect<LayoutUnit>;
pub type LogicalLayoutSides = LogicalSides<LayoutUnit>;

pub type CssLogicalPoint = LogicalPoint<f32>;
pub type CssLogicalSize = LogicalSize<f32>;
pub type CssLogicalRect = LogicalRect<f32>;
pub type CssLogicalSides = LogicalSides<f32>;

impl<T: fmt::Display> fmt::Display for LogicalRect<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LogicalRect(i_start: {}, b_start: {}, i_size: {}, b_size: {})",
            self.inline_start, self.block_start, self.inline_size, self.block_size
        )
    }
}
