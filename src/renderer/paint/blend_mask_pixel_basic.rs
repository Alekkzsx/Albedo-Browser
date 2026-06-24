use super::*;
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache};
use tiny_skia::{BlendMode, Color, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
pub(crate) fn blend_mask_pixel_basic() {
        let mut data = vec![128u8; 8]; // 2 pixels, RGBA
        blend_mask_pixel(&mut data, 0, 128, 1.0, (255.0, 0.0, 0.0));
        // src_a = 128/255 * 1.0 ≈ 0.502
        assert!(data[0] > 128); // red channel should increase
        assert_eq!(data[1], 128); // green unchanged (0.0 * src_a = 0)
        assert_eq!(data[2], 128); // blue unchanged
        assert_eq!(data[3], 255); // alpha always 255
    }

    #[test]
pub(crate) fn blend_mask_pixel_zero_alpha() {
        let mut data = vec![128u8; 4];
        blend_mask_pixel(&mut data, 0, 0, 1.0, (255.0, 0.0, 0.0));
        assert_eq!(data, vec![128, 128, 128, 128]); // no change
    }

    #[test]
pub(crate) fn blend_color_pixel_basic() {
        let mut data = vec![0u8; 8]; // 2 pixels RGBA
        let src = vec![255u8, 128, 64, 255]; // fully opaque source pixel
        blend_color_pixel(&mut data, 0, &src, 1.0);
        assert_eq!(data[0], 255); // red
        assert_eq!(data[1], 128); // green
        assert_eq!(data[2], 64); // blue
        assert_eq!(data[3], 255); // alpha
    }

    #[test]
pub(crate) fn blend_color_pixel_semi_transparent() {
        let mut data = vec![200u8; 4];
        let src = vec![100u8, 100, 100, 128]; // ~50% alpha
        blend_color_pixel(&mut data, 0, &src, 1.0);
        // src_a = 128/255 ≈ 0.502
        assert!(data[0] > 100 && data[0] < 200);
    }
}
