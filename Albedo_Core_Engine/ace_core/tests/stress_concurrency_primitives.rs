use ace_core::ring::RingBuffer;
use ace_core::sync::SpinLock;
use std::sync::Arc;
use std::thread;

#[test]
fn test_stress_ring_buffer_10_million() {
    let ring = Arc::new(RingBuffer::<u64>::new(1024));
    let ring_c = Arc::clone(&ring);
    
    let total_messages = 10_000_000;
    
    let producer = thread::spawn(move || {
        for i in 0..total_messages {
            // Tenta inserir, se estiver cheio faz yield para o consumidor agir
            while ring.push(i).is_err() {
                thread::yield_now();
            }
        }
    });
    
    let consumer = thread::spawn(move || {
        let mut count = 0;
        let mut expected = 0;
        
        while count < total_messages {
            if let Some(val) = ring_c.pop() {
                assert_eq!(val, expected, "Data corruption detectado no RingBuffer!");
                expected += 1;
                count += 1;
            } else {
                thread::yield_now();
            }
        }
    });
    
    producer.join().unwrap();
    consumer.join().unwrap();
}

#[test]
fn test_stress_spinlock_extreme_contention() {
    let counter = Arc::new(SpinLock::new(0usize));
    let num_threads = 8;
    let increments = 1_000_000;
    
    let mut handles = vec![];
    
    for _ in 0..num_threads {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..increments {
                let mut guard = c.lock();
                *guard += 1;
            }
        }));
    }
    
    for h in handles {
        h.join().unwrap();
    }
    
    let final_val = *counter.lock();
    assert_eq!(final_val, num_threads * increments);
}
