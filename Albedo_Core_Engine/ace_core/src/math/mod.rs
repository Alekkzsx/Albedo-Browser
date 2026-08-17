//! # Matemática, Geometria e Cores
//!
//! Fundação gráfica e matemática do Albedo Browser, provendo sistemas de coordenadas
//! tipados, transformações afins 2D/3D (sobre `euclid`) e manipulação de cores CSS.

pub mod units;
pub mod geometry;
pub mod color;

pub use units::{CssPixel, DevicePixel, LayoutPixel, ScreenPixel, DpiScale};
pub use geometry::{
    Point, Size, Rect2D, Vec2, Vec3, Vec4, Transform, Matrix4D,
    DevicePoint, DeviceSize, DeviceRect, DeviceEdgeInsets, EdgeInsets,
    PointExt, SizeExt, EdgeInsetsExt, RectExt,
    point2, rect, size2, vec2, vec3,
};
pub use color::Color;
