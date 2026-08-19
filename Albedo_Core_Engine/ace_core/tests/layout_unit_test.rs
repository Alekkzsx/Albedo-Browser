use ace_core::math::LayoutUnit;

#[test]
fn test_layout_unit_conversions_and_subpixels() {
    let u1 = LayoutUnit::from_px(10);
    assert_eq!(u1.raw(), 600);
    assert_eq!(u1.to_f32_px(), 10.0);

    let u2 = LayoutUnit::from_f32_px(10.5); // Meio pixel = 30 unidades
    assert_eq!(u2.raw(), 630);
    assert_eq!(u2.to_f32_px(), 10.5);

    let u3 = LayoutUnit::from_f32_px(10.3333);
    assert_eq!(u3.floor_px(), 10);
    assert_eq!(u3.ceil_px(), 11);
    assert_eq!(u3.round_px(), 10);

    let u4 = LayoutUnit::from_f32_px(10.6666);
    assert_eq!(u4.floor_px(), 10);
    assert_eq!(u4.ceil_px(), 11);
    assert_eq!(u4.round_px(), 11);
}

#[test]
fn test_layout_unit_arithmetic() {
    let a = LayoutUnit::from_px(100);
    let b = LayoutUnit::from_px(50);

    assert_eq!((a + b).to_f32_px(), 150.0);
    assert_eq!((a - b).to_f32_px(), 50.0);
    assert_eq!((b * 3).to_f32_px(), 150.0);
    assert_eq!((a / 2).to_f32_px(), 50.0);

    // Divisões sem perda fracionária comum
    let third = a / 3; // 6000 / 3 = 2000 unidades = 33.3333px
    assert_eq!(third.raw(), 2000);
    assert_eq!(third * 3, a); // Divisão e multiplicação exata sem drift!
}

#[test]
fn test_layout_unit_saturation() {
    let max = LayoutUnit::from_raw(i32::MAX);
    let extra = LayoutUnit::from_px(10);
    assert_eq!(max + extra, max); // Saturação positiva

    let min = LayoutUnit::from_raw(i32::MIN);
    assert_eq!(min - extra, min); // Saturação negativa
}

#[test]
fn test_layout_unit_fractional_multiplication() {
    let base = LayoutUnit::from_px(100); // 6000 raw

    // 100px * 16 / 9 (aspect ratio 16:9)
    let ar = base.mul_div(16, 9);
    assert_eq!(ar.raw(), (6000 * 16) / 9);

    // Test with large numbers that would overflow 32-bit if not using 64-bit intermediate
    let large = LayoutUnit::from_raw(1_000_000);
    let scaled = large.mul_div(5_000, 2_000);
    assert_eq!(scaled.raw(), 2_500_000);

    // mul_layout_unit
    let width = LayoutUnit::from_px(20);
    let height = LayoutUnit::from_px(30);
    let area_scaled = width.mul_layout_unit(height);
    assert_eq!(area_scaled.to_f32_px(), 600.0);

    // div_layout_unit ratio
    let ratio = height.div_layout_unit(width);
    assert!((ratio - 1.5).abs() < 1e-4);
}

#[test]
fn test_layout_unit_box_snapping_zero_pixel_cracking() {
    use ace_core::math::layout_unit::snap_box;

    // Test case 1: 3 adjacent boxes dividing 100px into 3 thirds
    let o1 = LayoutUnit::from_f32_px(0.0);
    let s1 = LayoutUnit::from_f32_px(33.3333);
    let o2 = o1 + s1;
    let s2 = LayoutUnit::from_f32_px(33.3333);
    let o3 = o2 + s2;
    let s3 = LayoutUnit::from_f32_px(33.3334);

    let (x1, w1) = snap_box(o1, s1);
    let (x2, w2) = snap_box(o2, s2);
    let (x3, w3) = snap_box(o3, s3);

    // Right edge of Box 1 MUST identically equal Left edge of Box 2
    assert_eq!(x1 + w1, x2, "Pixel cracking detected between Box 1 and Box 2");
    // Right edge of Box 2 MUST identically equal Left edge of Box 3
    assert_eq!(x2 + w2, x3, "Pixel cracking detected between Box 2 and Box 3");

    // Total snapped width must equal snapped total span
    assert_eq!(w1 + w2 + w3, 100);

    // Method on LayoutUnit
    let (m_x1, m_w1) = o1.snap_box(s1);
    assert_eq!((m_x1, m_w1), (x1, w1));

    // Systematic grid test with arbitrary fractional offsets and widths
    let fractional_offsets = [0.0, 0.1, 0.25, 0.3333, 0.5, 0.6667, 0.75, 0.9, 1.15, 10.45];
    let fractional_widths = [10.3333, 15.6667, 20.25, 0.5, 7.8, 33.3333, 40.75];

    for &start_x in &fractional_offsets {
        let mut cur_origin = LayoutUnit::from_f32_px(start_x);
        for &w in &fractional_widths {
            let cur_size = LayoutUnit::from_f32_px(w);
            let next_origin = cur_origin + cur_size;
            let next_size = LayoutUnit::from_f32_px(w * 1.5);

            let (snapped_x1, snapped_w1) = cur_origin.snap_box(cur_size);
            let (snapped_x2, _) = next_origin.snap_box(next_size);

            let right_1 = snapped_x1 + snapped_w1;
            let left_2 = snapped_x2;

            assert_eq!(
                right_1, left_2,
                "Pixel cracking invariant violation: right_1 ({}) != left_2 ({}) for origin {} and size {}",
                right_1, left_2, cur_origin, cur_size
            );

            cur_origin = next_origin;
        }
    }
}

