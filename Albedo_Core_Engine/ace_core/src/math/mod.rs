//! # Matemática, Geometria e Cores
//!
//! Fundação gráfica e matemática do Albedo Browser, provendo sistemas de coordenadas
//! tipados, transformações afins 2D/3D (sobre `euclid`), aritmética de layout em ponto fixo (`LayoutUnit`)
//! e manipulação de cores CSS.

pub mod units;
pub mod geometry;
pub mod color;
pub mod layout_unit;
pub mod utils;

pub use units::{CssPixel, DevicePixel, LayoutPixel, ScreenPixel, DpiScale};
pub use geometry::{
    Point, Size, Rect2D, Vec2, Vec3, Vec4, Transform, Matrix4D,
    DevicePoint, DeviceSize, DeviceRect, DeviceEdgeInsets, EdgeInsets,
    PointExt, SizeExt, EdgeInsetsExt, RectExt,
    point2, rect, size2, vec2, vec3,
};
pub use color::Color;
pub use layout_unit::LayoutUnit;
pub use utils::{lerp, clamp, saturate, almost_equal, snap_to_pixel, deg_to_rad, rad_to_deg};
