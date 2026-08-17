use ace_core::collections::{AtomicBitSet, FixedBitSet};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;

#[test]
fn test_fixed_bitset_operations() {
    let mut bitset = FixedBitSet::<2>::new(); // 128 bits
    assert!(bitset.is_empty());
    assert_eq!(bitset.count_ones(), 0);

    bitset.set(0, true);
    bitset.set(63, true);
    bitset.set(64, true);
    bitset.set(127, true);

    assert!(bitset.get(0));
    assert!(bitset.get(63));
    assert!(bitset.get(64));
    assert!(bitset.get(127));
    assert!(!bitset.get(1));
    assert!(!bitset.get(65));
    assert_eq!(bitset.count_ones(), 4);

    bitset.set(63, false);
    assert!(!bitset.get(63));
    assert_eq!(bitset.count_ones(), 3);

    bitset.clear();
    assert!(bitset.is_empty());
}

#[test]
fn test_atomic_bitset_concurrent_access() {
    let bitset = Arc::new(AtomicBitSet::<4>::new()); // 256 bits

    let mut handles = Vec::new();
    for i in 0..8 {
        let bs = Arc::clone(&bitset);
        handles.push(thread::spawn(move || {
            for bit in (i * 30)..((i + 1) * 30) {
                bs.set(bit, true, Ordering::SeqCst);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    for bit in 0..240 {
        assert!(bitset.get(bit, Ordering::SeqCst), "Bit {} deve estar ativo", bit);
    }
    assert!(!bitset.get(245, Ordering::SeqCst));

    // Test-and-Set
    assert!(bitset.test_and_set(0, Ordering::SeqCst)); // Já estava true
    assert!(!bitset.test_and_set(250, Ordering::SeqCst)); // Estava false, agora virou true
    assert!(bitset.get(250, Ordering::SeqCst));
}
