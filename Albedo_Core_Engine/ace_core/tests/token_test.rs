use ace_core::security::UnguessableToken;

#[test]
fn test_unguessable_token_uniqueness_and_formatting() {
    let t1 = UnguessableToken::new();
    let t2 = UnguessableToken::new();

    assert_ne!(t1, t2);
    assert!(!t1.is_empty());
    assert!(!t2.is_empty());

    let hex = t1.to_hex();
    assert_eq!(hex.len(), 32);

    let reconstructed = UnguessableToken::from_raw(t1.high(), t1.low());
    assert_eq!(t1, reconstructed);
    assert_eq!(format!("{}", t1), hex);
}
