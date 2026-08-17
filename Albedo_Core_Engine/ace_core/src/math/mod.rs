//! # Matemática, Geometria e Cores
//!
//! Fundação gráfica e matemática do Albedo Browser, provendo sistemas de coordenadas
//! tipados, transformações afins 2D/3D (sobre `euclid`), aritmética de layout em ponto fixo (`LayoutUnit`),
//! aritmética checked contra overflow (`Checked`), raios de borda W3C (`BorderRadii`) e manipulação de cores CSS.

pub mod border_radii;
pub mod checked;
pub mod color;
pub mod geometry;
pub mod layout_unit;
pub mod units;
pub mod utils;

pub use border_radii::{BorderRadii, CornerRadius};
pub use checked::{Checked, CheckedSize};
pub use color::Color;
pub use geometry::{
    point2, rect, size2, vec2, vec3, DeviceEdgeInsets, DevicePoint, DeviceRect, DeviceSize,
    EdgeInsets, EdgeInsetsExt, Matrix4D, Point, PointExt, Rect2D, RectExt, Size, SizeExt,
    Transform, Vec2, Vec3, Vec4,
};
pub use layout_unit::LayoutUnit;
pub use units::{CssPixel, DevicePixel, DpiScale, LayoutPixel, ScreenPixel};
pub use utils::{almost_equal, clamp, deg_to_rad, lerp, rad_to_deg, saturate, snap_to_pixel};
