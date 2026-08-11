// ============================================================================
// Albedo Core Engine (ACE)
// File: tests/math_tests.rs
// Description: Testes de integração do motor matemático de base.
//              Garante conformidade algorítmica e zero regressão.
// Author: Albedo Browser Engineering Team
// ============================================================================

use ace_core::math::*;

#[test]
fn test_point_rect() {
    let p = Point::new(10.0, 20.0);
    let rect = Rect::new(5.0, 5.0, 10.0, 20.0);
    
    assert!(rect.contains(&p));
    assert_eq!(rect.right(), 15.0);
    assert_eq!(rect.bottom(), 25.0);
}

#[test]
fn test_vec2_math() {
    let v1 = Vec2::new(1, 2);
    let v2 = Vec2::new(3, 4);
    
    let sum = v1 + v2;
    assert_eq!(sum.x, 4);
    assert_eq!(sum.y, 6);
    
    let sub = v2 - v1;
    assert_eq!(sub.x, 2);
    assert_eq!(sub.y, 2);
}

#[test]
fn test_clamp_lerp() {
    assert_eq!(clamp(5, 0, 10), 5);
    assert_eq!(clamp(-5, 0, 10), 0);
    assert_eq!(clamp(15, 0, 10), 10);
    
    assert_eq!(lerp(0.0, 100.0, 0.5), 50.0);
}

#[test]
fn test_vec3_math() {
    let v1 = Vec3::new(1.0, 2.0, 3.0);
    let v2 = Vec3::new(4.0, 5.0, 6.0);
    let sum = v1 + v2;
    
    assert_eq!(sum.x, 5.0);
    assert_eq!(sum.y, 7.0);
    assert_eq!(sum.z, 9.0);
}

#[test]
fn test_matrices_default() {
    let m3 = Matrix3x3::<f32>::default();
    assert_eq!(m3.data.len(), 9);
    assert_eq!(m3.data[0], 0.0);
    
    let m4 = Matrix4x4::<f32>::default();
    assert_eq!(m4.data.len(), 16);
    assert_eq!(m4.data[15], 0.0);
}
