use ace_core::intern::{intern, resolve};

#[test]
fn test_string_interner() {
    let sym1 = intern("display");
    let sym2 = intern("margin");
    let sym3 = intern("display");
    
    assert_ne!(sym1, sym2);
    assert_eq!(sym1, sym3); // Desduplicação O(1) funciona!
    
    assert_eq!(resolve(sym1), Some("display"));
    assert_eq!(resolve(sym2), Some("margin"));
}
