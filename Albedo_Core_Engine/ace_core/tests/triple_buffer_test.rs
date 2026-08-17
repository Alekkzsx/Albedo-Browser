use ace_core::collections::triple_buffer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_triple_buffer_single_thread() {
    let (mut producer, mut consumer) = triple_buffer(0u32);

    assert_eq!(consumer.consume(), None); // Nenhum novo frame publicado ainda

    producer.write(42);
    producer.publish();

    assert_eq!(consumer.consume(), Some(42));
    assert_eq!(consumer.consume(), None); // Já consumido

    producer.write_with(|val| *val = 100);
    producer.publish();

    assert_eq!(consumer.consume(), Some(100));
}

#[test]
fn test_triple_buffer_multi_threaded_sync() {
    let (mut producer, mut consumer) = triple_buffer(0u32);
    let is_running = Arc::new(AtomicBool::new(true));
    let run_producer = Arc::clone(&is_running);

    // Thread de Renderização (Produtor a alta frequência)
    let producer_thread = thread::spawn(move || {
        let mut frame_count = 0;
        while run_producer.load(Ordering::Relaxed) {
            frame_count += 1;
            producer.write(frame_count);
            producer.publish();
            thread::sleep(Duration::from_millis(1));
        }
        frame_count
    });

    // Thread da GPU (Consumidor)
    let mut last_seen = 0;
    for _ in 0..10 {
        if let Some(frame) = consumer.consume() {
            assert!(frame >= last_seen);
            last_seen = frame;
        }
        thread::sleep(Duration::from_millis(2));
    }

    is_running.store(false, Ordering::Relaxed);
    let total_produced = producer_thread.join().unwrap();
    assert!(total_produced > 0);
}
