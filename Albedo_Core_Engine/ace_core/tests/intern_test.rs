use ace_core::intern::{atoms, Atom};
use std::collections::HashMap;

#[test]
fn test_atom_equality_and_static_atoms() {
    let a = Atom::new("div");
    let b = Atom::new("div");

    // Igualdade O(1)
    assert_eq!(a, b);
    assert_eq!(a.as_str(), "div");
    assert!(!a.is_empty());
    assert_eq!(a.len(), 3);
    assert_eq!(a.as_bytes(), b"div");

    // Comparações diretas com &str e String
    assert_eq!(a, "div");
    assert_eq!("div", a);
    assert_eq!(a, String::from("div"));
    assert_eq!(String::from("div"), a);
    assert!(a.eq_ignore_ascii_case("DIV"));

    // Átomos estáticos conhecidos
    assert_eq!(atoms::DIV(), a);
    assert_eq!(atoms::SPAN(), "span");
    assert_eq!(atoms::CLASS(), "class");
    assert_eq!(atoms::DISPLAY(), "display");
    assert_eq!(atoms::COLOR(), "color");

    // Deref para &str
    assert_eq!(&*atoms::BODY(), "body");

    // Uso com HashMap (lookup O(1) por Atom ID)
    let mut map = HashMap::new();
    map.insert(atoms::DIV(), 42);
    assert_eq!(map.get(&atoms::DIV()), Some(&42));
    assert_eq!(map.get(&Atom::from("div")), Some(&42));
    assert_eq!(map.get(&atoms::SPAN()), None);
}
