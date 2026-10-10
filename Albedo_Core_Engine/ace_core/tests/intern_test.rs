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

    // Uso com HashMap (lookup O(1) por Atom ID e Borrow<str>)
    let mut map = HashMap::new();
    map.insert(atoms::DIV(), 42);
    assert_eq!(map.get(&atoms::DIV()), Some(&42));
    assert_eq!(map.get(&Atom::from("div")), Some(&42));
    assert_eq!(map.get("div"), Some(&42)); // Lookup zero-copy via Borrow<str>
    assert_eq!(map.get(&atoms::SPAN()), None);
    assert_eq!(map.get("span"), None);

    // Átomo estático vazio e Default
    let empty_atom = Atom::default();
    assert_eq!(empty_atom, atoms::EMPTY());
    assert_eq!(empty_atom, Atom::new(""));
    assert_eq!(empty_atom, "");
    assert!(empty_atom.is_empty());
    assert_eq!(empty_atom.len(), 0);
    assert_eq!(empty_atom.as_bytes(), b"");
}
