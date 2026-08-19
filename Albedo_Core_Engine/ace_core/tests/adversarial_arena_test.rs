use ace_core::arena::{Arena, ArenaId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Helper struct that tracks Drop invocations to detect leaks or double-frees.
struct DropTracker {
    #[allow(dead_code)]
    id: usize,
    counter: Arc<AtomicUsize>,
}

impl DropTracker {
    fn new(id: usize, counter: Arc<AtomicUsize>) -> Self {
        Self { id, counter }
    }
}

impl Drop for DropTracker {
    fn drop(&mut self) {
        self.counter.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_drop_tracking_zero_leaks_zero_double_frees() {
    let drop_counter = Arc::new(AtomicUsize::new(0));
    let mut arena = Arena::new();

    // 1. Allocate 1,000 items
    let mut ids = Vec::new();
    for i in 0..1000 {
        ids.push(arena.alloc(DropTracker::new(i, Arc::clone(&drop_counter))));
    }
    assert_eq!(drop_counter.load(Ordering::SeqCst), 0);
    assert_eq!(arena.len(), 1000);

    // 2. Remove 400 items
    for &id in &ids[0..400] {
        let val = arena.remove(id);
        assert!(val.is_some());
    }
    assert_eq!(drop_counter.load(Ordering::SeqCst), 400);
    assert_eq!(arena.len(), 600);

    // 3. Clear the arena (should drop remaining 600 items)
    arena.clear();
    assert_eq!(drop_counter.load(Ordering::SeqCst), 1000);
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());

    // 4. Allocate 500 items into the cleared arena
    for i in 0..500 {
        arena.alloc(DropTracker::new(i + 1000, Arc::clone(&drop_counter)));
    }
    assert_eq!(drop_counter.load(Ordering::SeqCst), 1000);
    assert_eq!(arena.len(), 500);

    // 5. Drop the entire arena
    drop(arena);
    assert_eq!(drop_counter.load(Ordering::SeqCst), 1500);
}

#[test]
fn test_generational_invalidation_aba_stress() {
    let mut arena = Arena::new();

    // Repeatedly allocate and remove on the same slot over 10,000 generations
    let mut old_ids = Vec::with_capacity(10_000);

    for gen in 1..=10_000 {
        let id = arena.alloc(format!("generation_{gen}"));
        let index = (id.raw() & 0xFFFF_FFFF) as u32;
        let version = (id.raw() >> 32) as u32;
        assert_eq!(index, 0, "Index must remain 0 for single-slot reuse");
        assert_eq!(version, gen as u32);
        assert_eq!(arena.get(id), Some(&format!("generation_{gen}")));

        // Verify that ALL previous generation IDs are completely invalid
        for (prev_gen, &prev_id) in old_ids.iter().enumerate() {
            assert_eq!(arena.get(prev_id), None, "Old gen {prev_gen} must not resolve");
            assert_eq!(arena.get_mut(prev_id), None);
            assert!(!arena.contains(prev_id));
        }

        old_ids.push(id);
        let removed = arena.remove(id);
        assert_eq!(removed, Some(format!("generation_{gen}")));
        assert_eq!(arena.get(id), None);
    }
}

#[test]
fn test_pseudo_random_alloc_free_permutation_oracle() {
    // Simple deterministic PRNG (Linear Congruential Generator)
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
            self.0
        }
        fn next_range(&mut self, max: usize) -> usize {
            if max == 0 { 0 } else { (self.next() % (max as u64)) as usize }
        }
    }

    let mut rng = Lcg(1337_2026_0819);
    let mut arena: Arena<u64> = Arena::new();
    let mut oracle: HashMap<ArenaId<u64>, u64> = HashMap::new();
    let mut active_ids: Vec<ArenaId<u64>> = Vec::new();

    for step in 0..50_000 {
        let op = rng.next_range(100);

        if op < 55 || active_ids.is_empty() {
            // 55% Alloc
            let val = rng.next();
            let id = arena.alloc(val);
            active_ids.push(id);
            oracle.insert(id, val);
        } else if op < 85 {
            // 30% Remove
            let idx = rng.next_range(active_ids.len());
            let id = active_ids.swap_remove(idx);
            let expected_val = oracle.remove(&id).unwrap();
            let actual_val = arena.remove(id);
            assert_eq!(actual_val, Some(expected_val));
        } else if op < 95 {
            // 10% Get / Mutate
            let idx = rng.next_range(active_ids.len());
            let id = active_ids[idx];
            let expected_val = *oracle.get(&id).unwrap();
            assert_eq!(arena.get(id), Some(&expected_val));

            // Mutate
            let new_val = rng.next();
            *arena.get_mut(id).unwrap() = new_val;
            oracle.insert(id, new_val);
        } else {
            // 5% Clear
            arena.clear();
            oracle.clear();
            active_ids.clear();
        }

        // Verify invariants periodically
        if step % 250 == 0 {
            assert_eq!(arena.len(), oracle.len());
            assert_eq!(arena.is_empty(), oracle.is_empty());

            // Check iter consistency
            let mut iter_count = 0;
            for (id, &val) in arena.iter() {
                assert_eq!(oracle.get(&id), Some(&val));
                iter_count += 1;
            }
            assert_eq!(iter_count, oracle.len());

            let stats = arena.stats();
            assert_eq!(stats.live, oracle.len());
        }
    }

    // Final purge check
    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());
}

#[test]
fn test_multi_round_clear_and_realloc_capacity() {
    let mut arena: Arena<usize> = Arena::new();
    const NUM_ITEMS: usize = 2000;
    const NUM_ROUNDS: usize = 20;

    let mut previous_generation_ids = Vec::new();

    for round in 0..NUM_ROUNDS {
        let mut current_ids = Vec::with_capacity(NUM_ITEMS);
        for i in 0..NUM_ITEMS {
            current_ids.push(arena.alloc(round * 100_000 + i));
        }

        assert_eq!(arena.len(), NUM_ITEMS);

        // Verify current round entries
        for (i, &id) in current_ids.iter().enumerate() {
            assert_eq!(arena.get(id), Some(&(round * 100_000 + i)));
        }

        // Verify all IDs from ALL previous rounds are dead
        for &prev_id in &previous_generation_ids {
            assert_eq!(arena.get(prev_id), None);
            assert_eq!(arena.get_mut(prev_id), None);
            assert!(!arena.contains(prev_id));
        }

        previous_generation_ids.extend(current_ids);

        // Clear arena
        arena.clear();
        assert_eq!(arena.len(), 0);
        assert!(arena.is_empty());
    }

    // After all rounds, capacity should not have grown unboundedly (should be exactly >= NUM_ITEMS)
    assert!(arena.capacity() >= NUM_ITEMS);
}

#[test]
fn test_forged_and_boundary_arena_ids() {
    let mut arena: Arena<u32> = Arena::new();
    let id1 = arena.alloc(111);
    let id2 = arena.alloc(222);

    // 1. Raw conversions
    let raw1 = id1.raw();
    let restored1 = ArenaId::<u32>::from_raw(raw1).unwrap();
    assert_eq!(arena.get(restored1), Some(&111));

    // 2. Forged ArenaIds
    let forged_zero = ArenaId::<u32>::from_raw(0);
    assert!(forged_zero.is_none());

    let forged_high_index = ArenaId::<u32>::from_raw((1u64 << 32) | 999_999u64).unwrap();
    assert_eq!(arena.get(forged_high_index), None);
    assert_eq!(arena.get_mut(forged_high_index), None);
    assert_eq!(arena.remove(forged_high_index), None);
    assert!(!arena.contains(forged_high_index));

    let index1 = id1.raw() & 0xFFFF_FFFF;
    let forged_wrong_version = ArenaId::<u32>::from_raw((999u64 << 32) | index1).unwrap();
    assert_eq!(arena.get(forged_wrong_version), None);
    assert_eq!(arena.get_mut(forged_wrong_version), None);
    assert_eq!(arena.remove(forged_wrong_version), None);
    assert!(!arena.contains(forged_wrong_version));

    let forged_max = ArenaId::<u32>::from_raw(u64::MAX).unwrap();
    assert_eq!(arena.get(forged_max), None);
    assert_eq!(arena.get_mut(forged_max), None);
    assert_eq!(arena.remove(forged_max), None);
    assert!(!arena.contains(forged_max));

    // Valid items still intact
    assert_eq!(arena.get(id1), Some(&111));
    assert_eq!(arena.get(id2), Some(&222));
}

#[test]
fn test_reserve_with_fragmented_free_list() {
    let mut arena = Arena::new();
    let mut ids = Vec::new();
    for i in 0..100 {
        ids.push(arena.alloc(i));
    }

    // Remove every alternate item to fragment the free list
    for i in (0..100).step_by(2) {
        arena.remove(ids[i]);
    }
    assert_eq!(arena.len(), 50);

    // Reserve large capacity triggering vector reallocation
    arena.reserve(10_000);
    assert!(arena.capacity() >= 10_050);

    // Verify existing live items survived reallocation
    for i in (1..100).step_by(2) {
        assert_eq!(arena.get(ids[i]), Some(&i));
    }

    // Allocate 50 items (should reuse the 50 free slots)
    let mut reused_ids = Vec::new();
    for i in 0..50 {
        reused_ids.push(arena.alloc(i + 500));
    }
    assert_eq!(arena.len(), 100);

    // Allocate 100 more items (should grow entries)
    let mut new_ids = Vec::new();
    for i in 0..100 {
        new_ids.push(arena.alloc(i + 1000));
    }
    assert_eq!(arena.len(), 200);

    // Verify all alive values
    for (i, &id) in reused_ids.iter().enumerate() {
        assert_eq!(arena.get(id), Some(&(i + 500)));
    }
    for (i, &id) in new_ids.iter().enumerate() {
        assert_eq!(arena.get(id), Some(&(i + 1000)));
    }
}

#[test]
fn test_arena_zst_zero_sized_types() {
    let mut arena: Arena<()> = Arena::new();
    let mut ids = Vec::new();

    for _ in 0..10_000 {
        ids.push(arena.alloc(()));
    }
    assert_eq!(arena.len(), 10_000);
    assert_eq!(arena.stats().bytes_allocated, 0);

    for &id in &ids[0..5000] {
        assert_eq!(arena.remove(id), Some(()));
    }
    assert_eq!(arena.len(), 5000);

    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());
}

#[test]
fn test_arena_large_types() {
    #[derive(Debug, PartialEq, Eq, Clone)]
    struct LargeBlob([u8; 4096]);

    let mut arena: Arena<LargeBlob> = Arena::new();
    let mut ids = Vec::new();

    for i in 0..100 {
        let mut blob = [0u8; 4096];
        blob[0] = i as u8;
        blob[4095] = (255 - i) as u8;
        ids.push(arena.alloc(LargeBlob(blob)));
    }

    assert_eq!(arena.len(), 100);
    assert_eq!(arena.stats().bytes_allocated, 100 * 4096);

    for (i, &id) in ids.iter().enumerate() {
        let val = arena.get(id).unwrap();
        assert_eq!(val.0[0], i as u8);
        assert_eq!(val.0[4095], (255 - i) as u8);
    }
}

#[test]
fn test_arena_values_and_values_mut_consistency() {
    let mut arena = Arena::new();
    let mut ids = Vec::new();

    for i in 0..20 {
        ids.push(arena.alloc(i));
    }

    // Remove some
    arena.remove(ids[3]);
    arena.remove(ids[7]);
    arena.remove(ids[15]);

    // Check values()
    let vals: Vec<i32> = arena.values().copied().collect();
    assert_eq!(vals.len(), 17);

    // Mutate via values_mut()
    for v in arena.values_mut() {
        *v *= 10;
    }

    // Verify through get()
    assert_eq!(arena.get(ids[0]), Some(&0));
    assert_eq!(arena.get(ids[1]), Some(&10));
    assert_eq!(arena.get(ids[2]), Some(&20));
    assert_eq!(arena.get(ids[3]), None);
    assert_eq!(arena.get(ids[4]), Some(&40));
}

#[test]
fn test_arena_clear_empty_and_consecutive_clears() {
    let mut arena: Arena<String> = Arena::new();

    // Clear brand new arena
    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());
    assert_eq!(arena.stats().total_freed, 0);

    // Alloc 1, remove 1 (now arena has 1 entry, 1 free_list item)
    let id = arena.alloc("hello".to_string());
    arena.remove(id);
    assert_eq!(arena.len(), 0);

    // Clear when all items were already removed
    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());

    // 10 consecutive clears
    for _ in 0..10 {
        arena.clear();
        assert_eq!(arena.len(), 0);
        assert!(arena.is_empty());
    }

    // Reallocate
    let id2 = arena.alloc("world".to_string());
    assert_eq!(arena.len(), 1);
    assert_eq!(arena.get(id2), Some(&"world".to_string()));
    assert_eq!(arena.get(id), None);
}
