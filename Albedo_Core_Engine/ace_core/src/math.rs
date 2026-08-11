// ============================================================================
// Albedo Core Engine (ACE)
// File: math.rs
// Description: Motor Matemático Geométrico e Computação Espacial sem
//              dependência de terceiros, essencial para BFC/IFC e Layout.
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Fundação Matemática
//! 
//! O motor de renderização necessita processar caixas, vetores espaciais (GPU futuro), e cores
//! constantemente. Esta biblioteca central substitui crates genéricas, mantendo o mínimo 
//! overhead absoluto (zero alocações dinâmicas).

use std::ops::{Add, Sub};

// ----------------------------------------------------------------------------
// 2D Spatial Primitives
// ----------------------------------------------------------------------------

/// Representa uma coordenada genérica no Espaço 2D (ViewPort ou Raster).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

impl<T> Point<T> {
    /// Construtor canônico.
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

/// Representação das dimensões bidimensionais de um bloco de conteúdo (Box Model).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl<T> Size<T> {
    /// Construtor canônico.
    pub fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

/// Bounding Box Fundamental (Rect). 
///
/// Define as coordenadas absolutas na hierarquia visual do navegador. 
/// Vital para algoritmos de Layout, Intersection Observer e Rasterização.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect<T> {
    pub origin: Point<T>,
    pub size: Size<T>,
}

impl<T: Add<Output = T> + Copy + PartialOrd> Rect<T> {
    /// Instancia a Bounding Box fornecendo X, Y, Largura e Altura originais.
    pub fn new(x: T, y: T, width: T, height: T) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    #[inline]
    pub fn x(&self) -> T { self.origin.x }
    
    #[inline]
    pub fn y(&self) -> T { self.origin.y }
    
    #[inline]
    pub fn width(&self) -> T { self.size.width }
    
    #[inline]
    pub fn height(&self) -> T { self.size.height }

    #[inline]
    pub fn right(&self) -> T { self.x() + self.width() }
    
    #[inline]
    pub fn bottom(&self) -> T { self.y() + self.height() }
    
    /// Computa algoritmicamente se a coordenada provida encontra-se
    /// no domínio espacial desta Bounding Box (Hit-Testing de Mouse).
    pub fn contains(&self, p: &Point<T>) -> bool {
        p.x >= self.x() && p.x <= self.right() &&
        p.y >= self.y() && p.y <= self.bottom()
    }
    
    /// Determina sobreposição matemática (Collision/Intersect).
    /// Crítico para o algoritmo de Painter's (descarte de componentes fora da tela).
    pub fn intersects(&self, other: &Rect<T>) -> bool {
        self.x() < other.right() && self.right() > other.x() &&
        self.y() < other.bottom() && self.bottom() > other.y()
    }
}

// ----------------------------------------------------------------------------
// Vectorial Primitives
// ----------------------------------------------------------------------------

/// Vetor Direcional (Velocidade, Força, Interpolação de Animações JS).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

impl<T> Vec2<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T: Add<Output = T>> Add for Vec2<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl<T: Sub<Output = T>> Sub for Vec2<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

// ----------------------------------------------------------------------------
// Colorimetry (Grafos & Pintura)
// ----------------------------------------------------------------------------

/// Modelo de Cores em bytes não alocados (RGBA/32bits primitivos).
/// Otimizado para alinhamento e upload na VRAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    
    /// Construtor auxiliar simplificado sem canal de opacidade (Alpha = 255).
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }
}

// ----------------------------------------------------------------------------
// Mathematical Core Utilities
// ----------------------------------------------------------------------------

/// Restringe puramente um valor escalar matemático aos limites exigidos.
#[inline]
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min { min } else if value > max { max } else { value }
}

/// (L)inear Int(erp)olation. Computa animações vetoriais, transições
/// no Event Loop e comportamentos flexíveis de renderização (T fixado entre 0.0 e 1.0).
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * clamp(t, 0.0, 1.0)
}

// ----------------------------------------------------------------------------
// 3D & 4D Vectors
// ----------------------------------------------------------------------------

/// Vetor 3D (Espaço Tridimensional)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> Vec3<T> {
    pub fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

impl<T: Add<Output = T>> Add for Vec3<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl<T: Sub<Output = T>> Sub for Vec3<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

/// Vetor Homogêneo 4D (Computação Gráfica / WebGL)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec4<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

impl<T> Vec4<T> {
    pub fn new(x: T, y: T, z: T, w: T) -> Self {
        Self { x, y, z, w }
    }
}

// ----------------------------------------------------------------------------
// Matrices
// ----------------------------------------------------------------------------

/// Matriz de Transformação 3x3 (Rotação, Escala, Cisalhamento em 2D)
/// Organizada em Column-Major format (padrão WebGL).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3x3<T> {
    pub data: [T; 9],
}

impl<T: Default + Copy> Default for Matrix3x3<T> {
    fn default() -> Self {
        Self { data: [T::default(); 9] }
    }
}

impl<T: Default + Copy> Matrix3x3<T> {
    pub fn new(data: [T; 9]) -> Self {
        Self { data }
    }
}

/// Matriz de Transformação Homogênea 4x4 (Projeção e Transformação 3D).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix4x4<T> {
    pub data: [T; 16],
}

impl<T: Default + Copy> Default for Matrix4x4<T> {
    fn default() -> Self {
        Self { data: [T::default(); 16] }
    }
}

impl<T: Default + Copy> Matrix4x4<T> {
    pub fn new(data: [T; 16]) -> Self {
        Self { data }
    }
}

