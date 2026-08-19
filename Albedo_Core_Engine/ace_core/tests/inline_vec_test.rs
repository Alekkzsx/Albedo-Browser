use ace_core::collections::InlineVec;

#[test]
fn test_inline_vec_stack_and_spill_to_heap() {
    let mut vec: InlineVec<String, 4> = InlineVec::new();
    assert!(vec.is_empty());
    assert_eq!(vec.len(), 0);
    assert!(vec.is_inline());

    vec.push("item1".to_string());
    vec.push("item2".to_string());
    vec.push("item3".to_string());
    vec.push("item4".to_string());

    assert_eq!(vec.len(), 4);
    assert!(vec.is_inline()); // Ainda na stack
    assert_eq!(vec[0], "item1");
    assert_eq!(vec[3], "item4");

    // 5º item força o spill para o Heap
    vec.push("item5".to_string());
    assert_eq!(vec.len(), 5);
    assert!(!vec.is_inline()); // Agora no Heap!
    assert_eq!(vec[4], "item5");

    // Pop
    assert_eq!(vec.pop(), Some("item5".to_string()));
    assert_eq!(vec.len(), 4);

    // Iteração e fatiamento
    let items: Vec<&str> = vec.iter().map(|s| s.as_str()).collect();
    assert_eq!(items, vec!["item1", "item2", "item3", "item4"]);

    // Clone
    let cloned = vec.clone();
    assert_eq!(cloned, vec);

    vec.clear();
    assert!(vec.is_empty());
}

#[test]
fn test_inline_vec_drain_and_truncate() {
    let mut vec: InlineVec<i32, 4> = InlineVec::new();
    vec.extend([10, 20, 30, 40]);

    assert_eq!(vec.first(), Some(&10));
    assert_eq!(vec.last(), Some(&40));
    assert_eq!(vec.get(2), Some(&30));

    // Drain partial range
    let drained = vec.drain(1..3);
    assert_eq!(drained, vec![20, 30]);
    assert_eq!(vec.as_slice(), &[10, 40]);

    // Truncate
    vec.truncate(1);
    assert_eq!(vec.as_slice(), &[10]);
    assert_eq!(vec.len(), 1);

    // Spill to heap and drain
    vec.extend([20, 30, 40, 50]);
    assert!(!vec.is_inline());
    let drained_heap = vec.drain(1..4);
    assert_eq!(drained_heap, vec![20, 30, 40]);
    assert_eq!(vec.as_slice(), &[10, 50]);
}

