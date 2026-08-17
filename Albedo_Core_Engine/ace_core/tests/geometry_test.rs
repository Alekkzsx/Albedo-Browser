use ace_core::math::{
    point2, rect, size2, DpiScale, EdgeInsets, Point, Rect2D, RectExt, Size, Transform,
};

#[test]
fn test_rect_ext_inflate_deflate() {
    let r = Rect2D::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0));
    let insets = EdgeInsets::new(5.0, 10.0, 15.0, 20.0); // top, right, bottom, left

    // Inflate
    let inflated = r.inflate_edges(insets);
    assert_eq!(inflated.origin.x, -10.0); // 10 - 20
    assert_eq!(inflated.origin.y, 15.0);  // 20 - 5
    assert_eq!(inflated.size.width, 130.0); // 100 + 20 + 10
    assert_eq!(inflated.size.height, 70.0); // 50 + 5 + 15

    // Deflate
    let deflated = r.deflate_edges(insets);
    assert_eq!(deflated.origin.x, 30.0); // 10 + 20
    assert_eq!(deflated.origin.y, 25.0); // 20 + 5
    assert_eq!(deflated.size.width, 70.0); // 100 - 20 - 10
    assert_eq!(deflated.size.height, 30.0); // 50 - 5 - 15
}

#[test]
fn test_dpi_scaling_conversion() {
    let scale = DpiScale::new(1.5); // 150% Display Scaling
    let css_rect = rect(10.0, 20.0, 200.0, 100.0);

    let dev_rect = css_rect.to_device_rect(scale);
    assert_eq!(dev_rect.origin.x, 15.0);
    assert_eq!(dev_rect.origin.y, 30.0);
    assert_eq!(dev_rect.size.width, 300.0);
    assert_eq!(dev_rect.size.height, 150.0);

    assert_eq!(scale.to_css_pixels(300.0), 200.0);
}

#[test]
fn test_affine_transform_2d() {
    let translation = Transform::translation(50.0, 100.0);
    let p = point2(10.0, 20.0);
    let p_transformed = translation.transform_point(p);
    assert_eq!(p_transformed, point2(60.0, 120.0));

    let scale = Transform::scale(2.0, 3.0);
    let p_scaled = scale.transform_point(p);
    assert_eq!(p_scaled, point2(20.0, 60.0));

    // Composição de transformações (scale depois translate)
    let combined = scale.then(&translation);
    let p_comb = combined.transform_point(p);
    assert_eq!(p_comb, point2(70.0, 160.0));

    // Inversão de matriz
    let inv = combined.inverse().expect("Matriz deve ser inversível");
    let p_restored = inv.transform_point(p_comb);
    assert!((p_restored.x - p.x).abs() < 1e-4);
    assert!((p_restored.y - p.y).abs() < 1e-4);
}

#[test]
fn test_rect_intersection_and_union() {
    let r1 = rect(0.0, 0.0, 100.0, 100.0);
    let r2 = rect(50.0, 50.0, 100.0, 100.0);

    // Interseção
    let inter = r1.intersection(&r2).unwrap();
    assert_eq!(inter, rect(50.0, 50.0, 50.0, 50.0));

    // União
    let union = r1.union(&r2);
    assert_eq!(union, rect(0.0, 0.0, 150.0, 150.0));

    // Retângulos disjuntos
    let r3 = rect(200.0, 200.0, 50.0, 50.0);
    assert_eq!(r1.intersection(&r3), None);
}
