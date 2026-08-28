//! # Bateria de Testes do Motor de Range DOM (ace_dom)

use ace_dom::node::element::Namespace;
use ace_dom::range::{BoundaryPoint, Range, RangeComparison};
use ace_dom::tree::Document;

#[test]
fn test_dom_range_boundary_comparison() {
    let mut doc = Document::new(None);
    let parent = doc.create_element("div", Namespace::Html);
    let c1 = doc.create_element("p", Namespace::Html);
    let c2 = doc.create_element("p", Namespace::Html);

    doc.append_child(parent, c1).unwrap();
    doc.append_child(parent, c2).unwrap();
    let root = doc.root();
    doc.append_child(root, parent).unwrap();

    let r1 = Range::from_points(BoundaryPoint::new(c1, 0), BoundaryPoint::new(c1, 5));
    let r2 = Range::from_points(BoundaryPoint::new(c2, 0), BoundaryPoint::new(c2, 3));

    // c1 precede c2 na ordem do documento
    let cmp = r1.compare_boundary_points(&doc, RangeComparison::StartToStart, &r2).unwrap();
    assert_eq!(cmp, -1);
}

#[test]
fn test_dom_range_common_ancestor_and_clone() {
    let mut doc = Document::new(None);
    let parent = doc.create_element("div", Namespace::Html);
    let c1 = doc.create_element("span", Namespace::Html);
    let c2 = doc.create_element("strong", Namespace::Html);

    doc.append_child(parent, c1).unwrap();
    doc.append_child(parent, c2).unwrap();
    let root = doc.root();
    doc.append_child(root, parent).unwrap();

    let r = Range::from_points(BoundaryPoint::new(c1, 0), BoundaryPoint::new(c2, 1));
    assert_eq!(r.common_ancestor_container(&doc), parent);

    let frag_id = r.clone_contents(&mut doc).unwrap();
    assert!(doc.children(frag_id).count() >= 2);
}
