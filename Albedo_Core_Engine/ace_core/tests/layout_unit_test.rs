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

