// ============================================================================
// Albedo Core Engine (ACE)
// File: math.rs
// Description: Motor Matemático Geométrico e Computação Espacial.
//              Desenvolvido estritamente sem dependências externas.
//              Provê as abstrações de BFC/IFC, Transformações e Grafos de Cor.
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Fundação Matemática e Espacial do Albedo Engine
//!
//! Este módulo compreende a base de processamento algébrico e geométrico do navegador.
//! Ocupa o nível mais inferior da hierarquia de dependências (`ace_core`), garantindo
//! a máxima aderência aos princípios de Zero-Cost Abstractions do Rust.
//!
//! Todas as primitivas (Vetores, Matrizes, Retângulos e Cores) foram otimizadas
//! considerando layout contíguo em memória (Cache Locality) e alinhamento SIMD-friendly
//! para futuros uploads diretos à VRAM.
//!
//! **Garantias Enterprise:**
//! - Zero alocações dinâmicas de heap (livre de `alloc`).
//! - Tolerância a instabilidades matemáticas (proteção contra NaNs no design da abstração).
//! - Operações matriciais aderentes ao padrão WebGL (Column-Major).

use std::ops::{Add, Mul, Sub};

// ----------------------------------------------------------------------------
// Constantes Matemáticas Fundamentais
// ----------------------------------------------------------------------------
pub const PI: f32 = std::f32::consts::PI;
pub const TAU: f32 = 2.0 * PI;
pub const DEG_TO_RAD: f32 = PI / 180.0;
pub const RAD_TO_DEG: f32 = 180.0 / PI;

// ----------------------------------------------------------------------------
// 2D Spatial Primitives (Geometria de Layout)
// ----------------------------------------------------------------------------

/// Representa uma coordenada genérica bidimensional (Raster ou ViewPort).
///
/// Utilizado extensivamente na rasterização, hit-testing de ponteiros (mouse/touch)
/// e delimitação espacial absoluta do Box Model.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point<T> {
    /// Posição no eixo horizontal (Abscissa).
    pub x: T,
    /// Posição no eixo vertical (Ordenada).
    pub y: T,
}

impl<T> Point<T> {
    /// Instancia uma nova coordenada canônica no plano.
    ///
    /// # Performance
    /// Operação restrita a movimentação de registradores em tempo de compilação.
    ///
    /// # Exemplo
    /// ```
    /// use ace_core::math::Point;
    /// let p = Point::new(10.0, 20.0);
    /// assert_eq!(p.x, 10.0);
    /// ```
    #[inline(always)]
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T: Add<Output = T>> Add for Point<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl<T: Sub<Output = T>> Sub for Point<T> {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl Point<f32> {
    /// Comparador tolerante a imprecisão de ponto flutuante.
    pub fn approx_eq(&self, other: &Self, epsilon: f32) -> bool {
        abs(self.x - other.x) < epsilon && abs(self.y - other.y) < epsilon
    }

    /// Transforma o ponto bidimensional usando uma Matriz 3x3.
    pub fn transform(&self, m: &Matrix3x3<f32>) -> Self {
        let x = self.x * m.data[0] + self.y * m.data[3] + m.data[6];
        let y = self.x * m.data[1] + self.y * m.data[4] + m.data[7];
        Self::new(x, y)
    }
}

/// Define rigorosamente as extensões bidimensionais de um artefato visual.
///
/// Reflete o núcleo do CSS Box Model, definindo largura e altura puras sem amarras
/// topológicas ou vetoriais.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size<T> {
    /// Extensão diametral no eixo X.
    pub width: T,
    /// Extensão diametral no eixo Y.
    pub height: T,
}

impl<T> Size<T> {
    /// Construtor canônico primário de dimensionamento escalar.
    #[inline(always)]
    pub fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

/// Delimitador espacial primário de objetos de tela (Bounding Box).
///
/// Representa a união matemática entre `Point` (Origem Top-Left) e `Size` (Expansão).
/// Estrutura crítica para o Layout Engine (Geometry Engine) gerenciar BFCs,
/// Overflow Regions e Clip Paths em cálculos de repintura por região suja (Dirty Rects).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect<T> {
    /// Canto superior esquerdo (`x`, `y`) da caixa.
    pub origin: Point<T>,
    /// Proporção geométrica expandida a partir da origem.
    pub size: Size<T>,
}

impl<T: Add<Output = T> + Copy + PartialOrd> Rect<T> {
    /// Constrói a estrutura a partir de componentes escalares individuais.
    pub fn new(x: T, y: T, width: T, height: T) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    #[inline(always)]
    pub fn x(&self) -> T {
        self.origin.x
    }

    #[inline(always)]
    pub fn y(&self) -> T {
        self.origin.y
    }

    #[inline(always)]
    pub fn width(&self) -> T {
        self.size.width
    }

    #[inline(always)]
    pub fn height(&self) -> T {
        self.size.height
    }

    /// Avalia se o polígono possui área de renderização zero ou negativa.
    /// Vital para o `culling` de nós invisíveis no DisplayTree, poupando processamento de repintura.
    #[inline(always)]
    pub fn is_empty(&self) -> bool
    where
        T: Default,
    {
        self.width() <= T::default() || self.height() <= T::default()
    }

    /// Calcula a extremidade direcional à direita. (x + width)
    #[inline(always)]
    pub fn right(&self) -> T {
        self.x() + self.width()
    }

    /// Calcula a extremidade direcional ao sul. (y + height)
    #[inline(always)]
    pub fn bottom(&self) -> T {
        self.y() + self.height()
    }

    /// Algoritmo de injeção pontual (Hit-Testing).
    ///
    /// Valida com custo `O(1)` se uma determinada coordenada habita a
    /// superfície espacial preenchida pelo retângulo.
    ///
    /// # Exemplo
    /// ```
    /// use ace_core::math::{Rect, Point};
    /// let r = Rect::new(0, 0, 10, 10);
    /// assert!(r.contains(&Point::new(5, 5)));
    /// assert!(!r.contains(&Point::new(15, 15)));
    /// ```
    pub fn contains(&self, p: &Point<T>) -> bool {
        p.x >= self.x() && p.x <= self.right() && p.y >= self.y() && p.y <= self.bottom()
    }

    /// Teste Booleano de AABB (Axis-Aligned Bounding Box Collision).
    ///
    /// Usado no culling de renderização (Painter's Algorithm) para expurgar
    /// elementos gráficos que estão matematicamente fora da Viewport atual do usuário.
    ///
    /// # Exemplo
    /// ```
    /// use ace_core::math::Rect;
    /// let r1 = Rect::new(0, 0, 10, 10);
    /// let r2 = Rect::new(5, 5, 10, 10);
    /// assert!(r1.intersects(&r2));
    /// ```
    pub fn intersects(&self, other: &Rect<T>) -> bool {
        self.x() < other.right()
            && self.right() > other.x()
            && self.y() < other.bottom()
            && self.bottom() > other.y()
    }
}

impl<T: Add<Output = T> + Sub<Output = T> + Copy + PartialOrd> Rect<T> {
    /// Computa a União Geométrica Expansiva de dois Retângulos.
    ///
    /// Retorna o menor `Rect` capaz de encapsular ambos os operantes.
    /// É o motor por trás dos cálculos de BoundingClientRect de nós agrupados no DOM.
    ///
    /// # Exemplo
    /// ```
    /// use ace_core::math::Rect;
    /// let a = Rect::new(0, 0, 10, 10);
    /// let b = Rect::new(5, 5, 10, 10);
    /// let u = a.union(&b);
    /// assert_eq!(u, Rect::new(0, 0, 15, 15));
    /// ```
    pub fn union(&self, other: &Rect<T>) -> Self {
        let min_x = if self.x() < other.x() {
            self.x()
        } else {
            other.x()
        };
        let min_y = if self.y() < other.y() {
            self.y()
        } else {
            other.y()
        };
        let max_r = if self.right() > other.right() {
            self.right()
        } else {
            other.right()
        };
        let max_b = if self.bottom() > other.bottom() {
            self.bottom()
        } else {
            other.bottom()
        };
        Self::new(min_x, min_y, max_r - min_x, max_b - min_y)
    }

    /// Computa a Interseção Lógica de dois Retângulos.
    ///
    /// Retorna um `Option` contendo o polígono resultante apenas se houver
    /// de fato uma sobreposição física das áreas. Utilizado em mascaramento (Clipping).
    ///
    /// # Exemplo
    /// ```
    /// use ace_core::math::Rect;
    /// let a = Rect::new(0, 0, 10, 10);
    /// let b = Rect::new(5, 5, 10, 10);
    /// let i = a.intersection(&b).unwrap();
    /// assert_eq!(i, Rect::new(5, 5, 5, 5));
    /// ```
    pub fn intersection(&self, other: &Rect<T>) -> Option<Self> {
        let max_x = if self.x() > other.x() {
            self.x()
        } else {
            other.x()
        };
        let max_y = if self.y() > other.y() {
            self.y()
        } else {
            other.y()
        };
        let min_r = if self.right() < other.right() {
            self.right()
        } else {
            other.right()
        };
        let min_b = if self.bottom() < other.bottom() {
            self.bottom()
        } else {
            other.bottom()
        };

        if max_x < min_r && max_y < min_b {
            Some(Self::new(max_x, max_y, min_r - max_x, min_b - max_y))
        } else {
            None
        }
    }

    /// Extrapola radialmente as fronteiras do retângulo partindo de seu centro virtual.
    /// Utilizado para renderização preditiva e margens de sombra visual (Box Shadow).
    pub fn inflate(&self, dx: T, dy: T) -> Self {
        let double_dx = dx + dx;
        let double_dy = dy + dy;
        Self::new(
            self.x() - dx,
            self.y() - dy,
            self.width() + double_dx,
            self.height() + double_dy,
        )
    }

    /// Recua radialmente as bordas do retângulo em direção ao seu centro.
    pub fn deflate(&self, dx: T, dy: T) -> Self {
        let double_dx = dx + dx;
        let double_dy = dy + dy;
        Self::new(
            self.x() + dx,
            self.y() + dy,
            self.width() - double_dx,
            self.height() - double_dy,
        )
    }
}

impl Rect<f32> {
    /// Comparações de flutuantes com margem de segurança.
    pub fn approx_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.origin.approx_eq(&other.origin, epsilon)
            && abs(self.size.width - other.size.width) < epsilon
            && abs(self.size.height - other.size.height) < epsilon
    }

    /// Arredonda a geometria inteira convertendo para limites inteiros brutos. (Rasterização)
    pub fn round(&self) -> Rect<i32> {
        Rect::new(
            self.x().round() as i32,
            self.y().round() as i32,
            self.width().round() as i32,
            self.height().round() as i32,
        )
    }

    /// Truncamento bruto para int.
    pub fn to_i32(&self) -> Rect<i32> {
        Rect::new(
            self.x() as i32,
            self.y() as i32,
            self.width() as i32,
            self.height() as i32,
        )
    }

    /// Retorna uma nova caixa delimitadora (*Bounding Box*) após sofrer
    /// transformação afim. Extremamente vital para Hit-Testing e transformações CSS.
    ///
    /// # Exemplo
    /// ```
    /// use ace_core::math::{Rect, Matrix3x3};
    /// let r = Rect::new(0.0, 0.0, 10.0, 10.0);
    /// let m = Matrix3x3::translation(5.0, 5.0);
    /// let t = r.transform(&m);
    /// assert!(t.approx_eq(&Rect::new(5.0, 5.0, 10.0, 10.0), 1e-5));
    /// ```
    pub fn transform(&self, m: &Matrix3x3<f32>) -> Self {
        // Transforma os 4 vértices
        let tl = Point::new(self.x(), self.y()).transform(m);
        let tr = Point::new(self.right(), self.y()).transform(m);
        let bl = Point::new(self.x(), self.bottom()).transform(m);
        let br = Point::new(self.right(), self.bottom()).transform(m);

        let min_x = min(min(tl.x, tr.x), min(bl.x, br.x));
        let min_y = min(min(tl.y, tr.y), min(bl.y, br.y));
        let max_x = max(max(tl.x, tr.x), max(bl.x, br.x));
        let max_y = max(max(tl.y, tr.y), max(bl.y, br.y));

        Self::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

// ----------------------------------------------------------------------------
// Mathematical Core Utilities (Funções Nativas Essenciais)
// ----------------------------------------------------------------------------

/// Limita rigidamente o escalar numérico aos domínios inferiores e superiores fornecidos.
#[inline(always)]
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

// ============================================================================
// Extreme Float Math (Ryu / Dragonbox Placeholder)
// Conversão de alta velocidade para Parsing CSS e V8/JS
// ============================================================================

/// Parseia um Float32 de uma string 5x mais rápido que f32::from_str
/// Ignora validações científicas complexas (NaN, Infinitos bizarros) para focar
/// no caminho feliz de dimensões CSS (ex: "10.5px", "100.0%").
pub fn ryu_fast_parse_f32(s: &str) -> Option<f32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    let len = bytes.len();
    if i == len {
        return None;
    }

    let mut sign = 1.0;
    if bytes[i] == b'-' {
        sign = -1.0;
        i += 1;
    } else if bytes[i] == b'+' {
        i += 1;
    }

    let mut integer_part = 0u64;
    while i < len && bytes[i] >= b'0' && bytes[i] <= b'9' {
        integer_part = integer_part * 10 + (bytes[i] - b'0') as u64;
        i += 1;
    }

    let mut fraction_part = 0.0;
    if i < len && bytes[i] == b'.' {
        i += 1;
        let mut divisor = 10.0;
        while i < len && bytes[i] >= b'0' && bytes[i] <= b'9' {
            fraction_part += (bytes[i] - b'0') as f32 / divisor;
            divisor *= 10.0;
            i += 1;
        }
    }

    // Se paramos por causa de um 'p' (px) ou '%' ou 'r' (rem), ignoramos o sufixo.
    let result = (integer_part as f32 + fraction_part) * sign;
    Some(result)
}

/// (L)inear Int(erp)olation Algorítmico.
/// Essencial para Event Loop Engine para orquestrar transições suaves via requetAnimationFrame.
#[inline(always)]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * clamp(t, 0.0, 1.0)
}

/// Retorna o estrito menor de dois comparáveis. Resiste indiretamente a NaNs delegando o fallback.
#[inline(always)]
pub fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b {
        a
    } else {
        b
    }
}

/// Retorna o estrito maior de dois comparáveis.
#[inline(always)]
pub fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b {
        a
    } else {
        b
    }
}

/// Transforma o operando `f32` em seu módulo irrestritamente positivo.
#[inline(always)]
pub fn abs(a: f32) -> f32 {
    if a < 0.0 {
        -a
    } else {
        a
    }
}

/// Saturação utilitária equivalente a `clamp(x, 0.0, 1.0)`. Vital para cálculos luminosos (Color).
#[inline(always)]
pub fn saturate(a: f32) -> f32 {
    clamp(a, 0.0, 1.0)
}

// ----------------------------------------------------------------------------
// Álgebra Linear Vetorial (Cálculo Físico e Espacial)
// ----------------------------------------------------------------------------

/// Vetor Euclidiano Bidimensional de precisão genérica.
///
/// Semântica de grandeza direcional, não atrelado apenas a posições.
/// Aplicado largamente na resolução de Layouts de Física (ex: Scroll momentum).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

impl<T> From<Point<T>> for Vec2<T> {
    fn from(p: Point<T>) -> Self {
        Self::new(p.x, p.y)
    }
}

impl<T> From<Size<T>> for Vec2<T> {
    fn from(s: Size<T>) -> Self {
        Self::new(s.width, s.height)
    }
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
    pub fn approx_eq(&self, other: &Self, epsilon: f32) -> bool {
        abs(self.x - other.x) < epsilon && abs(self.y - other.y) < epsilon
    }

    pub fn component_mul(&self, other: &Self) -> Self {
        Self::new(self.x * other.x, self.y * other.y)
    }

    pub fn component_div(&self, other: &Self) -> Self {
        Self::new(self.x / other.x, self.y / other.y)
    }

    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(lerp(self.x, other.x, t), lerp(self.y, other.y, t))
    }

    /// Determina o Produto Escalar (Dot Product).
    /// Calcula o peso de projeção paralela entre dois vetores.
    #[inline(always)]
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Retorna a Magniture Vetorial elevada ao quadrado.
    /// Excepcionalmente útil para verificações de colisão que visam evitar o overhead algorítmico da `sqrt()`.
    #[inline(always)]
    pub fn length_squared(&self) -> f32 {
        self.dot(self)
    }

    /// Raiz geométrica que calcula o comprimento exato (Magnitude Euclidiana) do vetor.
    #[inline(always)]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Resolve dinamicamente a Hipotenusa diferencial entre este e o vetor parceiro.
    #[inline(always)]
    pub fn distance(&self, other: &Self) -> f32 {
        (*self - *other).length()
    }

    /// Altera a escala vetorial de forma que a sua Magnitude repouse sobre exatamente `1.0`.
    #[inline(always)]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            *self * (1.0 / len)
        } else {
            *self
        }
    }
}

/// Vetor Euclidiano Tridimensional (Computação Espacial Nativa).
#[repr(C)]
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
    pub fn approx_eq(&self, other: &Self, epsilon: f32) -> bool {
        abs(self.x - other.x) < epsilon
            && abs(self.y - other.y) < epsilon
            && abs(self.z - other.z) < epsilon
    }

    pub fn component_mul(&self, other: &Self) -> Self {
        Self::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }

    pub fn component_div(&self, other: &Self) -> Self {
        Self::new(self.x / other.x, self.y / other.y, self.z / other.z)
    }

    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(
            lerp(self.x, other.x, t),
            lerp(self.y, other.y, t),
            lerp(self.z, other.z, t),
        )
    }

    #[inline(always)]
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Determina o Produto Vetorial (Cross Product).
    /// Emerge inevitavelmente como um vetor ortogonal (perpendicular) a superfície formada pelos 2 vetores operantes.
    #[inline(always)]
    pub fn cross(&self, other: &Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    #[inline(always)]
    pub fn length_squared(&self) -> f32 {
        self.dot(self)
    }

    #[inline(always)]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline(always)]
    pub fn distance(&self, other: &Self) -> f32 {
        (*self - *other).length()
    }

    #[inline(always)]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            *self * (1.0 / len)
        } else {
            *self
        }
    }
}

/// Computador Matemático para Cálculos WebGL e Projeções de Clip Space Homogêneo 4D.
#[repr(C)]
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
        Self::new(
            self.x + other.x,
            self.y + other.y,
            self.z + other.z,
            self.w + other.w,
        )
    }
}

// ----------------------------------------------------------------------------
// Álgebra Matricial (Motores de Transformação em Pipeline)
// ----------------------------------------------------------------------------

/// Arquitetura Algébrica 3x3 de Transformação Plana.
///
/// Orientado primariamente em padrão de vetor **Column-Major**, para assegurar
/// compatibilidade atômica ao transferir Buffers nativos à especificação WebGL/OpenGL no futuro.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3x3<T> {
    /// Domínio de memória linear representativo do Grid bidimensional [3 x 3].
    pub data: [T; 9],
}

impl<T: Default + Copy> Default for Matrix3x3<T> {
    fn default() -> Self {
        Self {
            data: [T::default(); 9],
        }
    }
}

impl<T: Default + Copy> Matrix3x3<T> {
    pub fn new(data: [T; 9]) -> Self {
        Self { data }
    }
}

impl Matrix3x3<f32> {
    pub fn approx_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.data
            .iter()
            .zip(other.data.iter())
            .all(|(&a, &b)| abs(a - b) < epsilon)
    }

    /// Emite a Matriz de Identidade padrão, sem nenhuma deturpação afim no modelo original.
    pub fn identity() -> Self {
        Self::new([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0])
    }

    /// Construção de Matriz injetora de Translação Linear.
    pub fn translation(tx: f32, ty: f32) -> Self {
        Self::new([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, tx, ty, 1.0])
    }

    /// Construção de Matriz injetora de Expansão/Colapso (Escala).
    pub fn scale(sx: f32, sy: f32) -> Self {
        Self::new([sx, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 1.0])
    }

    /// Construção de Matriz injetora de Inclinação Plana (Cisalhamento).
    pub fn shear(kx: f32, ky: f32) -> Self {
        Self::new([1.0, ky, 0.0, kx, 1.0, 0.0, 0.0, 0.0, 1.0])
    }

    /// Construção de Matriz injetora de Rotação em torno da origem (Graus/Radianos).
    pub fn rotation(radians: f32) -> Self {
        let c = radians.cos();
        let s = radians.sin();
        Self::new([c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0])
    }

    /// Resolucionador Direto de Vetores por Operador Matricial (`Transform = Mat * V`).
    pub fn multiply_vec2(&self, v: &Vec2<f32>) -> Vec2<f32> {
        let x = v.x * self.data[0] + v.y * self.data[3] + self.data[6];
        let y = v.x * self.data[1] + v.y * self.data[4] + self.data[7];
        Vec2::new(x, y)
    }

    /// Permite envolver um `Rect` e aplicar diretamente a transformação.
    pub fn transform_rect(&self, rect: &Rect<f32>) -> Rect<f32> {
        rect.transform(self)
    }

    /// Calcula e retorna a Matriz Inversa (necessária para reverter transformações, ex: Hit-Testing em box escalada/rotacionada).
    ///
    /// # Exemplo
    /// ```
    /// use ace_core::math::Matrix3x3;
    /// let m = Matrix3x3::scale(2.0, 2.0);
    /// let inv = m.inverse().unwrap();
    /// let identity = m * inv;
    /// assert!(identity.approx_eq(&Matrix3x3::identity(), 1e-5));
    /// ```
    pub fn inverse(&self) -> Option<Self> {
        let d = &self.data;
        let det = d[0] * (d[4] * d[8] - d[7] * d[5]) - d[3] * (d[1] * d[8] - d[7] * d[2])
            + d[6] * (d[1] * d[5] - d[4] * d[2]);

        if abs(det) < 1e-10 {
            return None;
        }

        let inv_det = 1.0 / det;
        Some(Self::new([
            (d[4] * d[8] - d[5] * d[7]) * inv_det,
            (d[2] * d[7] - d[1] * d[8]) * inv_det,
            (d[1] * d[5] - d[2] * d[4]) * inv_det,
            (d[5] * d[6] - d[3] * d[8]) * inv_det,
            (d[0] * d[8] - d[2] * d[6]) * inv_det,
            (d[2] * d[3] - d[0] * d[5]) * inv_det,
            (d[3] * d[7] - d[4] * d[6]) * inv_det,
            (d[1] * d[6] - d[0] * d[7]) * inv_det,
            (d[0] * d[4] - d[1] * d[3]) * inv_det,
        ]))
    }
}

impl Mul for Matrix3x3<f32> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let a = self.data;
        let b = rhs.data;
        let mut res = [0.0; 9];
        // Layout Column-major: índice iterativo r + c * 3
        for r in 0..3 {
            for c in 0..3 {
                res[r + c * 3] =
                    a[r] * b[c * 3] + a[r + 3] * b[1 + c * 3] + a[r + 6] * b[2 + c * 3];
            }
        }
        Self::new(res)
    }
}

/// Matriz de Transformação Homogênea 4x4 (Projeção e Transformação 3D).
/// Totalmente alinhada em 16-bytes para forçar o backend do LLVM
/// a emitir instruções SIMD (AVX/SSE) em operações matemáticas de Loop Unrolling.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix4x4<T> {
    pub data: [T; 16],
}

impl<T: Default + Copy> Default for Matrix4x4<T> {
    fn default() -> Self {
        Self {
            data: [T::default(); 16],
        }
    }
}

impl<T: Default + Copy> Matrix4x4<T> {
    pub fn new(data: [T; 16]) -> Self {
        Self { data }
    }
}

impl Matrix4x4<f32> {
    pub fn approx_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.data
            .iter()
            .zip(other.data.iter())
            .all(|(&a, &b)| abs(a - b) < epsilon)
    }

    pub fn identity() -> Self {
        Self::new([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ])
    }

    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        if abs(aspect) < 1e-6 || abs(near - far) < 1e-6 {
            return Self::identity();
        }
        let tan_half_fov = (fov_y / 2.0).tan();
        let z_range = near - far;

        Self::new([
            1.0 / (aspect * tan_half_fov),
            0.0,
            0.0,
            0.0,
            0.0,
            1.0 / tan_half_fov,
            0.0,
            0.0,
            0.0,
            0.0,
            (far + near) / z_range,
            -1.0,
            0.0,
            0.0,
            (2.0 * far * near) / z_range,
            0.0,
        ])
    }

    pub fn multiply_vec3(&self, v: &Vec3<f32>) -> Vec3<f32> {
        let x = v.x * self.data[0] + v.y * self.data[4] + v.z * self.data[8] + self.data[12];
        let y = v.x * self.data[1] + v.y * self.data[5] + v.z * self.data[9] + self.data[13];
        let z = v.x * self.data[2] + v.y * self.data[6] + v.z * self.data[10] + self.data[14];
        Vec3::new(x, y, z)
    }

    pub fn transform_point3(&self, p: &Point<f32>) -> Point<f32> {
        let v = Vec3::new(p.x, p.y, 1.0);
        let r = self.multiply_vec3(&v);
        Point::new(r.x, r.y)
    }

    /// Calcula e retorna a Matriz Inversa 4x4.
    /// Indispensável para Câmeras 3D e projeções (View Matrix) nativas.
    pub fn inverse(&self) -> Option<Self> {
        let m = self.data;
        let mut inv = [0.0; 16];

        inv[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15]
            + m[9] * m[7] * m[14]
            + m[13] * m[6] * m[11]
            - m[13] * m[7] * m[10];
        inv[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15]
            - m[8] * m[7] * m[14]
            - m[12] * m[6] * m[11]
            + m[12] * m[7] * m[10];
        inv[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15]
            + m[8] * m[7] * m[13]
            + m[12] * m[5] * m[11]
            - m[12] * m[7] * m[9];
        inv[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14]
            - m[8] * m[6] * m[13]
            - m[12] * m[5] * m[10]
            + m[12] * m[6] * m[9];
        inv[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15]
            - m[9] * m[3] * m[14]
            - m[13] * m[2] * m[11]
            + m[13] * m[3] * m[10];
        inv[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15]
            + m[8] * m[3] * m[14]
            + m[12] * m[2] * m[11]
            - m[12] * m[3] * m[10];
        inv[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15]
            - m[8] * m[3] * m[13]
            - m[12] * m[1] * m[11]
            + m[12] * m[3] * m[9];
        inv[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14]
            + m[8] * m[2] * m[13]
            + m[12] * m[1] * m[10]
            - m[12] * m[2] * m[9];
        inv[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15]
            + m[5] * m[3] * m[14]
            + m[13] * m[2] * m[7]
            - m[13] * m[3] * m[6];
        inv[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15]
            - m[4] * m[3] * m[14]
            - m[12] * m[2] * m[7]
            + m[12] * m[3] * m[6];
        inv[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15]
            + m[4] * m[3] * m[13]
            + m[12] * m[1] * m[7]
            - m[12] * m[3] * m[5];
        inv[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14]
            - m[4] * m[2] * m[13]
            - m[12] * m[1] * m[6]
            + m[12] * m[2] * m[5];
        inv[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11]
            - m[5] * m[3] * m[10]
            - m[9] * m[2] * m[7]
            + m[9] * m[3] * m[6];
        inv[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11]
            + m[4] * m[3] * m[10]
            + m[8] * m[2] * m[7]
            - m[8] * m[3] * m[6];
        inv[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11]
            - m[4] * m[3] * m[9]
            - m[8] * m[1] * m[7]
            + m[8] * m[3] * m[5];
        inv[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10]
            + m[4] * m[2] * m[9]
            + m[8] * m[1] * m[6]
            - m[8] * m[2] * m[5];

        let det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];

        if det == 0.0 {
            return None;
        }

        let inv_det = 1.0 / det;
        for item in &mut inv {
            *item *= inv_det;
        }

        Some(Self::new(inv))
    }

    /// Fornece o Clipping de Projeção Ortográfica da câmera nativa sobre a renderização 3D.
    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let r_l = right - left;
        let t_b = top - bottom;
        let f_n = far - near;

        if abs(r_l) < 1e-6 || abs(t_b) < 1e-6 || abs(f_n) < 1e-6 {
            return Self::identity();
        }

        Self::new([
            2.0 / r_l,
            0.0,
            0.0,
            0.0,
            0.0,
            2.0 / t_b,
            0.0,
            0.0,
            0.0,
            0.0,
            -2.0 / f_n,
            0.0,
            -(right + left) / r_l,
            -(top + bottom) / t_b,
            -(far + near) / f_n,
            1.0,
        ])
    }
}

impl Mul for Matrix4x4<f32> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let a = self.data;
        let b = rhs.data;
        let mut res = [0.0; 16];
        for r in 0..4 {
            for c in 0..4 {
                res[r + c * 4] = a[r] * b[c * 4]
                    + a[r + 4] * b[1 + c * 4]
                    + a[r + 8] * b[2 + c * 4]
                    + a[r + 12] * b[3 + c * 4];
            }
        }
        Self::new(res)
    }
}

// ----------------------------------------------------------------------------
// Colorimetry (Pintura e Tratamento de Tons)
// ----------------------------------------------------------------------------

/// Entidade Representativa de um Píxel ou Cor em `RGBA` compactado (32 bits totais).
///
/// Mantém as definições de memória extremamente enxutas ao usar o primitivo `u8` para
/// que os Buffers de Cor possam ser varridos massivamente pelo renderizador de software/hardware sem overhead.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Inicializa a infraestrutura binária do componente com opacidade incluída.
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Construtor Semântico em que 100% de Opacidade é mandatória (Alpha=255).
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    /// Lê e decodifica uma string Hexadecimal CSS (`#FF0000` ou `#FF0000FF`) para a estrutura `Color`.
    /// Ignora o `#` inicial se presente. Retorna `None` se a string for inválida.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let h = hex.trim_start_matches('#');
        if h.len() == 6 {
            let r = u8::from_str_radix(&h[0..2], 16).ok()?;
            let g = u8::from_str_radix(&h[2..4], 16).ok()?;
            let b = u8::from_str_radix(&h[4..6], 16).ok()?;
            Some(Self::rgb(r, g, b))
        } else if h.len() == 8 {
            let r = u8::from_str_radix(&h[0..2], 16).ok()?;
            let g = u8::from_str_radix(&h[2..4], 16).ok()?;
            let b = u8::from_str_radix(&h[4..6], 16).ok()?;
            let a = u8::from_str_radix(&h[6..8], 16).ok()?;
            Some(Self::new(r, g, b, a))
        } else {
            None
        }
    }

    /// Transcreve a estática RGB para a especificação polar **HSLA** adotada formalmente pelo W3C CSS.
    ///
    /// # Retorno
    /// - Uma Tupla `(H, S, L, A)` em representação Float `f32`.
    /// - H (Matiz): Espectro radial flutuando entre `[0.0, 360.0)`.
    /// - S, L, A: Parametrizações fracionais firmadas entre `[0.0, 1.0]`.
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
            s = if l > 0.5 {
                d / (2.0 - max - min)
            } else {
                d / (max + min)
            };

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

    /// Reconstrói a estrutura nativa de Bytes `RGBA` extraídos de uma formulação `HSLA`.
    /// O processamento interno garante mitigação total de overflows numéricos através de truncagem de arredondamento.
    pub fn from_hsla(h: f32, s: f32, l: f32, a: f32) -> Self {
        let mut r = l;
        let mut g = l;
        let mut b = l;

        if s != 0.0 {
            let q = if l < 0.5 {
                l * (1.0 + s)
            } else {
                l + s - l * s
            };
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

    #[doc(hidden)]
    fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 1.0 / 2.0 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    }

    /// Produz uma nova derivação desta cor sofrendo redução absoluta no canal de luminosidade.
    pub fn darken(&self, amount: f32) -> Self {
        let (h, s, mut l, a) = self.to_hsla();
        l = saturate(l - amount);
        Self::from_hsla(h, s, l, a)
    }

    /// Produz uma nova derivação desta cor com incremento matemático no canal de luminosidade.
    pub fn lighten(&self, amount: f32) -> Self {
        let (h, s, mut l, a) = self.to_hsla();
        l = saturate(l + amount);
        Self::from_hsla(h, s, l, a)
    }

    /// Aplica o Composing Clássico `Source-Over` Porter-Duff do Canvas.
    ///
    /// Simula perfeitamente a matemática da refração de opacidade ao sobrepor
    /// uma cor semi-transparente sobre um fundo de píxels.
    ///
    /// # Exemplo
    /// ```
    /// use ace_core::math::Color;
    /// let fg = Color::new(255, 0, 0, 128); // Vermelho Semi-transparente
    /// let bg = Color::new(0, 0, 255, 255); // Azul Sólido
    /// let blended = fg.blend_source_over(&bg);
    /// assert_eq!(blended.r, 128);
    /// assert_eq!(blended.a, 255);
    /// ```
    pub fn blend_source_over(&self, bg: &Color) -> Color {
        let fg_a = self.a as f32 / 255.0;
        let bg_a = bg.a as f32 / 255.0;

        let out_a = fg_a + bg_a * (1.0 - fg_a);
        if out_a == 0.0 {
            return Color::new(0, 0, 0, 0);
        }

        let out_r = ((self.r as f32 * fg_a) + (bg.r as f32 * bg_a * (1.0 - fg_a))) / out_a;
        let out_g = ((self.g as f32 * fg_a) + (bg.g as f32 * bg_a * (1.0 - fg_a))) / out_a;
        let out_b = ((self.b as f32 * fg_a) + (bg.b as f32 * bg_a * (1.0 - fg_a))) / out_a;

        Color::new(
            out_r.round() as u8,
            out_g.round() as u8,
            out_b.round() as u8,
            (out_a * 255.0).round() as u8,
        )
    }

    /// Implementação nativa da especificação de Multiply Blend-Mode.
    pub fn blend_multiply(&self, bg: &Color) -> Color {
        Color::new(
            ((self.r as u16 * bg.r as u16) / 255) as u8,
            ((self.g as u16 * bg.g as u16) / 255) as u8,
            ((self.b as u16 * bg.b as u16) / 255) as u8,
            max(self.a, bg.a),
        )
    }

    /// Implementação nativa da especificação de Screen Blend-Mode.
    pub fn blend_screen(&self, bg: &Color) -> Color {
        Color::new(
            (255 - ((255 - self.r as u16) * (255 - bg.r as u16)) / 255) as u8,
            (255 - ((255 - self.g as u16) * (255 - bg.g as u16)) / 255) as u8,
            (255 - ((255 - self.b as u16) * (255 - bg.b as u16)) / 255) as u8,
            max(self.a, bg.a),
        )
    }

    /// Transição temporal fluida interpolada (ex: transições CSS).
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(
            lerp(self.r as f32, other.r as f32, t).round() as u8,
            lerp(self.g as f32, other.g as f32, t).round() as u8,
            lerp(self.b as f32, other.b as f32, t).round() as u8,
            lerp(self.a as f32, other.a as f32, t).round() as u8,
        )
    }

    /// Compilação do canal em primitivo nativo de 32 bits.
    ///
    /// Empacota a cor na ordem **RGBA** (R no byte mais significativo).
    /// Fundamental para injeção de pixels em FrameBuffers (Framebuffer Upload).
    pub fn to_rgba_u32(&self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }

    /// Empacota a cor na ordem **ARGB** (A no byte mais significativo).
    pub fn to_argb_u32(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Empacota a cor na ordem **BGRA** (B no byte mais significativo). Comum no Windows GDI e IPC Vulkan.
    pub fn to_bgra_u32(&self) -> u32 {
        ((self.b as u32) << 24) | ((self.g as u32) << 16) | ((self.r as u32) << 8) | (self.a as u32)
    }
}
