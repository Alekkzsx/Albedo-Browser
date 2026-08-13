use ace_core::rope::Rope;

#[test]
fn test_rope_basic_creation() {
    let r = Rope::from_str("Hello");
    assert_eq!(r.len(), 5);
    assert_eq!(r.to_string(), "Hello");
}

#[test]
fn test_rope_concatenation_zero_copy() {
    let r1 = Rope::from_str("Albedo");
    let r2 = Rope::from_str(" ");
    let r3 = Rope::from_str("Browser");

    // Estas operações rodam em O(1) gerando apenas um novo Nó Concat (24 bytes)
    // independentemente do tamanho das strings.
    let r4 = Rope::concat(&r1, &r2);
    let final_rope = Rope::concat(&r4, &r3);

    assert_eq!(final_rope.len(), 14);
    assert_eq!(final_rope.to_string(), "Albedo Browser");
}

#[test]
fn test_rope_empty() {
    let r1 = Rope::new();
    let r2 = Rope::from_str("Test");

    let r3 = Rope::concat(&r1, &r2);
    assert_eq!(r3.len(), 4);
    assert_eq!(r3.to_string(), "Test");
}
