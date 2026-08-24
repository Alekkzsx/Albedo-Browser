use ace_core::math::{BorderRadii, CornerRadius};

#[test]
fn test_border_radii_basic_and_uniform() {
    let radii = BorderRadii::uniform(10.0);
    assert_eq!(radii.top_left, CornerRadius::new(10.0));
    assert_eq!(radii.bottom_right, CornerRadius::new(10.0));
    assert!(!radii.is_zero());

    let zero = BorderRadii::ZERO;
    assert!(zero.is_zero());
}

#[test]
fn test_border_radii_w3c_overlapping_reduction() {
    // Caixa de 100x40 com cantos de raio 50px (ultrapassam a altura de 40px)
    // Soma vertical s_left = 50 + 50 = 100px > 40px
    // Fator f = 40 / 100 = 0.4
    let initial = BorderRadii::uniform(50.0);
    let resolved = initial.resolve_overlapping(100.0, 40.0);

    assert_eq!(resolved.top_left.x, 20.0); // 50 * 0.4
    assert_eq!(resolved.top_left.y, 20.0);
    assert_eq!(resolved.bottom_left.x, 20.0);
    assert_eq!(resolved.bottom_left.y, 20.0);
}
