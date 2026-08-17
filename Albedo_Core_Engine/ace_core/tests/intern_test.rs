use ace_core::intern::{atoms, Atom};

#[test]
fn test_atom_equality_and_static_atoms() {
    let a = Atom::new("div");
    let b = Atom::new("div");

    // Igualdade O(1)
    assert_eq!(a, b);
    assert_eq!(a.as_str(), "div");

    // Átomos estáticos conhecidos
    assert_eq!(atoms::DIV(), a);
    assert_eq!(atoms::SPAN().as_str(), "span");
    assert_eq!(atoms::CLASS().as_str(), "class");
    assert_eq!(atoms::DISPLAY().as_str(), "display");
    assert_eq!(atoms::COLOR().as_str(), "color");

    // Deref para &str
    assert_eq!(&*atoms::BODY(), "body");
}
