use super::*;
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache};
use tiny_skia::{BlendMode, Color, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};



pub(crate) fn blend_color_pixel(data: &mut [u8], pixel_idx: usize, src_pixel: &[u8], opacity: f32) {
    let src_a = (src_pixel[3] as f32 / 255.0) * opacity;
    if src_a > 0.0 {
        let dst_a = 1.0 - src_a;
        data[pixel_idx] = (src_pixel[0] as f32 * src_a + data[pixel_idx] as f32 * dst_a) as u8;
        data[pixel_idx + 1] =
            (src_pixel[1] as f32 * src_a + data[pixel_idx + 1] as f32 * dst_a) as u8;
        data[pixel_idx + 2] =
            (src_pixel[2] as f32 * src_a + data[pixel_idx + 2] as f32 * dst_a) as u8;
        data[pixel_idx + 3] = 255;
    }
}
