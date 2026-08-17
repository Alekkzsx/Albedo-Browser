//! # Primitivas Geométricas e Transformações Afins 2D/3D
//!
//! Tipos geométricos fortemente tipados integrando o `euclid` com as unidades do Albedo.

use super::units::{CssPixel, DevicePixel, DpiScale};
pub use euclid::{
    default::{Point2D as RawPoint2D, Rect as RawRect, Size2D as RawSize2D, Transform2D as RawTransform2D},
    point2, rect, size2, vec2, vec3,
    Box2D, Length, Point2D, Point3D, Rect, SideOffsets2D, Size2D, Transform2D, Transform3D,
    Vector2D, Vector3D, Vector4D,
};

/// Ponto 2D no espaço de coordenadas CSS (`f32`).
pub type Point = Point2D<f32, CssPixel>;

/// Tamanho 2D (largura, altura) no espaço de coordenadas CSS (`f32`).
pub type Size = Size2D<f32, CssPixel>;

/// Retângulo 2D (origem, tamanho) no espaço de coordenadas CSS (`f32`).
pub type Rect2D = Rect<f32, CssPixel>;

/// Vetor 2D no espaço CSS (`f32`).
pub type Vec2 = Vector2D<f32, CssPixel>;

/// Vetor 3D no espaço CSS (`f32`).
pub type Vec3 = Vector3D<f32, CssPixel>;

/// Vetor 4D / Coordenadas Homogêneas (`f32`).
pub type Vec4 = Vector4D<f32, CssPixel>;

/// Transformação afim 2D (matriz 3x3) no espaço CSS.
pub type Transform = Transform2D<f32, CssPixel, CssPixel>;

/// Transformação 3D (matriz 4x4) para compositing GPU e transformações CSS 3D.
pub type Matrix4D = Transform3D<f32, CssPixel, CssPixel>;

/// Ponto 2D no buffer físico de pixels do dispositivo.
pub type DevicePoint = Point2D<f32, DevicePixel>;

/// Tamanho 2D no buffer físico de pixels do dispositivo.
pub type DeviceSize = Size2D<f32, DevicePixel>;

/// Retângulo 2D no buffer físico de pixels do dispositivo.
pub type DeviceRect = Rect<f32, DevicePixel>;

/// Margens, paddings e borders (top, right, bottom, left) no espaço CSS.
pub type EdgeInsets = SideOffsets2D<f32, CssPixel>;

/// Trait de extensões ergonômicas para manipulação de retângulos em motores de layout.
pub trait RectExt {
    /// Expande o retângulo aplicando insets externos (ex: margens/paddings).
    fn inflate_edges(&self, edges: EdgeInsets) -> Self;
    /// Encolhe o retângulo aplicando insets internos.
    fn deflate_edges(&self, edges: EdgeInsets) -> Self;
    /// Converte para coordenadas de pixel de dispositivo com base na escala de DPI.
    fn to_device_rect(&self, scale: DpiScale) -> DeviceRect;
}

impl RectExt for Rect2D {
    #[inline]
    fn inflate_edges(&self, edges: EdgeInsets) -> Self {
        Rect::new(
            Point::new(self.origin.x - edges.left, self.origin.y - edges.top),
            Size::new(
                (self.size.width + edges.left + edges.right).max(0.0),
                (self.size.height + edges.top + edges.bottom).max(0.0),
            ),
        )
    }

    #[inline]
    fn deflate_edges(&self, edges: EdgeInsets) -> Self {
        Rect::new(
            Point::new(self.origin.x + edges.left, self.origin.y + edges.top),
            Size::new(
                (self.size.width - edges.left - edges.right).max(0.0),
                (self.size.height - edges.top - edges.bottom).max(0.0),
            ),
        )
    }

    #[inline]
    fn to_device_rect(&self, scale: DpiScale) -> DeviceRect {
        DeviceRect::new(
            DevicePoint::new(
                scale.to_device_pixels(self.origin.x),
                scale.to_device_pixels(self.origin.y),
            ),
            DeviceSize::new(
                scale.to_device_pixels(self.size.width),
                scale.to_device_pixels(self.size.height),
            ),
        )
    }
}
