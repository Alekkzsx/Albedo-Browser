use ace_core::intern::Atom;

#[test]
fn test_atom_equality_is_fast() {
    let a = Atom::new("div");
    let b = Atom::new("div");
    
    // As duas strings diferentes apontam para o mesmo ID na string_cache.
    assert_eq!(a, b);
    assert_eq!(a.as_str(), "div");
}
