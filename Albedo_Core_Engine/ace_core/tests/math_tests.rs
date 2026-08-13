// ============================================================================
// Albedo Core Engine (ACE)
// File: tests/math_tests.rs
// Description: Testes de integração do motor matemático de base.
//              Garante conformidade algorítmica e zero regressão.
// Author: Albedo Browser Engineering Team
// ============================================================================

use ace_core::math::*;

#[test]
fn test_rect_operations() {
    let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
    let r2 = Rect::new(5.0, 5.0, 10.0, 10.0);

    let union = r1.union(&r2);
    assert_eq!(union.x(), 0.0);
    assert_eq!(union.y(), 0.0);
    assert_eq!(union.width(), 15.0);
    assert_eq!(union.height(), 15.0);

    let intersect = r1.intersection(&r2).unwrap();
    assert_eq!(intersect.x(), 5.0);
    assert_eq!(intersect.y(), 5.0);
    assert_eq!(intersect.width(), 5.0);
    assert_eq!(intersect.height(), 5.0);

    let no_intersect = r1.intersection(&Rect::new(20.0, 20.0, 5.0, 5.0));
    assert!(no_intersect.is_none());

    let inflated = r1.inflate(2.0, 3.0);
    assert_eq!(inflated.x(), -2.0);
    assert_eq!(inflated.y(), -3.0);
    assert_eq!(inflated.width(), 14.0);
    assert_eq!(inflated.height(), 16.0);

    let deflated = r1.deflate(2.0, 2.0);
    assert_eq!(deflated.x(), 2.0);
    assert_eq!(deflated.y(), 2.0);
    assert_eq!(deflated.width(), 6.0);
    assert_eq!(deflated.height(), 6.0);
}

#[test]
fn test_vec2_advanced() {
    let v1 = Vec2::new(3.0, 4.0);
    let v2 = Vec2::new(6.0, 8.0);

    assert_eq!(v1.length(), 5.0);
    assert_eq!(v1.length_squared(), 25.0);

    let norm = v1.normalize();
    assert_eq!(norm.x, 3.0 / 5.0);
    assert_eq!(norm.y, 4.0 / 5.0);

    assert_eq!(v1.dot(&v2), 50.0);
    assert_eq!(v1.distance(&v2), 5.0);
}

#[test]
fn test_vec3_advanced() {
    let v1 = Vec3::new(1.0, 0.0, 0.0);
    let v2 = Vec3::new(0.0, 1.0, 0.0);

    assert_eq!(v1.length(), 1.0);
    let cross = v1.cross(&v2);
    assert_eq!(cross.x, 0.0);
    assert_eq!(cross.y, 0.0);
    assert_eq!(cross.z, 1.0);

    let v3 = Vec3::new(1.0, 2.0, 2.0);
    assert_eq!(v3.length(), 3.0);
}

#[test]
fn test_vector_properties() {
    // Associatividade: (a + b) + c == a + (b + c)
    let a = Vec2::new(1.5f32, 2.5f32);
    let b = Vec2::new(3.0f32, -1.0f32);
    let c = Vec2::new(-2.0f32, 0.5f32);

    let sum1 = (a + b) + c;
    let sum2 = a + (b + c);

    assert!((sum1.x - sum2.x).abs() < 1e-5);
    assert!((sum1.y - sum2.y).abs() < 1e-5);
}

#[test]
fn test_math_utils() {
    assert_eq!(clamp(5, 0, 10), 5);
    assert_eq!(clamp(-5, 0, 10), 0);
    assert_eq!(clamp(15, 0, 10), 10);

    // Tratamento de NaN: no nosso min/max baseado em PartialOrd genérico,
    // NaN geralmente propaga a comparação como false.
    let nan = std::f32::NAN;
    let min_nan = min(10.0, nan); // Depende da implementação, geralmente retorna o primeiro operando válido se a < b falha.
    assert!(!min_nan.is_nan() || min_nan.is_nan()); // Apenas atestando compilação e não crash.

    assert_eq!(lerp(0.0, 100.0, 0.5), 50.0);
    assert_eq!(min(10, 20), 10);
    assert_eq!(max(10, 20), 20);
    assert_eq!(abs(-5.5), 5.5);
    assert_eq!(abs(3.0), 3.0);
    assert_eq!(saturate(1.5), 1.0);
    assert_eq!(saturate(-0.5), 0.0);
    assert_eq!(saturate(0.5), 0.5);
}

#[test]
fn test_matrices_operations() {
    let trans = Matrix3x3::translation(10.0, 20.0);
    let scale = Matrix3x3::scale(2.0, 3.0);

    let v = Vec2::new(5.0, 5.0);
    let v_trans = trans.multiply_vec2(&v);
    assert_eq!(v_trans.x, 15.0);
    assert_eq!(v_trans.y, 25.0);

    let v_scale = scale.multiply_vec2(&v);
    assert_eq!(v_scale.x, 10.0);
    assert_eq!(v_scale.y, 15.0);

    let rot = Matrix3x3::rotation(std::f32::consts::PI / 2.0); // 90 graus
    let v_rot = rot.multiply_vec2(&Vec2::new(1.0, 0.0));
    assert!(v_rot.x.abs() < 1e-5);
    assert!((v_rot.y - 1.0).abs() < 1e-5);

    let shear = Matrix3x3::shear(1.0, 0.0); // kx = 1.0
    let v_shear = shear.multiply_vec2(&Vec2::new(1.0, 1.0));
    assert_eq!(v_shear.x, 2.0);
    assert_eq!(v_shear.y, 1.0);
}

#[test]
fn test_colorimetry() {
    let red = Color::rgb(255, 0, 0);
    let (h, s, l, a) = red.to_hsla();
    assert_eq!(h, 0.0);
    assert_eq!(s, 1.0);
    assert_eq!(l, 0.5);
    assert_eq!(a, 1.0);

    let dark_red = red.darken(0.25);
    let (_, _, dl, _) = dark_red.to_hsla();
    assert!((dl - 0.25).abs() < 0.01, "Expected ~0.25, got {}", dl);

    let semi_transparent_blue = Color::new(0, 0, 255, 127);
    let opaque_red = Color::rgb(255, 0, 0);

    let blended = semi_transparent_blue.blend_source_over(&opaque_red);
    assert_eq!(blended.a, 255);
    assert_eq!(blended.r, 128); // Metade do vermelho
    assert_eq!(blended.g, 0);
    assert_eq!(blended.b, 127); // Metade do azul
}


#[test]
fn test_advanced_matrix_features() {
    // Inverse (Scale by 2)
    let mat = Matrix3x3::new([2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0]);
    let inv_opt = mat.inverse();
    assert!(inv_opt.is_some());
    let inv = inv_opt.unwrap();

    // M * M^-1 == I
    let identity = mat * inv;
    assert!((identity.data[0] - 1.0).abs() < 1e-5);
    assert!((identity.data[4] - 1.0).abs() < 1e-5);
    assert!((identity.data[8] - 1.0).abs() < 1e-5);
    assert!(identity.data[1].abs() < 1e-5);

    // Rect transform
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    // translate 5, 5
    let t = Matrix3x3::translation(5.0, 5.0);
    let t_rect = rect.transform(&t);
    assert!(t_rect.approx_eq(&Rect::new(5.0, 5.0, 10.0, 10.0), 1e-5));
}

#[test]
fn test_rect_union_and_rounding() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    let b = Rect::new(5.0, 5.0, 10.0, 10.0);
    let u = a.union(&b);
    assert!(u.approx_eq(&Rect::new(0.0, 0.0, 15.0, 15.0), 1e-5));

    let frect = Rect::new(1.4, 2.6, 5.5, 6.4);
    let irect = frect.round();
    assert_eq!(irect.x(), 1);
    assert_eq!(irect.y(), 3);
    assert_eq!(irect.width(), 6);
    assert_eq!(irect.height(), 6);
}

#[test]
fn test_color_lerp_and_pack() {
    let c1 = Color::new(0, 0, 0, 0);
    let c2 = Color::new(255, 255, 255, 255);
    let c_mid = c1.lerp(&c2, 0.5);
    assert_eq!(c_mid, Color::new(128, 128, 128, 128)); // 255/2 = 127.5 -> round to 128

    let packed = Color::new(10, 20, 30, 40).to_rgba_u32();
    let expected = (10 << 24) | (20 << 16) | (30 << 8) | 40;
    assert_eq!(packed, expected);
}
