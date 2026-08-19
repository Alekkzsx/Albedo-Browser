//! # Geometria de Polígonos Convexos e Quadriláteros Transformados (`Quad2D`)
//!
//! Representação de 4 vértices planares projetados através de matrizes de transformação 2D (CSS Transforms)
//! com cálculo de AABB (*Axis-Aligned Bounding Box*) e detecção de inclusão de ponto para Hit-Testing.

use crate::math::geometry::rect;
use crate::math::layout_unit::LayoutUnit;
use crate::math::units::{CssPixel, DevicePixel, LayoutPixel};
use euclid::{Point2D, Rect, Transform2D};

/// Quadrilátero planar definido por 4 vértices $[p_0, p_1, p_2, p_3]$ no sentido horário.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad2D<T, U> {
    /// Os quatro vértices do quadrilátero (TopLeft, TopRight, BottomRight, BottomLeft).
    pub points: [Point2D<T, U>; 4],
}

impl<T: Copy, U> Quad2D<T, U> {
    /// Cria um novo `Quad2D` a partir de 4 pontos explícitos.
    #[inline]
    pub const fn new(
        p0: Point2D<T, U>,
        p1: Point2D<T, U>,
        p2: Point2D<T, U>,
        p3: Point2D<T, U>,
    ) -> Self {
        Self {
            points: [p0, p1, p2, p3],
        }
    }
}

impl<U: Copy> Quad2D<f32, U> {
    /// Constrói um quadrilátero a partir de um retângulo alinhado aos eixos.
    pub fn from_rect(r: &Rect<f32, U>) -> Self {
        let tl = r.origin;
        let tr = Point2D::new(r.origin.x + r.size.width, r.origin.y);
        let br = Point2D::new(r.origin.x + r.size.width, r.origin.y + r.size.height);
        let bl = Point2D::new(r.origin.x, r.origin.y + r.size.height);
        Self::new(tl, tr, br, bl)
    }

    /// Projeta os 4 vértices de um retângulo através de uma matriz de transformação afim.
    pub fn from_transformed_rect(r: &Rect<f32, U>, transform: &Transform2D<f32, U, U>) -> Self {
        let base_quad = Self::from_rect(r);
        let p0 = transform.transform_point(base_quad.points[0]);
        let p1 = transform.transform_point(base_quad.points[1]);
        let p2 = transform.transform_point(base_quad.points[2]);
        let p3 = transform.transform_point(base_quad.points[3]);

        Self::new(p0, p1, p2, p3)
    }

    /// Calcula a menor caixa delimitadora alinhada aos eixos (AABB) que envolve todos os 4 vértices.
    pub fn bounding_box(&self) -> Rect<f32, U> {
        let mut min_x = self.points[0].x;
        let mut max_x = self.points[0].x;
        let mut min_y = self.points[0].y;
        let mut max_y = self.points[0].y;

        for p in &self.points[1..] {
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

        rect(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Retorna o número de enrolamento (*Winding Number*) do quadrilátero em relação a um ponto.
    pub fn winding_number(&self, point: &Point2D<f32, U>) -> i32 {
        let mut wn = 0;
        let px = point.x;
        let py = point.y;

        for i in 0..4 {
            let p1 = self.points[i];
            let p2 = self.points[(i + 1) % 4];

            if p1.y <= py {
                if p2.y > py {
                    let is_left = (p2.x - p1.x) * (py - p1.y) - (px - p1.x) * (p2.y - p1.y);
                    if is_left > 0.0 {
                        wn += 1;
                    }
                }
            } else if p2.y <= py {
                let is_left = (p2.x - p1.x) * (py - p1.y) - (px - p1.x) * (p2.y - p1.y);
                if is_left < 0.0 {
                    wn -= 1;
                }
            }
        }
        wn
    }

    /// Testa se um ponto bidimensional está contido no interior do quadrilátero
    /// utilizando o algoritmo de Ray-Casting (Even-Odd / Jordan Curve Theorem),
    /// suportando quadriláteros convexos, côncavos, rotacionados ou projetados por matrizes 3D.
    pub fn contains_point(&self, point: &Point2D<f32, U>) -> bool {
        let mut inside = false;
        let px = point.x;
        let py = point.y;

        for i in 0..4 {
            let j = (i + 3) % 4;
            let pi = self.points[i];
            let pj = self.points[j];

            let dy = pj.y - pi.y;
            if dy.abs() > 1e-6 {
                let intersect = ((pi.y > py) != (pj.y > py))
                    && (px < (pj.x - pi.x) * (py - pi.y) / dy + pi.x);

                if intersect {
                    inside = !inside;
                }
            }
        }

        inside
    }
}

/// Aliases convenientes para os sistemas de unidades do motor
pub type CssQuad = Quad2D<f32, CssPixel>;
pub type DeviceQuad = Quad2D<f32, DevicePixel>;
pub type LayoutQuad = Quad2D<LayoutUnit, LayoutPixel>;
