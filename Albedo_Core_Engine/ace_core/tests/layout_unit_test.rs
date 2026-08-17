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
