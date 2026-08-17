use ace_core::collections::RingBuffer;

#[test]
fn test_ring_buffer_push_and_rotation() {
    let mut rb: RingBuffer<i32, 3> = RingBuffer::new();
    assert!(rb.is_empty());
    assert_eq!(rb.capacity(), 3);

    assert_eq!(rb.push(10), None);
    assert_eq!(rb.push(20), None);
    assert_eq!(rb.push(30), None);
    assert!(rb.is_full());
    assert_eq!(rb.len(), 3);

    // Sobrescrita do elemento mais antigo (10)
    let old = rb.push(40);
    assert_eq!(old, Some(10));
    assert_eq!(rb.len(), 3);

    // Ordem cronológica esperada: [20, 30, 40]
    assert_eq!(rb.get(0), Some(&20));
    assert_eq!(rb.get(1), Some(&30));
    assert_eq!(rb.get(2), Some(&40));
    assert_eq!(rb.get(3), None);

    let collected: Vec<i32> = rb.iter().copied().collect();
    assert_eq!(collected, vec![20, 30, 40]);
}
