//! # Matemática, Geometria e Cores
//!
//! Fundação gráfica e matemática do Albedo Browser, provendo sistemas de coordenadas
//! tipados, transformações afins 2D/3D (sobre `euclid`), decomposição W3C 4x4 com SLERP (`TransformMatrix4`),
//! operadores Porter-Duff e blend modes CSS, colorimetria avançada (CIEDE2000 e Oklab),
//! aritmética de layout em ponto fixo (`LayoutUnit`) e interseção de raios 3D (`Ray3D` / `Aabb3D`).

pub mod blend;
pub mod border_radii;
pub mod checked;
pub mod ciede2000;
pub mod color;
pub mod decompose;
pub mod geometry;
pub mod layout_unit;
pub mod logical;
pub mod oklab;
pub mod quad;
pub mod ray_aabb;
pub mod units;
pub mod utils;

pub use blend::{
    composite_blend, composite_porter_duff, ColorRgba, CssBlendMode, PorterDuffOperator,
};
pub use border_radii::{BorderRadii, CornerRadius};
pub use checked::{Checked, CheckedSize};
pub use ciede2000::ciede2000;
pub use color::Color;
pub use decompose::{DecomposedTransform, Quaternion, TransformMatrix4};
pub use geometry::{
    point2, rect, size2, vec2, vec3, DeviceEdgeInsets, DevicePoint, DeviceRect, DeviceSize,
    EdgeInsets, EdgeInsetsExt, Matrix4D, Point, PointExt, Rect2D, RectExt, Size, SizeExt,
    Transform, Vec2, Vec3, Vec4,
};
pub use layout_unit::LayoutUnit;
pub use logical::{
    CssLogicalPoint, CssLogicalRect, CssLogicalSides, CssLogicalSize, Direction, LayoutPoint,
    LayoutRect, LayoutSize, LogicalLayoutPoint, LogicalLayoutRect, LogicalLayoutSides,
    LogicalLayoutSize, LogicalPoint, LogicalRect, LogicalSides, LogicalSize, WritingMode,
};
pub use oklab::{Oklab, Oklch};
pub use quad::{CssQuad, DeviceQuad, LayoutQuad, Quad2D};
pub use ray_aabb::{Aabb3D, Ray3D};
pub use units::{CssPixel, DevicePixel, DpiScale, LayoutPixel, ScreenPixel};
pub use utils::{almost_equal, clamp, deg_to_rad, lerp, rad_to_deg, saturate, snap_to_pixel};
