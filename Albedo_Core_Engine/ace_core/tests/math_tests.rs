// ============================================================================
// Albedo Core Engine (ACE)
// File: tests/math_tests.rs
// Description: Testes de integração do motor matemático de base.
//              Garante conformidade algorítmica e zero regressão.
// Author: Albedo Browser Engineering Team
// ============================================================================

use ace_core::math::*;
use ace_core::time::*;

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
fn test_math_utils() {
    assert_eq!(clamp(5, 0, 10), 5);
    assert_eq!(clamp(-5, 0, 10), 0);
    assert_eq!(clamp(15, 0, 10), 10);
    
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
fn test_mock_clock() {
    MockClock::reset();
    assert_eq!(MockClock::now_ms(), 0);
    
    MockClock::advance(100);
    assert_eq!(MockClock::now_ms(), 100);
    
    MockClock::tick_frame();
    assert_eq!(MockClock::now_ms(), 116);
    
    let vt = VirtualTime;
    assert_eq!(vt.now_ms(), 116);
}
