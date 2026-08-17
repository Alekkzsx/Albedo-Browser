use ace_core::collections::AncestorFilter;
use ace_core::intern::Atom;

#[test]
fn test_ancestor_filter_push_pop_lifecycle() {
    let mut filter = AncestorFilter::<256>::new();

    let tag_div = Atom::new("div");
    let class_container = Atom::new("container");
    let tag_p = Atom::new("p");

    assert!(!filter.may_contain(&tag_div));
    assert!(!filter.may_contain(&class_container));

    // Entra no <div>
    filter.push(&tag_div);
    filter.push(&class_container);

    assert!(filter.may_contain(&tag_div));
    assert!(filter.may_contain(&class_container));
    assert!(!filter.may_contain(&tag_p)); // <p> definitivamente não está nos ancestrais

    // Sai do <div>
    filter.pop(&tag_div);
    filter.pop(&class_container);

    assert!(!filter.may_contain(&tag_div));
    assert!(!filter.may_contain(&class_container));
}

#[test]
fn test_ancestor_filter_nested_duplicate_counts() {
    let mut filter = AncestorFilter::<256>::new();
    let tag_div = Atom::new("div");

    // Entra em <div> aninhado dentro de <div>
    filter.push(&tag_div);
    filter.push(&tag_div);

    assert!(filter.may_contain(&tag_div));

    // Sai do <div> interno
    filter.pop(&tag_div);
    assert!(filter.may_contain(&tag_div)); // Ainda presente no <div> externo!

    // Sai do <div> externo
    filter.pop(&tag_div);
    assert!(!filter.may_contain(&tag_div));
}
