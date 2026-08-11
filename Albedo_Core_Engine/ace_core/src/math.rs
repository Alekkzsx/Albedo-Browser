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

use std::ops::{Add, Sub, Mul};

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

impl<T: Add<Output = T> + Sub<Output = T> + Copy + PartialOrd> Rect<T> {
    /// Encontra o menor retângulo que engloba ambos.
    pub fn union(&self, other: &Rect<T>) -> Self {
        let min_x = if self.x() < other.x() { self.x() } else { other.x() };
        let min_y = if self.y() < other.y() { self.y() } else { other.y() };
        let max_r = if self.right() > other.right() { self.right() } else { other.right() };
        let max_b = if self.bottom() > other.bottom() { self.bottom() } else { other.bottom() };
        Self::new(min_x, min_y, max_r - min_x, max_b - min_y)
    }

    /// Encontra a sobreposição geométrica, se houver.
    pub fn intersection(&self, other: &Rect<T>) -> Option<Self> {
        let max_x = if self.x() > other.x() { self.x() } else { other.x() };
        let max_y = if self.y() > other.y() { self.y() } else { other.y() };
        let min_r = if self.right() < other.right() { self.right() } else { other.right() };
        let min_b = if self.bottom() < other.bottom() { self.bottom() } else { other.bottom() };
        
        if max_x < min_r && max_y < min_b {
            Some(Self::new(max_x, max_y, min_r - max_x, min_b - max_y))
        } else {
            None
        }
    }

    /// Expande o retângulo a partir do centro.
    pub fn inflate(&self, dx: T, dy: T) -> Self {
        let double_dx = dx + dx;
        let double_dy = dy + dy;
        Self::new(self.x() - dx, self.y() - dy, self.width() + double_dx, self.height() + double_dy)
    }

    /// Encolhe o retângulo a partir do centro.
    pub fn deflate(&self, dx: T, dy: T) -> Self {
        let double_dx = dx + dx;
        let double_dy = dy + dy;
        // Evitar erro matemático se o encolhimento ultrapassar as bordas:
        // No ACE simplificamos confiando no chamador ou gerando geometria invertida.
        Self::new(self.x() + dx, self.y() + dy, self.width() - double_dx, self.height() - double_dy)
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

#[inline]
pub fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

#[inline]
pub fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

#[inline]
pub fn abs(a: f32) -> f32 {
    if a < 0.0 { -a } else { a }
}

#[inline]
pub fn saturate(a: f32) -> f32 {
    clamp(a, 0.0, 1.0)
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

impl<T: Mul<Output = T> + Copy> Mul<T> for Vec2<T> {
    type Output = Self;
    fn mul(self, scalar: T) -> Self {
        Self::new(self.x * scalar, self.y * scalar)
    }
}

impl Vec2<f32> {
    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    pub fn length_squared(&self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn distance(&self, other: &Self) -> f32 {
        (*self - *other).length()
    }

    #[inline]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            *self * (1.0 / len)
        } else {
            *self
        }
    }
}

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

impl<T: Mul<Output = T> + Copy> Mul<T> for Vec3<T> {
    type Output = Self;
    fn mul(self, scalar: T) -> Self {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }
}

impl Vec3<f32> {
    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn cross(&self, other: &Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    #[inline]
    pub fn length_squared(&self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn distance(&self, other: &Self) -> f32 {
        (*self - *other).length()
    }

    #[inline]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            *self * (1.0 / len)
        } else {
            *self
        }
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

impl<T: Add<Output = T>> Add for Vec4<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z, self.w + other.w)
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

impl Matrix3x3<f32> {
    pub fn identity() -> Self {
        Self::new([
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ])
    }

    pub fn translation(tx: f32, ty: f32) -> Self {
        Self::new([
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            tx,  ty,  1.0,
        ])
    }

    pub fn scale(sx: f32, sy: f32) -> Self {
        Self::new([
            sx,  0.0, 0.0,
            0.0, sy,  0.0,
            0.0, 0.0, 1.0,
        ])
    }

    pub fn rotation(radians: f32) -> Self {
        let c = radians.cos();
        let s = radians.sin();
        Self::new([
            c,   s,   0.0,
            -s,  c,   0.0,
            0.0, 0.0, 1.0,
        ])
    }

    pub fn multiply_vec2(&self, v: &Vec2<f32>) -> Vec2<f32> {
        let x = v.x * self.data[0] + v.y * self.data[3] + self.data[6];
        let y = v.x * self.data[1] + v.y * self.data[4] + self.data[7];
        Vec2::new(x, y)
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

impl Matrix4x4<f32> {
    pub fn identity() -> Self {
        Self::new([
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let r_l = right - left;
        let t_b = top - bottom;
        let f_n = far - near;

        Self::new([
            2.0 / r_l, 0.0,       0.0,        0.0,
            0.0,       2.0 / t_b, 0.0,        0.0,
            0.0,       0.0,       -2.0 / f_n, 0.0,
            -(right + left) / r_l, -(top + bottom) / t_b, -(far + near) / f_n, 1.0,
        ])
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

    /// Retorna (H, S, L, A) onde H em [0, 360), S, L, A em [0.0, 1.0]
    pub fn to_hsla(&self) -> (f32, f32, f32, f32) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;
        let a = self.a as f32 / 255.0;

        let max = max(max(r, g), b);
        let min = min(min(r, g), b);
        
        let l = (max + min) / 2.0;
        let mut h = 0.0;
        let mut s = 0.0;

        if max != min {
            let d = max - min;
            s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
            
            if max == r {
                h = (g - b) / d + (if g < b { 6.0 } else { 0.0 });
            } else if max == g {
                h = (b - r) / d + 2.0;
            } else if max == b {
                h = (r - g) / d + 4.0;
            }
            h /= 6.0;
        }

        (h * 360.0, s, l, a)
    }

    /// Constrói uma cor a partir de HSL (A assumido como 1.0)
    pub fn from_hsla(h: f32, s: f32, l: f32, a: f32) -> Self {
        let mut r = l;
        let mut g = l;
        let mut b = l;

        if s != 0.0 {
            let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
            let p = 2.0 * l - q;
            let h_normalized = h / 360.0;

            r = Self::hue_to_rgb(p, q, h_normalized + 1.0 / 3.0);
            g = Self::hue_to_rgb(p, q, h_normalized);
            b = Self::hue_to_rgb(p, q, h_normalized - 1.0 / 3.0);
        }

        Self::new(
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
            (a * 255.0).round() as u8,
        )
    }

    fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
        if t < 0.0 { t += 1.0; }
        if t > 1.0 { t -= 1.0; }
        if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
        if t < 1.0 / 2.0 { return q; }
        if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
        p
    }

    pub fn darken(&self, amount: f32) -> Self {
        let (h, s, mut l, a) = self.to_hsla();
        l = saturate(l - amount);
        Self::from_hsla(h, s, l, a)
    }

    pub fn lighten(&self, amount: f32) -> Self {
        let (h, s, mut l, a) = self.to_hsla();
        l = saturate(l + amount);
        Self::from_hsla(h, s, l, a)
    }

    pub fn blend_source_over(&self, bg: &Color) -> Color {
        let fg_a = self.a as f32 / 255.0;
        let bg_a = bg.a as f32 / 255.0;
        
        let out_a = fg_a + bg_a * (1.0 - fg_a);
        if out_a == 0.0 { return Color::new(0,0,0,0); }

        let out_r = ((self.r as f32 * fg_a) + (bg.r as f32 * bg_a * (1.0 - fg_a))) / out_a;
        let out_g = ((self.g as f32 * fg_a) + (bg.g as f32 * bg_a * (1.0 - fg_a))) / out_a;
        let out_b = ((self.b as f32 * fg_a) + (bg.b as f32 * bg_a * (1.0 - fg_a))) / out_a;

        Color::new(out_r.round() as u8, out_g.round() as u8, out_b.round() as u8, (out_a * 255.0).round() as u8)
    }

    pub fn blend_multiply(&self, bg: &Color) -> Color {
        Color::new(
            ((self.r as u16 * bg.r as u16) / 255) as u8,
            ((self.g as u16 * bg.g as u16) / 255) as u8,
            ((self.b as u16 * bg.b as u16) / 255) as u8,
            max(self.a, bg.a)
        )
    }

    pub fn blend_screen(&self, bg: &Color) -> Color {
        Color::new(
            (255 - ((255 - self.r as u16) * (255 - bg.r as u16)) / 255) as u8,
            (255 - ((255 - self.g as u16) * (255 - bg.g as u16)) / 255) as u8,
            (255 - ((255 - self.b as u16) * (255 - bg.b as u16)) / 255) as u8,
            max(self.a, bg.a)
        )
    }
}
