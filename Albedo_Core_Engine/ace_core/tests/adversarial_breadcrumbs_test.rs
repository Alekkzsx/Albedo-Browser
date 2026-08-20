use ace_core::diagnostics::{BreadcrumbBuffer, BreadcrumbEntry};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

#[test]
fn test_adversarial_breadcrumbs_capacity_zero_stress() {
    let buffer: Arc<BreadcrumbBuffer<0>> = Arc::new(BreadcrumbBuffer::new());
    assert_eq!(buffer.capacity(), 0);
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());

    let mut handles = Vec::new();
    for thread_idx in 0..8 {
        let b = Arc::clone(&buffer);
        handles.push(thread::spawn(move || {
            for i in 0..5_000 {
                b.record("TestCat", format!("Thread {} Msg {}", thread_idx, i), i as u64);
                b.push(BreadcrumbEntry {
                    category: "PushCat",
                    message: format!("Payload {}", i),
                    timestamp_ms: i as u64,
                });
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
    assert_eq!(buffer.snapshot().len(), 0);
    buffer.with_slices(|(s1, s2)| {
        assert_eq!(s1.len(), 0);
        assert_eq!(s2.len(), 0);
    });
}

#[test]
fn test_adversarial_breadcrumbs_capacity_one_rapid_turnover() {
    let buffer = BreadcrumbBuffer::<1>::new();
    assert_eq!(buffer.capacity(), 1);

    for i in 0..10_000 {
        buffer.record("Cat", format!("Msg_{i}"), i as u64);
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());

        let snap = buffer.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].message, format!("Msg_{i}"));
        assert_eq!(snap[0].timestamp_ms, i as u64);

        buffer.with_slices(|(s1, s2)| {
            assert_eq!(s1.len() + s2.len(), 1);
            let item = if !s1.is_empty() { &s1[0] } else { &s2[0] };
            assert_eq!(item.message, format!("Msg_{i}"));
        });
    }

    buffer.clear();
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
}

#[test]
fn test_adversarial_breadcrumbs_multithreaded_high_contention() {
    const CAPACITY: usize = 64;
    const NUM_WRITERS: usize = 12;
    const WRITES_PER_THREAD: usize = 10_000;
    const NUM_READERS: usize = 4;

    let buffer = Arc::new(BreadcrumbBuffer::<CAPACITY>::new());
    let stop_flag = Arc::new(AtomicBool::new(false));
    let total_written = Arc::new(AtomicUsize::new(0));

    // Spawn readers
    let mut reader_handles = Vec::new();
    for _ in 0..NUM_READERS {
        let b = Arc::clone(&buffer);
        let stop = Arc::clone(&stop_flag);
        reader_handles.push(thread::spawn(move || {
            let mut read_iterations = 0usize;
            while !stop.load(Ordering::Relaxed) {
                let len = b.len();
                assert!(len <= CAPACITY, "Buffer len {} exceeded capacity {}", len, CAPACITY);

                let snap = b.snapshot();
                assert!(snap.len() <= CAPACITY);

                b.with_slices(|(s1, s2)| {
                    let total_slice_len = s1.len() + s2.len();
                    assert!(total_slice_len <= CAPACITY);
                    for item in s1.iter().chain(s2.iter()) {
                        assert_eq!(item.category, "ContentionCat");
                    }
                });

                read_iterations += 1;
                if read_iterations.is_multiple_of(100) {
                    thread::yield_now();
                }
            }
        }));
    }

    // Spawn writers
    let mut writer_handles = Vec::new();
    for thread_idx in 0..NUM_WRITERS {
        let b = Arc::clone(&buffer);
        let tw = Arc::clone(&total_written);
        writer_handles.push(thread::spawn(move || {
            for i in 0..WRITES_PER_THREAD {
                let ts = (thread_idx * 1_000_000 + i) as u64;
                b.record("ContentionCat", format!("W_{thread_idx}_I_{i}"), ts);
                tw.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Wait for all writers
    for handle in writer_handles {
        handle.join().unwrap();
    }

    // Signal readers to stop
    stop_flag.store(true, Ordering::Relaxed);
    for handle in reader_handles {
        handle.join().unwrap();
    }

    assert_eq!(total_written.load(Ordering::SeqCst), NUM_WRITERS * WRITES_PER_THREAD);
    assert_eq!(buffer.len(), CAPACITY);

    let snap = buffer.snapshot();
    assert_eq!(snap.len(), CAPACITY);

    buffer.with_slices(|(s1, s2)| {
        assert_eq!(s1.len() + s2.len(), CAPACITY);
    });
}

#[test]
fn test_ring_buffer_slices_continuity_under_continuous_overflow() {
    const CAPACITY: usize = 7;
    let buffer = BreadcrumbBuffer::<CAPACITY>::new();

    for i in 0..200 {
        buffer.record("Sequence", format!("Item_{i}"), i as u64);

        let expected_len = (i + 1).min(CAPACITY);
        assert_eq!(buffer.len(), expected_len);

        let snap = buffer.snapshot();
        assert_eq!(snap.len(), expected_len);

        // Verify order in snapshot
        let start_seq = (i + 1).saturating_sub(CAPACITY);
        for (idx, entry) in snap.iter().enumerate() {
            let expected_seq = start_seq + idx;
            assert_eq!(entry.message, format!("Item_{expected_seq}"));
            assert_eq!(entry.timestamp_ms, expected_seq as u64);
        }

        // Verify with_slices matches snapshot exactly
        buffer.with_slices(|(s1, s2)| {
            let mut combined = Vec::new();
            combined.extend_from_slice(s1);
            combined.extend_from_slice(s2);
            assert_eq!(combined, snap);
        });
    }
}

#[test]
fn test_large_payload_no_leak_or_corruption() {
    const CAPACITY: usize = 16;
    let buffer = BreadcrumbBuffer::<CAPACITY>::new();

    let large_text = "X".repeat(64 * 1024); // 64 KB per string

    for i in 0..1000 {
        buffer.record("Heavy", format!("{}_{}", large_text, i), i as u64);
    }

    assert_eq!(buffer.len(), CAPACITY);
    let snap = buffer.snapshot();
    assert_eq!(snap.len(), CAPACITY);
    for (idx, entry) in snap.iter().enumerate() {
        let expected_i = 1000 - CAPACITY + idx;
        assert_eq!(entry.timestamp_ms, expected_i as u64);
        assert!(entry.message.ends_with(&format!("_{expected_i}")));
    }
}

#[test]
fn test_global_breadcrumb_buffer_concurrency() {
    let mut handles = Vec::new();
    for thread_idx in 0..8 {
        handles.push(thread::spawn(move || {
            for i in 0..1_000 {
                BreadcrumbBuffer::<32>::add("GlobalCat", format!("T_{thread_idx}_{i}"), (thread_idx * 10_000 + i) as u64);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let snap = BreadcrumbBuffer::<32>::global().snapshot();
    assert_eq!(snap.len(), 32);
}

#[test]
fn test_concurrent_clear_during_active_writes() {
    const CAPACITY: usize = 32;
    let buffer = Arc::new(BreadcrumbBuffer::<CAPACITY>::new());
    let stop_flag = Arc::new(AtomicBool::new(false));

    let mut writer_handles = Vec::new();
    for t in 0..6 {
        let b = Arc::clone(&buffer);
        let stop = Arc::clone(&stop_flag);
        writer_handles.push(thread::spawn(move || {
            let mut i = 0u64;
            while !stop.load(Ordering::Relaxed) {
                b.record("ConcurrentClear", format!("T_{t}_M_{i}"), i);
                i += 1;
            }
        }));
    }

    // Flurry of clears
    for _ in 0..200 {
        buffer.clear();
        let len = buffer.len();
        assert!(len <= CAPACITY);
    }

    stop_flag.store(true, Ordering::Relaxed);
    for h in writer_handles {
        h.join().unwrap();
    }

    let final_len = buffer.len();
    assert!(final_len <= CAPACITY);
}

#[test]
fn test_multibyte_utf8_and_empty_strings() {
    let buffer = BreadcrumbBuffer::<5>::new();

    buffer.record("Unicode", "🦀 Rustacean 🚀 💖 \u{1F980}", 1);
    buffer.record("Empty", "", 2);
    buffer.record("ZeroByteCategory", "Message", 3);

    assert_eq!(buffer.len(), 3);
    let snap = buffer.snapshot();
    assert_eq!(snap[0].message, "🦀 Rustacean 🚀 💖 \u{1F980}");
    assert_eq!(snap[1].message, "");
}
