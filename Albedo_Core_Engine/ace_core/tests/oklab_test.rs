use ace_core::math::color::Color;

#[test]
fn test_oklab_from_and_to_srgb_roundtrip() {
    let red = Color::RED;
    let oklab = red.to_oklab();
    assert!((oklab.l - 0.627).abs() < 0.05); // Luminosidade aproximada do vermelho puro em Oklab
    assert!(oklab.a > 0.1); // Componente positiva no eixo verde-vermelho

    let back_to_color = Color::from_oklab(oklab);
    assert_eq!(back_to_color.r, 255);
    assert_eq!(back_to_color.g, 0);
    assert_eq!(back_to_color.b, 0);
}

#[test]
fn test_oklch_polar_roundtrip() {
    let blue = Color::BLUE;
    let oklch = blue.to_oklch();
    assert!((oklch.l - 0.45).abs() < 0.1);
    assert!(oklch.c > 0.2); // Alto croma

    let back_to_color = Color::from_oklch(oklch);
    assert_eq!(back_to_color.r, 0);
    assert_eq!(back_to_color.g, 0);
    assert_eq!(back_to_color.b, 255);
}

#[test]
fn test_color_parse_oklab() {
    let c = Color::parse("oklab(0.59 0.1 0.1 / 0.8)").unwrap();
    assert_eq!(c.a, 204); // 0.8 * 255 = 204
    assert!(c.r > 100);
}

#[test]
fn test_color_parse_oklch() {
    let c = Color::parse("oklch(0.6 0.15 120deg)").unwrap();
    assert_eq!(c.a, 255);
    assert!(c.g > 0);
}

#[test]
fn test_oklab_perceptual_interpolation() {
    let c1 = Color::from_rgb(255, 0, 0);
    let c2 = Color::from_rgb(0, 255, 0);

    let mid = c1.lerp_oklab(c2, 0.5);
    // Em Oklab, a transição vermelho-verde passa por um amarelo perceptual agradável
    assert!(mid.r > 100);
    assert!(mid.g > 100);
}
