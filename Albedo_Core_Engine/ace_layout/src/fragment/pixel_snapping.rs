//! # Subpixel Pixel Snapping (`pixel_snapping.rs`)
//!
//! Implements exact continuous pixel snapping based on `ace_core::math::layout_unit::snap_box`.
//! Ensures zero 1px gaps (pixel cracking) and zero 1px overlaps between contiguous adjacent boxes
//! across any screen density or fractional subpixel boundary.

use ace_core::math::layout_unit::{snap_box, LayoutUnit};
use ace_core::math::logical::LogicalRect;
use euclid::default::Rect;

/// Snaps a 1D scalar interval `(origin, size)` using the zero-cracking rounding formula:
/// - `snapped_origin = round(origin)`
/// - `snapped_end = round(origin + size)`
/// - `snapped_size = snapped_end - snapped_origin`
#[inline]
pub fn snap_box_1d(origin: LayoutUnit, size: LayoutUnit) -> (i32, i32) {
    snap_box(origin, size)
}

/// Snaps a 2D `LogicalRect<LayoutUnit>` to an integer device pixel rectangle `Rect<i32>`.
///
/// Ensures the Adjacency Theorem:
/// If box B2 starts exactly at B1's end (`B2.origin == B1.origin + B1.size`),
/// then `snapped_B1.origin + snapped_B1.size == snapped_B2.origin`.
#[inline]
pub fn snap_logical_rect(rect: &LogicalRect<LayoutUnit>) -> Rect<i32> {
    let (x, w) = snap_box(rect.inline_start, rect.inline_size);
    let (y, h) = snap_box(rect.block_start, rect.block_size);
    Rect::new(euclid::point2(x, y), euclid::size2(w, h))
}

/// Verifies that a sequence of contiguous 1D intervals exhibits strictly zero gaps or overlaps.
/// Returns true if all adjacent boundaries align to the exact same snapped pixel.
pub fn verify_zero_pixel_cracking(slices: &[(LayoutUnit, LayoutUnit)]) -> bool {
    if slices.len() <= 1 {
        return true;
    }

    let mut prev_end: Option<i32> = None;
    for &(origin, size) in slices {
        let (snapped_orig, snapped_size) = snap_box(origin, size);
        let snapped_end = snapped_orig + snapped_size;

        if let Some(prev) = prev_end {
            if prev != snapped_orig {
                return false;
            }
        }
        prev_end = Some(snapped_end);
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapping_contiguous_boxes_zero_cracking() {
        // Divide 100px into thirds: each 33.333px = 2000 au
        let third = LayoutUnit::from_raw(2000);
        let origin = LayoutUnit::ZERO;

        let slices = [
            (origin, third),
            (origin + third, third),
            (origin + third + third, third),
        ];

        assert!(verify_zero_pixel_cracking(&slices));

        let (o0, s0) = snap_box_1d(slices[0].0, slices[0].1);
        let (o1, s1) = snap_box_1d(slices[1].0, slices[1].1);
        let (o2, s2) = snap_box_1d(slices[2].0, slices[2].1);

        assert_eq!(o0, 0);
        assert_eq!(s0, 33);
        assert_eq!(o1, 33);
        assert_eq!(s1, 34);
        assert_eq!(o2, 67);
        assert_eq!(s2, 33);

        assert_eq!(o0 + s0, o1);
        assert_eq!(o1 + s1, o2);
        assert_eq!(s0 + s1 + s2, 100);
    }

    #[test]
    fn test_snap_logical_rect() {
        let lrect = LogicalRect {
            inline_start: LayoutUnit::from_px(10),
            block_start: LayoutUnit::from_px(20),
            inline_size: LayoutUnit::from_px(100),
            block_size: LayoutUnit::from_px(200),
        };

        let snapped = snap_logical_rect(&lrect);
        assert_eq!(snapped.origin.x, 10);
        assert_eq!(snapped.origin.y, 20);
        assert_eq!(snapped.size.width, 100);
        assert_eq!(snapped.size.height, 200);
    }
}
