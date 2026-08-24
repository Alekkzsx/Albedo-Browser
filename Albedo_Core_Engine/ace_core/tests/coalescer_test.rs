use ace_core::event_loop::InputEventCoalescer;

#[test]
fn test_input_event_coalescing() {
    let mut coalescer = InputEventCoalescer::new();
    assert!(!coalescer.has_pending());

    coalescer.push_movement(10.0, 20.0, 5.0, 5.0, 1000);
    coalescer.push_movement(15.0, 25.0, 5.0, 5.0, 1010);
    coalescer.push_movement(20.0, 30.0, 5.0, 5.0, 1020);

    assert!(coalescer.has_pending());

    let event = coalescer.drain().expect("deve conter evento coalescido");
    assert_eq!(event.last_x, 20.0);
    assert_eq!(event.last_y, 30.0);
    assert_eq!(event.delta_x, 15.0); // 5 + 5 + 5
    assert_eq!(event.delta_y, 15.0);
    assert_eq!(event.count, 3);
    assert_eq!(event.timestamp_us, 1020);

    assert!(!coalescer.has_pending());
    assert_eq!(coalescer.drain(), None);
}
