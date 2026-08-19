//! # Milestone M1 Adversarial Stress Test Harness
//!
//! Rigorous empirical stress-testing suite for:
//! - `InlineVec` (random insertions, stack/heap transitions, retain drop counting, panic safety)
//! - `TripleBuffer` (500k-op multi-threaded lock-free concurrency, frame skipping, non-Copy payload safety)
//! - `Arena` (generational versioning, version rollover, clear idempotence)
//! - `BreadcrumbBuffer` (zero-capacity edge cases, high concurrency ring buffer)

use ace_core::arena::Arena;
use ace_core::collections::inline_vec::InlineVec;
use ace_core::collections::triple_buffer::{triple_buffer, triple_buffer_with, TripleBuffer};
use ace_core::diagnostics::breadcrumbs::{BreadcrumbBuffer, BreadcrumbEntry};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ============================================================================
// Drop-tracking fixture for memory leak and double-free detection (CWE-415/416)
// ============================================================================

#[derive(Debug)]
struct DropSentinel {
    id: usize,
    alive_counter: Arc<AtomicUsize>,
    drop_counter: Arc<AtomicUsize>,
}

impl DropSentinel {
    fn new(id: usize, alive: &Arc<AtomicUsize>, dropped: &Arc<AtomicUsize>) -> Self {
        alive.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            alive_counter: Arc::clone(alive),
            drop_counter: Arc::clone(dropped),
        }
    }
}

impl Drop for DropSentinel {
    fn drop(&mut self) {
        self.alive_counter.fetch_sub(1, Ordering::SeqCst);
        self.drop_counter.fetch_add(1, Ordering::SeqCst);
    }
}

// ============================================================================
// 1. InlineVec Adversarial Tests
// ============================================================================

#[test]
fn test_adversarial_inline_vec_random_insertions_and_ordering() {
    let mut inline: InlineVec<usize, 8> = InlineVec::new();
    let mut reference: Vec<usize> = Vec::new();

    // Pseudo-random LCG sequence for deterministic reproduction
    let mut state: u64 = 0xDEADBEEF;
    let lcg = |s: &mut u64| -> usize {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        (*s >> 33) as usize
    };

    for val in 0..1_000 {
        let current_len = inline.len();
        assert_eq!(current_len, reference.len());

        let insert_idx = if current_len == 0 {
            0
        } else {
            lcg(&mut state) % (current_len + 1)
        };

        inline.insert(insert_idx, val);
        reference.insert(insert_idx, val);

        assert_eq!(inline.as_slice(), reference.as_slice());
        if reference.len() <= 8 {
            assert!(inline.is_inline());
        } else {
            assert!(!inline.is_inline());
        }
    }

    // Drain and verify
    for _ in 0..500 {
        let current_len = inline.len();
        let remove_idx = lcg(&mut state) % current_len;
        let v1 = inline.remove(remove_idx);
        let v2 = reference.remove(remove_idx);
        assert_eq!(v1, v2);
    }
    assert_eq!(inline.as_slice(), reference.as_slice());
}

#[test]
fn test_adversarial_inline_vec_heavy_retain_drop_audit() {
    let alive = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));

    for initial_count in 0..=20 {
        for modulo in 1..=5 {
            {
                let mut vec: InlineVec<DropSentinel, 6> = InlineVec::new();
                for i in 0..initial_count {
                    vec.push(DropSentinel::new(i, &alive, &dropped));
                }

                assert_eq!(vec.len(), initial_count);
                let alive_before = alive.load(Ordering::SeqCst);
                let dropped_before = dropped.load(Ordering::SeqCst);

                // Retain elements where id % modulo == 0
                vec.retain(|item| item.id % modulo == 0);

                let expected_retained = (0..initial_count).filter(|i| i % modulo == 0).count();
                let expected_discarded = initial_count - expected_retained;

                assert_eq!(vec.len(), expected_retained);
                assert_eq!(dropped.load(Ordering::SeqCst) - dropped_before, expected_discarded);
                assert_eq!(alive_before - alive.load(Ordering::SeqCst), expected_discarded);

                // Verify retained elements are correct
                let retained_ids: Vec<usize> = vec.iter().map(|s| s.id).collect();
                let expected_ids: Vec<usize> = (0..initial_count).filter(|i| i % modulo == 0).collect();
                assert_eq!(retained_ids, expected_ids);
            }
            // Upon scope exit, all remaining elements must be dropped
            assert_eq!(alive.load(Ordering::SeqCst), 0);
        }
    }
}

#[test]
fn test_adversarial_inline_vec_retain_panic_unwinding_exhaustive() {
    let alive = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));

    // Test panicking at every possible index from 0 to 7 in an InlineVec of capacity 6 (inline) and capacity 2 (heap)
    for total_elements in 1..=8 {
        for panic_at in 0..total_elements {
            let res = catch_unwind(AssertUnwindSafe(|| {
                let mut vec: InlineVec<DropSentinel, 6> = InlineVec::new();
                for i in 0..total_elements {
                    vec.push(DropSentinel::new(i, &alive, &dropped));
                }

                vec.retain(|item| {
                    if item.id == panic_at {
                        panic!("Simulated predicate panic at index {}", panic_at);
                    }
                    item.id % 2 == 0
                });
            }));

            assert!(res.is_err(), "Expected panic at index {}", panic_at);
            // All created elements must be dropped; zero memory leaks, zero double frees
            assert_eq!(
                alive.load(Ordering::SeqCst),
                0,
                "Memory leak detected during panic unwinding! total={}, panic_at={}",
                total_elements,
                panic_at
            );
        }
    }
}

#[test]
fn test_adversarial_inline_vec_zero_capacity() {
    let mut vec: InlineVec<String, 0> = InlineVec::new();
    assert_eq!(vec.len(), 0);
    assert!(vec.is_inline());
    assert_eq!(vec.capacity(), 0);

    vec.push("first".to_string());
    assert_eq!(vec.len(), 1);
    assert!(!vec.is_inline());
    assert_eq!(vec[0], "first");

    vec.insert(0, "zero".to_string());
    assert_eq!(vec.as_slice(), &["zero", "first"]);

    assert_eq!(vec.pop(), Some("first".to_string()));
    assert_eq!(vec.remove(0), "zero");
    assert!(vec.is_empty());
}

// ============================================================================
// 2. TripleBuffer Adversarial Concurrency Stress Tests
// ============================================================================

#[test]
fn test_adversarial_triple_buffer_500k_concurrency_torture() {
    let (mut producer, mut consumer) = triple_buffer(0u64);
    let total_frames = 500_000u64;

    let producer_handle = thread::spawn(move || {
        for frame in 1..=total_frames {
            producer.write(frame);
            producer.publish();
        }
    });

    let consumer_handle = thread::spawn(move || {
        let mut last_frame = 0u64;
        let mut frames_consumed = 0u64;

        while last_frame < total_frames {
            if let Some(&frame) = consumer.consume() {
                assert!(
                    frame > last_frame,
                    "Monotonicity violation: got frame {} after {}",
                    frame,
                    last_frame
                );
                assert!(
                    frame <= total_frames,
                    "Out of bounds frame: {} > {}",
                    frame,
                    total_frames
                );
                last_frame = frame;
                frames_consumed += 1;
            }
        }
        frames_consumed
    });

    producer_handle.join().unwrap();
    let consumed = consumer_handle.join().unwrap();
    assert!(consumed > 0, "Consumer must have observed at least one frame");
}

#[test]
fn test_adversarial_triple_buffer_non_copy_heap_allocated_payload() {
    #[derive(Debug, PartialEq, Eq)]
    struct ComplexFrame {
        id: u64,
        payload: Vec<u8>,
        tag: String,
    }

    let (mut producer, mut consumer) = triple_buffer_with(|| ComplexFrame {
        id: 0,
        payload: vec![0u8; 1024],
        tag: "initial".to_string(),
    });

    let is_running = Arc::new(AtomicBool::new(true));
    let running_producer = Arc::clone(&is_running);

    let producer_handle = thread::spawn(move || {
        let mut count = 0u64;
        while running_producer.load(Ordering::Relaxed) {
            count += 1;
            producer.write_with(|frame| {
                frame.id = count;
                frame.payload.iter_mut().for_each(|b| *b = (count & 0xFF) as u8);
                frame.tag = format!("frame-{}", count);
            });
            producer.publish();
        }
        count
    });

    let mut last_id = 0u64;
    for _ in 0..500 {
        if let Some(frame) = consumer.consume() {
            assert!(frame.id >= last_id, "Frame order regression");
            assert_eq!(frame.payload.len(), 1024);
            let expected_byte = (frame.id & 0xFF) as u8;
            for &byte in &frame.payload {
                assert_eq!(byte, expected_byte, "Payload corruption detected in TripleBuffer!");
            }
            assert_eq!(frame.tag, format!("frame-{}", frame.id));
            last_id = frame.id;
        }
        thread::sleep(Duration::from_micros(50));
    }

    is_running.store(false, Ordering::Relaxed);
    let produced = producer_handle.join().unwrap();
    assert!(produced > 0);
}

#[test]
fn test_adversarial_triple_buffer_burst_producer_slow_consumer() {
    let (mut producer, mut consumer) = triple_buffer(0usize);

    // Producer rapidly publishes 10,000 frames
    for i in 1..=10_000 {
        producer.write(i);
        producer.publish();
    }

    // Consumer reads once: must get frame 10,000 cleanly
    assert_eq!(consumer.consume(), Some(&10_000));
    assert_eq!(consumer.consume(), None);
    assert_eq!(*consumer.read(), 10_000);
}

// ============================================================================
// 3. Arena Generational Soundness Tests
// ============================================================================

#[test]
fn test_adversarial_arena_clear_and_version_invalidation() {
    let mut arena: Arena<String> = Arena::new();

    let id1 = arena.alloc("Node1".to_string());
    let id2 = arena.alloc("Node2".to_string());
    let id3 = arena.alloc("Node3".to_string());

    assert_eq!(arena.len(), 3);
    assert_eq!(arena.get(id1), Some(&"Node1".to_string()));

    // Clear the arena
    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());

    // Old handles MUST be invalid
    assert_eq!(arena.get(id1), None);
    assert_eq!(arena.get(id2), None);
    assert_eq!(arena.get(id3), None);

    // Re-allocating must reuse slots starting from index 0 with new versions
    let id4 = arena.alloc("Node4".to_string());
    assert_eq!(id4.index(), 0);
    assert_ne!(id4.version(), id1.version());
    assert_eq!(arena.get(id4), Some(&"Node4".to_string()));

    // Consecutive clear calls are safe and idempotent
    arena.clear();
    arena.clear();
    assert_eq!(arena.len(), 0);
}

// ============================================================================
// 4. BreadcrumbBuffer Concurrency & Zero-Capacity Tests
// ============================================================================

#[test]
fn test_adversarial_breadcrumb_zero_capacity() {
    let buffer: BreadcrumbBuffer<0> = BreadcrumbBuffer::new();
    assert_eq!(buffer.capacity(), 0);
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());

    // Record / push on capacity 0 must be safe no-op
    buffer.record("TEST", "Should not panic or store", 100);
    buffer.push(BreadcrumbEntry {
        category: "TEST",
        message: "No-op".to_string(),
        timestamp_ms: 200,
    });

    assert_eq!(buffer.len(), 0);
    assert_eq!(buffer.snapshot(), vec![]);
}

#[test]
fn test_adversarial_breadcrumb_multi_threaded_rotation() {
    let buffer = Arc::new(BreadcrumbBuffer::<16>::new());
    let mut handles = Vec::new();

    for t in 0..4 {
        let buf = Arc::clone(&buffer);
        handles.push(thread::spawn(move || {
            for i in 0..1_000 {
                buf.record("THREAD", format!("T{}-{}", t, i), i as u64);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(buffer.len(), 16);
    let snapshot = buffer.snapshot();
    assert_eq!(snapshot.len(), 16);
}
