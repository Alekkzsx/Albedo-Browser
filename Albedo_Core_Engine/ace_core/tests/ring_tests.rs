use ace_core::ring::RingBuffer;

#[test]
fn test_spsc_ring_buffer() {
    let ring = RingBuffer::new(3);
    assert!(ring.push(10).is_ok());
    assert!(ring.push(20).is_ok());
    assert!(ring.push(30).is_ok());

    // A capacidade máxima é 3, o quarto vai falhar
    assert!(ring.push(40).is_err());

    assert_eq!(ring.pop(), Some(10));
    assert_eq!(ring.pop(), Some(20));

    assert!(ring.push(40).is_ok());

    assert_eq!(ring.pop(), Some(30));
    assert_eq!(ring.pop(), Some(40));
    assert_eq!(ring.pop(), None);
}
