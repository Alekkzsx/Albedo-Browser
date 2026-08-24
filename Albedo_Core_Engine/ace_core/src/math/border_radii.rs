//! # Geometria de Bordas Arredondadas (W3C CSS Backgrounds & Borders Level 3)
//!
//! Representação dos 4 cantos com raios elípticos $(r_x, r_y)$ e algoritmo canônico da W3C
//! para redução proporcional de curvas que se sobrepõem (*Overlapping Curves Resolution*).

/// Raio elíptico de um único canto ($x$ horizontal, $y$ vertical).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CornerRadius {
    pub x: f32,
    pub y: f32,
}

impl CornerRadius {
    /// Cria um raio circular uniforme ($r_x = r_y = r$).
    #[inline]
    pub const fn new(r: f32) -> Self {
        Self { x: r, y: r }
    }

    /// Cria um raio elíptico com componentes horizontal e vertical distintos.
    #[inline]
    pub const fn elliptical(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Retorna `true` se o raio for zero ou negativo.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.x <= 0.0 || self.y <= 0.0
    }
}

impl From<f32> for CornerRadius {
    #[inline]
    fn from(r: f32) -> Self {
        Self::new(r)
    }
}

impl From<(f32, f32)> for CornerRadius {
    #[inline]
    fn from((x, y): (f32, f32)) -> Self {
        Self::elliptical(x, y)
    }
}

/// Conjunto de raios de borda para os 4 cantos de uma caixa de layout.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BorderRadii {
    pub top_left: CornerRadius,
    pub top_right: CornerRadius,
    pub bottom_right: CornerRadius,
    pub bottom_left: CornerRadius,
}

impl BorderRadii {
    /// Cria bordas retangulares sem arredondamento (raios zerados).
    pub const ZERO: Self = Self {
        top_left: CornerRadius { x: 0.0, y: 0.0 },
        top_right: CornerRadius { x: 0.0, y: 0.0 },
        bottom_right: CornerRadius { x: 0.0, y: 0.0 },
        bottom_left: CornerRadius { x: 0.0, y: 0.0 },
    };

    /// Cria raios circulares idênticos para todos os 4 cantos.
    #[inline]
    pub const fn uniform(r: f32) -> Self {
        let cr = CornerRadius::new(r);
        Self {
            top_left: cr,
            top_right: cr,
            bottom_right: cr,
            bottom_left: cr,
        }
    }

    /// Cria `BorderRadii` especificando os 4 cantos individualmente.
    #[inline]
    pub const fn from_corners(
        top_left: CornerRadius,
        top_right: CornerRadius,
        bottom_right: CornerRadius,
        bottom_left: CornerRadius,
    ) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// Retorna `true` se todos os cantos tiverem raio zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.top_left.is_zero()
            && self.top_right.is_zero()
            && self.bottom_right.is_zero()
            && self.bottom_left.is_zero()
    }

    /// Executa o algoritmo canônico da W3C CSS Backgrounds & Borders Level 3 (Seção 5.5)
    /// para redução proporcional de curvas que ultrapassam as dimensões da caixa.
    ///
    /// Se a soma de dois raios adjacentes exceder a largura ou altura da caixa, todos os raios
    /// são multiplicados por um fator $f \in (0.0, 1.0]$.
    #[inline]
    pub fn resolve_overlapping(&self, width: f32, height: f32) -> Self {
        if width <= 0.0 || height <= 0.0 || self.is_zero() {
            return *self;
        }

        let s_top = self.top_left.x + self.top_right.x;
        let s_right = self.top_right.y + self.bottom_right.y;
        let s_bottom = self.bottom_left.x + self.bottom_right.x;
        let s_left = self.top_left.y + self.bottom_left.y;

        let max_w = s_top.max(s_bottom);
        let max_h = s_left.max(s_right);

        let scale_w = if max_w > width { width / max_w } else { 1.0 };
        let scale_h = if max_h > height { height / max_h } else { 1.0 };
        let f = scale_w.min(scale_h);

        if f < 1.0 {
            Self {
                top_left: CornerRadius::elliptical(self.top_left.x * f, self.top_left.y * f),
                top_right: CornerRadius::elliptical(self.top_right.x * f, self.top_right.y * f),
                bottom_right: CornerRadius::elliptical(
                    self.bottom_right.x * f,
                    self.bottom_right.y * f,
                ),
                bottom_left: CornerRadius::elliptical(
                    self.bottom_left.x * f,
                    self.bottom_left.y * f,
                ),
            }
        } else {
            *self
        }
    }
}
