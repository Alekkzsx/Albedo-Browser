use ace_core::collections::triple_buffer::{triple_buffer, triple_buffer_with, TripleBuffer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_triple_buffer_single_thread() {
    let (mut producer, mut consumer) = triple_buffer(0u32);

    assert_eq!(consumer.consume(), None); // Nenhum novo frame publicado ainda
    assert_eq!(*consumer.read(), 0);

    producer.write(42);
    producer.publish();

    assert_eq!(consumer.consume(), Some(&42));
    assert_eq!(consumer.consume(), None); // Já consumido

    producer.write_with(|val| *val = 100);
    producer.publish();

    assert_eq!(consumer.consume(), Some(&100));
    assert_eq!(consumer.consume(), None);
}

#[test]
fn test_triple_buffer_struct_constructor() {
    let (mut producer, mut consumer) = TripleBuffer::new(10i32);
    assert_eq!(*consumer.read(), 10);
    producer.write(20);
    producer.publish();
    assert_eq!(consumer.updated(), Some(&20));
}

#[test]
fn test_triple_buffer_dirty_flag_multiple_writes() {
    let (mut producer, mut consumer) = triple_buffer(0u32);

    // Escreve múltiplos frames antes de qualquer leitura
    producer.write(1);
    producer.publish();

    producer.write(2);
    producer.publish();

    producer.write(3);
    producer.publish();

    // O consumidor deve obter o frame mais recente (3)
    assert_eq!(consumer.consume(), Some(&3));
    assert_eq!(consumer.consume(), None);
}

#[test]
fn test_triple_buffer_non_clone_type() {
    struct NonCloneFrame {
        data: Vec<u8>,
        id: usize,
    }

    let (mut producer, mut consumer) = triple_buffer_with(|| NonCloneFrame {
        data: vec![1, 2, 3],
        id: 0,
    });

    producer.write_with(|frame| {
        frame.id = 42;
        frame.data.push(4);
    });
    producer.publish();

    let consumed = consumer.consume().unwrap();
    assert_eq!(consumed.id, 42);
    assert_eq!(consumed.data.as_slice(), &[1, 2, 3, 4]);
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
    for _ in 0..15 {
        if let Some(&frame) = consumer.consume() {
            assert!(frame >= last_seen, "Frame regression detected: {} < {}", frame, last_seen);
            last_seen = frame;
        }
        thread::sleep(Duration::from_millis(2));
    }

    is_running.store(false, Ordering::Relaxed);
    let total_produced = producer_thread.join().unwrap();
    assert!(total_produced > 0);
}

#[test]
fn test_triple_buffer_high_throughput_stress() {
    let (mut producer, mut consumer) = triple_buffer(0usize);
    let iterations = 100_000;

    let producer_thread = thread::spawn(move || {
        for i in 1..=iterations {
            producer.write(i);
            producer.publish();
        }
    });

    let consumer_thread = thread::spawn(move || {
        let mut last_val = 0;
        let mut received = 0;
        while last_val < iterations {
            if let Some(&val) = consumer.consume() {
                assert!(val >= last_val, "Monotonic ordering violation: {} < {}", val, last_val);
                last_val = val;
                received += 1;
            }
        }
        received
    });

    producer_thread.join().unwrap();
    let received = consumer_thread.join().unwrap();
    assert!(received > 0);
}
