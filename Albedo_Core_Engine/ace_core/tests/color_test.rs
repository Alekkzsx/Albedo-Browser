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

#[test]
fn test_css_color_level_4_and_5() {
    // color(display-p3 ...)
    let p3_red = Color::parse("color(display-p3 1 0 0)").unwrap();
    assert_eq!(p3_red.r, 255);
    assert_eq!(p3_red.a, 255);

    let p3_alpha = Color::parse("color(display-p3 0 1 0 / 0.5)").unwrap();
    assert_eq!(p3_alpha.g, 255);
    assert_eq!(p3_alpha.a, 128);

    // hwb()
    let hwb_green = Color::parse("hwb(120deg 0% 0%)").unwrap();
    assert_eq!(hwb_green, Color::LIME);

    // light-dark()
    let light = Color::parse("light-dark(white, black)").unwrap();
    assert_eq!(light, Color::WHITE);

    // color-mix(in srgb, ...)
    let mixed_srgb = Color::parse("color-mix(in srgb, red 50%, blue 50%)").unwrap();
    assert_eq!(mixed_srgb.r, 128);
    assert_eq!(mixed_srgb.b, 128);
    assert_eq!(mixed_srgb.g, 0);

    // color-mix(in oklab, ...)
    let mixed_oklab = Color::parse("color-mix(in oklab, red, blue)").unwrap();
    assert!(mixed_oklab.r > 100);
    assert!(mixed_oklab.b > 100);
}

#[test]
fn test_w3c_bradford_chromatic_adaptation_lab_lch() {
    let colors = [
        Color::WHITE,
        Color::BLACK,
        Color::RED,
        Color::GREEN,
        Color::LIME,
        Color::BLUE,
        Color::YELLOW,
        Color::CYAN,
        Color::MAGENTA,
        Color::GRAY,
        Color::SILVER,
        Color::from_rgb(18, 52, 86),
        Color::from_rgb(255, 136, 0),
    ];

    // Reference white test in D50 Lab
    let (white_l, white_a, white_b, _) = Color::WHITE.to_lab();
    assert!((white_l - 100.0).abs() < 0.05, "White L* should be 100, got {}", white_l);
    assert!(white_a.abs() < 0.05, "White a* should be ~0, got {}", white_a);
    assert!(white_b.abs() < 0.05, "White b* should be ~0, got {}", white_b);

    // Reference black test in D50 Lab
    let (black_l, black_a, black_b, _) = Color::BLACK.to_lab();
    assert!(black_l.abs() < 0.05, "Black L* should be 0, got {}", black_l);
    assert!(black_a.abs() < 0.05, "Black a* should be ~0, got {}", black_a);
    assert!(black_b.abs() < 0.05, "Black b* should be ~0, got {}", black_b);

    // High precision roundtrips: Delta E < 0.001
    for &c in &colors {
        // Lab roundtrip
        let (l, a, b, alpha) = c.to_lab();
        let rt_lab = Color::from_lab(l, a, b, alpha);
        let delta_e_lab = c.delta_e_76(rt_lab);
        assert!(
            delta_e_lab < 0.001,
            "Delta E exceeds 0.001 for {:?}: got {} (rt: {:?})",
            c, delta_e_lab, rt_lab
        );
        assert_eq!(c, rt_lab, "Lab roundtrip equality failed for {:?}", c);

        // Lch roundtrip
        let (l, ch, h, alpha) = c.to_lch();
        let rt_lch = Color::from_lch(l, ch, h, alpha);
        let delta_e_lch = c.delta_e_76(rt_lch);
        assert!(
            delta_e_lch < 0.001,
            "Delta E (Lch) exceeds 0.001 for {:?}: got {} (rt: {:?})",
            c, delta_e_lch, rt_lch
        );
        assert_eq!(c, rt_lch, "Lch roundtrip equality failed for {:?}", c);
    }
}

#[test]
fn test_lab_lch_css_parsing() {
    // lab()
    let white_lab = Color::parse("lab(100% 0 0)").unwrap();
    assert_eq!(white_lab, Color::WHITE);

    let black_lab = Color::parse("lab(0% 0 0)").unwrap();
    assert_eq!(black_lab, Color::BLACK);

    // Red in D50 Lab is approx L=54.29, a=80.82, b=69.88
    let red_lab = Color::parse("lab(54.29% 80.82 69.88)").unwrap();
    assert_eq!(red_lab, Color::RED);

    // Comma-separated with alpha
    let semi_lab = Color::parse("lab(50%, 20, -30, 0.5)").unwrap();
    assert_eq!(semi_lab.a, 128);

    // Slash separated with alpha
    let semi_lab_slash = Color::parse("lab(50% 20 -30 / 50%)").unwrap();
    assert_eq!(semi_lab_slash.a, 128);

    // lch()
    let white_lch = Color::parse("lch(100% 0 0deg)").unwrap();
    assert_eq!(white_lch, Color::WHITE);

    let red_lch = Color::parse("lch(54.29% 106.84 40.85deg)").unwrap();
    assert_eq!(red_lch, Color::RED);

    // Different angle units: rad, grad, turn
    let lch_turn = Color::parse("lch(50% 30 0.5turn / 0.5)").unwrap();
    assert_eq!(lch_turn.a, 128);

    let lch_grad = Color::parse("lch(50% 30 200grad / 1.0)").unwrap();
    assert_eq!(lch_grad.a, 255);
}

#[test]
fn test_polar_hue_interpolation_methods() {
    use ace_core::math::color::{interpolate_hue, HueInterpolation};

    // 1. Shorter: shortest path on circle
    // 10deg to 350deg -> spans 20deg crossing 0/360
    let h_short = interpolate_hue(10.0, 350.0, 0.5, HueInterpolation::Shorter);
    assert!((h_short - 0.0).abs() < 1e-4 || (h_short - 360.0).abs() < 1e-4);

    let h_short2 = interpolate_hue(10.0, 50.0, 0.5, HueInterpolation::Shorter);
    assert!((h_short2 - 30.0).abs() < 1e-4);

    // 2. Longer: longest path on circle (>= 180deg)
    // 10deg to 50deg -> spans 320deg, midpoint at 210deg
    let h_long = interpolate_hue(10.0, 50.0, 0.5, HueInterpolation::Longer);
    assert!((h_long - 210.0).abs() < 1e-4);

    // 3. Increasing: angles must increase (counter-clockwise)
    // 300deg to 60deg -> 300 -> 360/0 -> 60, midpoint at 0/360deg
    let h_inc = interpolate_hue(300.0, 60.0, 0.5, HueInterpolation::Increasing);
    assert!((h_inc - 0.0).abs() < 1e-4 || (h_inc - 360.0).abs() < 1e-4);

    // 4. Decreasing: angles must decrease (clockwise)
    // 60deg to 300deg -> 60 -> 0/360 -> 300, midpoint at 0/360deg
    let h_dec = interpolate_hue(60.0, 300.0, 0.5, HueInterpolation::Decreasing);
    assert!((h_dec - 0.0).abs() < 1e-4 || (h_dec - 360.0).abs() < 1e-4);

    // Color::interpolate_oklch with different methods
    let c1 = Color::from_rgb(255, 0, 0);   // Red
    let c2 = Color::from_rgb(0, 0, 255);   // Blue

    let mid_short = c1.interpolate_oklch(c2, 0.5, HueInterpolation::Shorter);
    let mid_long = c1.interpolate_oklch(c2, 0.5, HueInterpolation::Longer);
    assert_ne!(mid_short, mid_long, "Shorter and Longer should yield distinct hues");

    // Color::interpolate_hsl
    let hsl_inc = c1.interpolate_hsl(c2, 0.5, HueInterpolation::Increasing);
    let hsl_dec = c1.interpolate_hsl(c2, 0.5, HueInterpolation::Decreasing);
    assert_ne!(hsl_inc, hsl_dec, "Increasing and Decreasing should yield distinct hues");
}

#[test]
fn test_powerless_components() {
    use ace_core::math::color::HueInterpolation;

    // Gray is achromatic (chroma == 0 / saturation == 0) -> hue is powerless
    let gray = Color::from_rgb(128, 128, 128);
    let red = Color::from_rgb(255, 0, 0);

    // Interpolating gray and red in Oklch: gray adopts red's hue
    let mid = gray.interpolate_oklch(red, 0.5, HueInterpolation::Shorter);
    let red_oklch = red.to_oklch();
    let mid_oklch = mid.to_oklch();

    // Hue should match red's hue closely
    assert!(
        (mid_oklch.h - red_oklch.h).abs() < 2.0,
        "Powerless hue should adopt red's hue: mid.h={}, red.h={}",
        mid_oklch.h, red_oklch.h
    );

    // In HSL
    let mid_hsl = gray.interpolate_hsl(red, 0.5, HueInterpolation::Shorter);
    let (red_h, _, _, _) = red.to_hsla();
    let (mid_h, _, _, _) = mid_hsl.to_hsla();
    assert!(
        (mid_h - red_h).abs() < 2.0,
        "Powerless HSL hue should adopt red's hue: mid.h={}, red.h={}",
        mid_h, red_h
    );
}

#[test]
fn test_alpha_premultiplication_in_polar_and_lab_interpolation() {
    use ace_core::math::color::HueInterpolation;

    // Fully opaque red and 50% semi-transparent blue
    let opaque_red = Color::from_rgba(255, 0, 0, 255);
    let semi_blue = Color::from_rgba(0, 0, 255, 128);

    // Output alpha should be 1.0 * 0.5 + 0.5019 * 0.5 ≈ 0.751
    let mid_lab = opaque_red.lerp_lab(semi_blue, 0.5);
    assert_eq!(mid_lab.a, 192); // (255 + 128) / 2 = 191.5 ≈ 192

    let mid_oklch = opaque_red.interpolate_oklch(semi_blue, 0.5, HueInterpolation::Shorter);
    assert_eq!(mid_oklch.a, 192);

    // Pure transparent + opaque red: color channel is preserved without dark/black contamination
    let trans = Color::from_rgba(0, 0, 0, 0);
    let mid_trans = trans.interpolate_oklch(opaque_red, 0.5, HueInterpolation::Shorter);
    assert_eq!(mid_trans.a, 128);
    // Red component should be preserved as 255
    assert_eq!(mid_trans.r, 255);
}

#[test]
fn test_color_mix_css_level_4_and_5_advanced() {
    // color-mix in lab
    let mix_lab = Color::parse("color-mix(in lab, red 50%, blue 50%)").unwrap();
    assert_eq!(mix_lab.a, 255);

    // color-mix in lch with hue method
    let mix_lch_short = Color::parse("color-mix(in lch shorter hue, red, blue)").unwrap();
    let mix_lch_long = Color::parse("color-mix(in lch longer hue, red, blue)").unwrap();
    assert_ne!(mix_lch_short, mix_lch_long);

    // color-mix in oklch
    let mix_oklch = Color::parse("color-mix(in oklch decreasing hue, red 30%, blue 70%)").unwrap();
    assert_eq!(mix_oklch.a, 255);

    // color-mix in hsl
    let mix_hsl = Color::parse("color-mix(in hsl increasing hue, red, blue)").unwrap();
    assert_eq!(mix_hsl.a, 255);
}

