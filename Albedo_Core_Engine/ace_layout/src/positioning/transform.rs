//! # Matrizes de Transformação Afim 2D (`Transform2D`)
//!
//! Implementação completa de transformações geométricas afins 2D conforme
//! a especificação W3C CSS Transforms Module Level 1 §3.

use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::LogicalRect;
use euclid::default::{Point2D, Rect};

/// Matriz de transformação afim 2D em coordenadas homogêneas (3x3):
///
/// ```text
/// [ a  c  e ]
/// [ b  d  f ]
/// [ 0  0  1 ]
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D<T = f32> {
    pub a: T,
    pub b: T,
    pub c: T,
    pub d: T,
    pub e: T,
    pub f: T,
}

impl Default for Transform2D<f32> {
    #[inline]
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform2D<f32> {
    /// Retorna a matriz identidade.
    #[inline]
    pub const fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Cria uma transformação de translação por um vetor `(tx, ty)`.
    #[inline]
    pub const fn translate(tx: f32, ty: f32) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    /// Cria uma transformação de escala pelos fatores `(sx, sy)`.
    #[inline]
    pub const fn scale(sx: f32, sy: f32) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Cria uma transformação de rotação por `radians` radianos (sentido horário em coordenadas de tela).
    #[inline]
    pub fn rotate(radians: f32) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Cria uma transformação de cisalhamento (shear) com fatores `(kx, ky)`.
    #[inline]
    pub const fn shear(kx: f32, ky: f32) -> Self {
        Self {
            a: 1.0,
            b: ky,
            c: kx,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Cria uma transformação de inclinação (skew) com ângulos em radianos.
    #[inline]
    pub fn skew(rad_x: f32, rad_y: f32) -> Self {
        Self::shear(rad_x.tan(), rad_y.tan())
    }

    /// Verifica se a transformação é aproximadamente a matriz identidade.
    #[inline]
    pub fn is_identity(&self) -> bool {
        (self.a - 1.0).abs() < 1e-6
            && self.b.abs() < 1e-6
            && self.c.abs() < 1e-6
            && (self.d - 1.0).abs() < 1e-6
            && self.e.abs() < 1e-6
            && self.f.abs() < 1e-6
    }

    /// Multiplicação de matrizes: `self * other`.
    ///
    /// Aplica `other` primeiro e depois `self`: $P' = \mathbf{M}_{\text{self}} \times (\mathbf{M}_{\text{other}} \times P)$.
    #[inline]
    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            e: self.a * other.e + self.c * other.f + self.e,
            f: self.b * other.e + self.d * other.f + self.f,
        }
    }

    /// Calcula o determinante $\det(\mathbf{M}) = ad - bc$.
    #[inline]
    pub fn determinant(&self) -> f32 {
        self.a * self.d - self.b * self.c
    }

    /// Calcula a matriz inversa $\mathbf{M}^{-1}$.
    ///
    /// Retorna `None` se a matriz for singular ($|\det| < 10^{-6}$).
    pub fn inverse(&self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < 1e-6 {
            return None;
        }
        let inv_det = 1.0 / det;
        Some(Self {
            a: self.d * inv_det,
            b: -self.b * inv_det,
            c: -self.c * inv_det,
            d: self.a * inv_det,
            e: (self.c * self.f - self.d * self.e) * inv_det,
            f: (self.b * self.e - self.a * self.f) * inv_det,
        })
    }

    /// Transforma um ponto 2D: $(x', y') = (ax + cy + e, bx + dy + f)$.
    #[inline]
    pub fn transform_point(&self, p: Point2D<f32>) -> Point2D<f32> {
        Point2D::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    /// Aplica a transformação em torno de um ponto pivô arbitrário (`transform-origin`):
    /// $$\mathbf{M}_{\text{origin}} = \text{Translate}(x_0, y_0) \times \mathbf{M} \times \text{Translate}(-x_0, -y_0)$$
    pub fn with_transform_origin(&self, origin: Point2D<f32>) -> Self {
        let t_pre = Self::translate(-origin.x, -origin.y);
        let t_post = Self::translate(origin.x, origin.y);
        t_post.multiply(&self.multiply(&t_pre))
    }

    /// Método de interface canônico conforme PROJECT.md.
    ///
    /// Calcula o Axis-Aligned Bounding Box (AABB) do retângulo transformado
    /// em coordenadas de layout, garantindo limites conservadores para culling de visual overflow.
    pub fn project_rect(&self, rect: Rect<LayoutUnit>) -> Rect<LayoutUnit> {
        let x0 = rect.origin.x.to_f32_px();
        let y0 = rect.origin.y.to_f32_px();
        let x1 = x0 + rect.size.width.to_f32_px();
        let y1 = y0 + rect.size.height.to_f32_px();

        let corners = [
            self.transform_point(Point2D::new(x0, y0)),
            self.transform_point(Point2D::new(x1, y0)),
            self.transform_point(Point2D::new(x1, y1)),
            self.transform_point(Point2D::new(x0, y1)),
        ];

        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for p in &corners {
            if p.x < min_x {
                min_x = p.x;
            }
            if p.x > max_x {
                max_x = p.x;
            }
            if p.y < min_y {
                min_y = p.y;
            }
            if p.y > max_y {
                max_y = p.y;
            }
        }

        let origin_x = LayoutUnit::from_f32_px(min_x);
        let origin_y = LayoutUnit::from_f32_px(min_y);
        let width = LayoutUnit::from_f32_px((max_x - min_x).max(0.0));
        let height = LayoutUnit::from_f32_px((max_y - min_y).max(0.0));

        Rect::new(
            Point2D::new(origin_x, origin_y),
            euclid::default::Size2D::new(width, height),
        )
    }

    /// Projeta um `LogicalRect<LayoutUnit>` calculando seu AABB transformado.
    pub fn project_logical_rect(&self, rect: LogicalRect<LayoutUnit>) -> LogicalRect<LayoutUnit> {
        let phys_rect = Rect::new(
            Point2D::new(rect.inline_start, rect.block_start),
            euclid::default::Size2D::new(rect.inline_size, rect.block_size),
        );
        let proj = self.project_rect(phys_rect);
        LogicalRect {
            inline_start: proj.origin.x,
            block_start: proj.origin.y,
            inline_size: proj.size.width,
            block_size: proj.size.height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let id = Transform2D::identity();
        assert!(id.is_identity());
        let p = Point2D::new(10.0, 20.0);
        assert_eq!(id.transform_point(p), p);
        assert_eq!(id.determinant(), 1.0);
    }

    #[test]
    fn test_translate() {
        let t = Transform2D::translate(15.0, -25.0);
        let p = Point2D::new(10.0, 20.0);
        let res = t.transform_point(p);
        assert_eq!(res, Point2D::new(25.0, -5.0));
    }

    #[test]
    fn test_scale() {
        let s = Transform2D::scale(2.0, 3.0);
        let p = Point2D::new(10.0, 20.0);
        let res = s.transform_point(p);
        assert_eq!(res, Point2D::new(20.0, 60.0));
        assert_eq!(s.determinant(), 6.0);
    }

    #[test]
    fn test_rotate_and_inverse() {
        use std::f32::consts::PI;
        let r = Transform2D::rotate(PI / 2.0); // 90 graus
        let p = Point2D::new(10.0, 0.0);
        let p_rot = r.transform_point(p);
        assert!((p_rot.x - 0.0).abs() < 1e-5);
        assert!((p_rot.y - 10.0).abs() < 1e-5);

        let inv = r.inverse().expect("Inverse should exist for rotation");
        let p_back = inv.transform_point(p_rot);
        assert!((p_back.x - 10.0).abs() < 1e-5);
        assert!((p_back.y - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_project_rect() {
        let t = Transform2D::scale(2.0, 2.0);
        let rect = Rect::new(
            Point2D::new(LayoutUnit::from_px(10), LayoutUnit::from_px(20)),
            euclid::default::Size2D::new(LayoutUnit::from_px(50), LayoutUnit::from_px(100)),
        );
        let proj = t.project_rect(rect);
        assert_eq!(proj.origin.x, LayoutUnit::from_px(20));
        assert_eq!(proj.origin.y, LayoutUnit::from_px(40));
        assert_eq!(proj.size.width, LayoutUnit::from_px(100));
        assert_eq!(proj.size.height, LayoutUnit::from_px(200));
    }
}
