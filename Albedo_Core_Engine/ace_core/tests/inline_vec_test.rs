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
