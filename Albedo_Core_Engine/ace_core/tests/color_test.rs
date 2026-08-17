use ace_core::math::Color;

#[test]
fn test_hex_colors() {
    // #RGB
    assert_eq!(Color::from_hex("#f00").unwrap(), Color::from_rgb(255, 0, 0));
    assert_eq!(Color::from_hex("#0f0").unwrap(), Color::from_rgb(0, 255, 0));
    assert_eq!(Color::from_hex("#00f").unwrap(), Color::from_rgb(0, 0, 255));
    assert_eq!(
        Color::from_hex("#fff").unwrap(),
        Color::from_rgb(255, 255, 255)
    );

    // #RGBA
    assert_eq!(
        Color::from_hex("#f00f").unwrap(),
        Color::from_rgba(255, 0, 0, 255)
    );
    assert_eq!(
        Color::from_hex("#f008").unwrap(),
        Color::from_rgba(255, 0, 0, 136)
    );

    // #RRGGBB
    assert_eq!(
        Color::from_hex("#ff8800").unwrap(),
        Color::from_rgb(255, 136, 0)
    );
    assert_eq!(
        Color::from_hex("#123456").unwrap(),
        Color::from_rgb(18, 52, 86)
    );

    // #RRGGBBAA
    assert_eq!(
        Color::from_hex("#ff880080").unwrap(),
        Color::from_rgba(255, 136, 0, 128)
    );

    // Invalid hex
    assert!(Color::from_hex("#zzz").is_err());
    assert!(Color::from_hex("#12").is_err());
    assert!(Color::from_hex("#12345").is_err());
}

#[test]
fn test_css_functional_colors() {
    // rgb()
    assert_eq!(Color::parse_css("rgb(255, 0, 0)").unwrap(), Color::RED);
    assert_eq!(Color::parse_css("rgb(100%, 0%, 0%)").unwrap(), Color::RED);
    assert_eq!(Color::parse_css("rgb(0 255 0)").unwrap(), Color::LIME);

    // rgba()
    assert_eq!(
        Color::parse_css("rgba(0, 0, 255, 1.0)").unwrap(),
        Color::BLUE
    );
    assert_eq!(
        Color::parse_css("rgba(0, 0, 255, 0.5)").unwrap(),
        Color::from_rgba(0, 0, 255, 128)
    );
    assert_eq!(
        Color::parse_css("rgba(0, 0, 255, 50%)").unwrap(),
        Color::from_rgba(0, 0, 255, 128)
    );

    // hsl()
    let red_hsl = Color::parse_css("hsl(0, 100%, 50%)").unwrap();
    assert_eq!(red_hsl, Color::RED);

    let green_hsl = Color::parse_css("hsl(120, 100%, 50%)").unwrap();
    assert_eq!(green_hsl, Color::LIME);

    let blue_hsl = Color::parse_css("hsl(240deg, 100%, 50%)").unwrap();
    assert_eq!(blue_hsl, Color::BLUE);

    // Named colors
    assert_eq!(
        Color::parse_css("rebeccapurple").unwrap(),
        Color::from_rgb(102, 51, 153)
    );
    assert_eq!(Color::parse_css("transparent").unwrap(), Color::TRANSPARENT);
    assert_eq!(
        Color::parse_css("cornflowerblue").unwrap(),
        Color::from_rgb(100, 149, 237)
    );
}

#[test]
fn test_alpha_blending_source_over() {
    // Opaco sobre qualquer coisa substitui completamente
    let red = Color::RED;
    let blue = Color::BLUE;
    assert_eq!(red.blend_source_over(blue), Color::RED);

    // 50% vermelho sobre azul opaco
    let semi_red = Color::from_rgba(255, 0, 0, 128);
    let blended = semi_red.blend_source_over(blue);
    assert_eq!(blended.r, 128);
    assert_eq!(blended.g, 0);
    assert_eq!(blended.b, 127); // 255 * (1 - 0.5019) ≈ 127
    assert_eq!(blended.a, 255);

    // Transparente sobre azul mantém o azul
    let trans = Color::TRANSPARENT;
    assert_eq!(trans.blend_source_over(blue), Color::BLUE);
}

#[test]
fn test_color_lerp() {
    let black = Color::BLACK;
    let white = Color::WHITE;
    let mid = black.lerp(white, 0.5);
    assert_eq!(mid.r, 128);
    assert_eq!(mid.g, 128);
    assert_eq!(mid.b, 128);
    assert_eq!(mid.a, 255);
}

#[test]
fn test_premultiplied_f32() {
    let semi_white = Color::from_rgba(255, 255, 255, 128);
    let (r, g, b, a) = semi_white.to_premultiplied_f32();
    let expected_a = 128.0 / 255.0;
    assert!((a - expected_a).abs() < 1e-4);
    assert!((r - expected_a).abs() < 1e-4);
    assert!((g - expected_a).abs() < 1e-4);
    assert!((b - expected_a).abs() < 1e-4);
}
