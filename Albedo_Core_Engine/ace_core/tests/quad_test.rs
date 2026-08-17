use ace_core::math::geometry::{point2, rect, Matrix4D, Point, Transform};
use ace_core::math::quad::CssQuad;
use ace_core::math::units::CssPixel;
use euclid::Point2D;

#[test]
fn test_quad_from_rect_and_bounding_box() {
    let r = rect(10.0, 20.0, 100.0, 50.0);
    let quad = CssQuad::from_rect(&r);

    let bbox = quad.bounding_box();
    assert_eq!(bbox.origin.x, 10.0);
    assert_eq!(bbox.origin.y, 20.0);
    assert_eq!(bbox.size.width, 100.0);
    assert_eq!(bbox.size.height, 50.0);
}

#[test]
fn test_quad_contains_point_basic() {
    let r = rect(0.0, 0.0, 100.0, 100.0);
    let quad = CssQuad::from_rect(&r);

    assert!(quad.contains_point(&point2(50.0, 50.0)));
    assert!(quad.contains_point(&point2(10.0, 10.0)));
    assert!(!quad.contains_point(&point2(150.0, 50.0)));
    assert!(!quad.contains_point(&point2(-10.0, 50.0)));
}

#[test]
fn test_quad_transformed_rotated_hit_testing() {
    let r = rect(-50.0, -50.0, 100.0, 100.0);
    // Rotação de 45 graus em torno da origem (0,0)
    let rot = Transform::<f32, CssPixel, CssPixel>::rotation(0.0, 0.0, euclid::Angle::degrees(45.0));
    let quad = CssQuad::from_transformed_rect(&r, &rot);

    // O ponto central está contido
    assert!(quad.contains_point(&point2(0.0, 0.0)));

    // Em 45 graus, o topo estende até ~70.7px
    assert!(quad.contains_point(&point2(0.0, 60.0)));

    // Um ponto no canto do bounding box original fora do losango
    assert!(!quad.contains_point(&point2(60.0, 60.0)));
}
