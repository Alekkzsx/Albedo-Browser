use super::*;
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache};
use tiny_skia::{BlendMode, Color, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};


pub(crate) fn blend_mask_pixel(
    data: &mut [u8],
    pixel_idx: usize,
    alpha_byte: u8,
    opacity: f32,
    color: (f32, f32, f32),
) {
    let src_a = (alpha_byte as f32 / 255.0) * opacity;
    if src_a > 0.0 {
        let dst_a = 1.0 - src_a;
        data[pixel_idx] = (color.0 * src_a + data[pixel_idx] as f32 * dst_a) as u8;
        data[pixel_idx + 1] = (color.1 * src_a + data[pixel_idx + 1] as f32 * dst_a) as u8;
        data[pixel_idx + 2] = (color.2 * src_a + data[pixel_idx + 2] as f32 * dst_a) as u8;
        data[pixel_idx + 3] = 255;
    }
}
