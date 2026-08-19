//! # E2E Tier 2: Boundary & Corner Cases Suite for `ace_core`
//!
//! Comprehensive boundary, extreme, overflow, and malformed input test coverage for all 19 features.
//! Naming convention: `tier2_featXX_<feature_name>_boundary_<scenario>`
//! Total test functions >= 95.

use ace_core::arena::{Arena, ArenaId};
use ace_core::collections::inline_vec::InlineVec;
use ace_core::collections::triple_buffer::triple_buffer;
use ace_core::diagnostics::BreadcrumbBuffer;
use ace_core::event_loop::{EventLoop, TaskSource};
use ace_core::flags::{is_node_dirty, style_hint_to_node_flags, NodeFlags, StyleChangeHint};
use ace_core::math::color::Color;
use ace_core::math::layout_unit::{LayoutUnit, MAX, MIN, ZERO};
use ace_core::math::oklab::Oklch;
use ace_core::net::{percent_decode, sniff_mime_type};
use ace_core::security::{compute_referrer, matches_domain_pattern, Origin, ReferrerPolicy, UnguessableToken};
use ace_core::time::MockClock;

use rustc_hash::FxHashSet;
use std::panic::catch_unwind;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ============================================================================
// Feature 1: InlineVec Memory Shift Fix (5+ tests)
// ============================================================================

#[test]
fn tier2_feat01_inline_vec_boundary_zero_capacity() {
    let mut vec: InlineVec<i32, 0> = InlineVec::new();
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
    assert_eq!(vec.capacity(), 0);

    vec.push(42);
    assert_eq!(vec.len(), 1);
    assert_eq!(vec[0], 42);
    assert!(!vec.is_inline());

    vec.push(100);
    assert_eq!(vec.len(), 2);
    assert_eq!(vec[1], 100);

    assert_eq!(vec.pop(), Some(100));
    assert_eq!(vec.pop(), Some(42));
    assert_eq!(vec.pop(), None);
}

#[test]
fn tier2_feat01_inline_vec_boundary_exact_capacity_insert_at_bounds() {
    let mut vec: InlineVec<u32, 4> = InlineVec::new();
    vec.insert(0, 10);
    vec.insert(1, 40);
    vec.insert(1, 20);
    vec.insert(2, 30);

    assert_eq!(vec.len(), 4);
    assert!(vec.is_inline());
    assert_eq!(vec.as_slice(), &[10, 20, 30, 40]);

    // Inserting when at capacity transitions to heap
    vec.insert(0, 5);
    assert_eq!(vec.len(), 5);
    assert!(!vec.is_inline());
    assert_eq!(vec.as_slice(), &[5, 10, 20, 30, 40]);

    // Insert at end of heap
    vec.insert(5, 50);
    assert_eq!(vec.as_slice(), &[5, 10, 20, 30, 40, 50]);
}

#[test]
fn tier2_feat01_inline_vec_boundary_out_of_bounds_panic() {
    let mut vec: InlineVec<i32, 2> = InlineVec::new();
    vec.push(1);

    // Insert past len must panic
    let res_insert = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut v = vec.clone();
        v.insert(5, 99);
    }));
    assert!(res_insert.is_err());

    // Remove past len must panic
    let res_remove = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut v = vec.clone();
        v.remove(3);
    }));
    assert!(res_remove.is_err());
}

#[test]
fn tier2_feat01_inline_vec_boundary_large_buffer_spill() {
    let mut vec: InlineVec<usize, 4> = InlineVec::new();
    for i in 0..1000 {
        vec.push(i * 2);
    }
    assert_eq!(vec.len(), 1000);
    assert!(!vec.is_inline());
    for i in 0..1000 {
        assert_eq!(vec[i], i * 2);
        assert_eq!(vec.get(i), Some(&(i * 2)));
    }
    assert_eq!(vec.get(1000), None);
}

#[test]
fn tier2_feat01_inline_vec_boundary_empty_drain_and_truncate() {
    let mut vec: InlineVec<String, 4> = InlineVec::new();
    let drained = vec.drain(0..0);
    assert!(drained.is_empty());
    assert_eq!(vec.len(), 0);

    vec.push("alpha".to_string());
    vec.push("beta".to_string());
    vec.truncate(0);
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
    assert_eq!(vec.pop(), None);
}

#[test]
fn tier2_feat01_inline_vec_boundary_single_element_remove_and_insert() {
    let mut vec: InlineVec<String, 1> = InlineVec::new();
    vec.insert(0, "first".to_string());
    assert_eq!(vec.len(), 1);
    assert_eq!(vec.remove(0), "first".to_string());
    assert_eq!(vec.len(), 0);

    vec.insert(0, "second".to_string());
    assert_eq!(vec.len(), 1);
    assert_eq!(vec.pop(), Some("second".to_string()));
    assert_eq!(vec.pop(), None);
}

// ============================================================================
// Feature 2: InlineVec Double Free Fix / Retain (5+ tests)
// ============================================================================

#[derive(Clone)]
struct DropTracker {
    id: usize,
    drop_count: Arc<AtomicUsize>,
}

impl Drop for DropTracker {
    fn drop(&mut self) {
        self.drop_count.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn tier2_feat02_inline_vec_retain_boundary_retain_all() {
    let mut vec: InlineVec<i32, 4> = InlineVec::new();
    vec.push(10);
    vec.push(20);
    vec.push(30);

    vec.retain(|_| true);
    assert_eq!(vec.len(), 3);
    assert_eq!(vec.as_slice(), &[10, 20, 30]);
}

#[test]
fn tier2_feat02_inline_vec_retain_boundary_retain_none() {
    let mut vec: InlineVec<i32, 4> = InlineVec::new();
    vec.push(1);
    vec.push(2);
    vec.push(3);
    vec.push(4);

    vec.retain(|_| false);
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());

    // Should still accept new items cleanly
    vec.push(42);
    assert_eq!(vec.len(), 1);
    assert_eq!(vec[0], 42);
}

#[test]
fn tier2_feat02_inline_vec_retain_boundary_drop_tracking() {
    let drop_counter = Arc::new(AtomicUsize::new(0));
    {
        let mut vec: InlineVec<DropTracker, 6> = InlineVec::new();
        for i in 0..6 {
            vec.push(DropTracker {
                id: i,
                drop_count: Arc::clone(&drop_counter),
            });
        }
        assert_eq!(drop_counter.load(Ordering::SeqCst), 0);

        // Retain only even IDs (0, 2, 4). IDs 1, 3, 5 must be dropped exactly once.
        vec.retain(|t| t.id % 2 == 0);
        assert_eq!(vec.len(), 3);
        assert_eq!(drop_counter.load(Ordering::SeqCst), 3);
        assert_eq!(vec[0].id, 0);
        assert_eq!(vec[1].id, 2);
        assert_eq!(vec[2].id, 4);
    }
    // Remaining 3 items dropped on vec drop
    assert_eq!(drop_counter.load(Ordering::SeqCst), 6);
}

#[test]
fn tier2_feat02_inline_vec_retain_boundary_zst() {
    let mut vec: InlineVec<(), 8> = InlineVec::new();
    for _ in 0..10 {
        vec.push(());
    }
    assert_eq!(vec.len(), 10);
    vec.retain(|_| true);
    assert_eq!(vec.len(), 10);
    vec.retain(|_| false);
    assert_eq!(vec.len(), 0);
}

#[test]
fn tier2_feat02_inline_vec_retain_boundary_alternating_heap() {
    let mut vec: InlineVec<usize, 2> = InlineVec::new();
    for i in 0..50 {
        vec.push(i);
    }
    assert!(!vec.is_inline());

    // Keep multiples of 5
    vec.retain(|&x| x % 5 == 0);
    assert_eq!(vec.len(), 10);
    assert_eq!(vec.as_slice(), &[0, 5, 10, 15, 20, 25, 30, 35, 40, 45]);
}

// ============================================================================
// Feature 3: TripleBuffer Lock-Free Zero-Copy (5+ tests)
// ============================================================================

#[test]
fn tier2_feat03_triple_buffer_boundary_rapid_producer_overwrites() {
    let (mut producer, mut consumer) = triple_buffer(0u64);

    for i in 1..=100 {
        producer.write(i);
        producer.publish();
    }

    // Consumer should see the most recently published frame
    let latest = consumer.consume_cloned();
    assert_eq!(latest, Some(100));

    // Next consume without new publish should be None
    assert_eq!(consumer.consume(), None);
}

#[test]
fn tier2_feat03_triple_buffer_boundary_empty_consumer_reads() {
    let (mut _producer, mut consumer) = triple_buffer(42u32);

    assert_eq!(consumer.consume(), None);
    assert_eq!(consumer.consume(), None);
    assert_eq!(*consumer.read_latest(), 42);
}

#[test]
fn tier2_feat03_triple_buffer_boundary_read_latest_stability() {
    let (mut producer, mut consumer) = triple_buffer(100u32);

    producer.write(200);
    producer.publish();

    assert_eq!(consumer.consume_cloned(), Some(200));
    assert_eq!(*consumer.read_latest(), 200);
    assert_eq!(*consumer.read_latest(), 200);
    assert_eq!(consumer.consume(), None);
}

#[test]
fn tier2_feat03_triple_buffer_boundary_large_payload() {
    #[derive(Clone, PartialEq, Debug)]
    struct BigPayload {
        data: Vec<u8>,
        checksum: u32,
    }

    let initial = BigPayload {
        data: vec![0u8; 10_000],
        checksum: 0,
    };
    let (mut producer, mut consumer) = triple_buffer(initial);

    let updated = BigPayload {
        data: vec![0xAA; 10_000],
        checksum: 0xDEADBEEF,
    };
    producer.write(updated.clone());
    producer.publish();

    let received = consumer.consume().unwrap();
    assert_eq!(*received, updated);
}

#[test]
fn tier2_feat03_triple_buffer_boundary_concurrent_heavy_contention() {
    let (mut producer, mut consumer) = triple_buffer(0u64);
    let running = Arc::new(AtomicBool::new(true));
    let r_clone = Arc::clone(&running);

    let prod_handle = thread::spawn(move || {
        let mut count = 0;
        while r_clone.load(Ordering::Relaxed) || count < 50 {
            count += 1;
            producer.write(count);
            producer.publish();
            thread::yield_now();
        }
        count
    });

    let mut last_seen = 0;
    for _ in 0..100 {
        if let Some(&val) = consumer.consume() {
            assert!(val >= last_seen, "Monotonic frame progress violation");
            last_seen = val;
        }
        thread::yield_now();
    }

    running.store(false, Ordering::Relaxed);
    let total_produced = prod_handle.join().unwrap();
    assert!(total_produced >= 50);
}

// ============================================================================
// Feature 4: Arena<T> Deterministic clear() (5+ tests)
// ============================================================================

#[test]
fn tier2_feat04_arena_boundary_clear_on_empty() {
    let mut arena: Arena<String> = Arena::new();
    assert_eq!(arena.len(), 0);
    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());

    let id = arena.alloc("hello".to_string());
    assert_eq!(arena.get(id), Some(&"hello".to_string()));
}

#[test]
fn tier2_feat04_arena_boundary_clear_with_10k_items() {
    let mut arena: Arena<usize> = Arena::with_capacity(10_000);
    let mut ids = Vec::with_capacity(10_000);
    for i in 0..10_000 {
        ids.push(arena.alloc(i));
    }
    assert_eq!(arena.len(), 10_000);

    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());

    for id in ids {
        assert_eq!(arena.get(id), None);
        assert!(!arena.contains(id));
    }
}

#[test]
fn tier2_feat04_arena_boundary_generational_index_reuse_after_clear() {
    let mut arena: Arena<u32> = Arena::new();
    let old_id0 = arena.alloc(100);
    let old_id1 = arena.alloc(200);

    arena.clear();

    let new_id0 = arena.alloc(300);
    let new_id1 = arena.alloc(400);

    // Old IDs must remain invalid even if indices match because version bumped
    assert_eq!(arena.get(old_id0), None);
    assert_eq!(arena.get(old_id1), None);
    assert_eq!(arena.get(new_id0), Some(&300));
    assert_eq!(arena.get(new_id1), Some(&400));
}

#[test]
fn tier2_feat04_arena_boundary_invalid_and_corrupted_keys() {
    let mut arena: Arena<i32> = Arena::new();
    let valid_id = arena.alloc(42);

    let oob_id = ArenaId::<i32>::from_raw((1u64 << 32) | 999_999).unwrap();
    let wrong_version_id = ArenaId::<i32>::from_raw((9999u64 << 32) | (valid_id.raw() as u32 as u64)).unwrap();

    assert_eq!(arena.get(oob_id), None);
    assert_eq!(arena.get(wrong_version_id), None);
    assert_eq!(arena.remove(oob_id), None);
}

#[test]
fn tier2_feat04_arena_boundary_repeated_alloc_remove_cycles() {
    let mut arena: Arena<usize> = Arena::new();
    for i in 0..1000 {
        let id = arena.alloc(i);
        assert_eq!(arena.len(), 1);
        assert_eq!(arena.remove(id), Some(i));
        assert_eq!(arena.len(), 0);
        assert_eq!(arena.get(id), None);
    }
    assert_eq!(arena.stats().slot_reuses, 999);
}

// ============================================================================
// Feature 5: BreadcrumbBuffer O(1) Deque & Zero Guard (5+ tests)
// ============================================================================

#[test]
fn tier2_feat05_breadcrumb_buffer_boundary_capacity_zero() {
    let buffer: BreadcrumbBuffer<0> = BreadcrumbBuffer::new();
    buffer.record("Nav", "Testing zero cap", 100);
    buffer.record("DOM", "Another event", 105);

    let snap = buffer.snapshot();
    assert_eq!(snap.len(), 0);
}

#[test]
fn tier2_feat05_breadcrumb_buffer_boundary_capacity_one() {
    let buffer: BreadcrumbBuffer<1> = BreadcrumbBuffer::new();
    buffer.record("Cat1", "Msg1", 10);
    assert_eq!(buffer.snapshot().len(), 1);
    assert_eq!(buffer.snapshot()[0].message, "Msg1");

    buffer.record("Cat2", "Msg2", 20);
    let snap = buffer.snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0].message, "Msg2");
    assert_eq!(snap[0].category, "Cat2");
}

#[test]
fn tier2_feat05_breadcrumb_buffer_boundary_exceed_capacity_100x() {
    let buffer: BreadcrumbBuffer<5> = BreadcrumbBuffer::new();
    for i in 0..500 {
        buffer.record("Load", format!("Event {}", i), i as u64);
    }

    let snap = buffer.snapshot();
    assert_eq!(snap.len(), 5);
    for (idx, entry) in snap.iter().enumerate() {
        let expected_i = 495 + idx;
        assert_eq!(entry.message, format!("Event {}", expected_i));
        assert_eq!(entry.timestamp_ms, expected_i as u64);
    }
}

#[test]
fn tier2_feat05_breadcrumb_buffer_boundary_empty_snapshot_and_clear() {
    let buffer: BreadcrumbBuffer<10> = BreadcrumbBuffer::new();
    assert!(buffer.snapshot().is_empty());
    buffer.clear();
    assert!(buffer.snapshot().is_empty());

    buffer.record("A", "B", 1);
    assert_eq!(buffer.snapshot().len(), 1);
    buffer.clear();
    assert!(buffer.snapshot().is_empty());
}

#[test]
fn tier2_feat05_breadcrumb_buffer_boundary_unicode_and_extreme_messages() {
    let buffer: BreadcrumbBuffer<4> = BreadcrumbBuffer::new();
    let big_msg = "🔥".repeat(1000);
    buffer.record("Unicode", &big_msg, 9999);

    let snap = buffer.snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0].message, big_msg);
    assert_eq!(snap[0].category, "Unicode");
}

// ============================================================================
// Feature 6: UnguessableToken CSPRNG (5+ tests)
// ============================================================================

#[test]
fn tier2_feat06_unguessable_token_boundary_non_zero_check() {
    for _ in 0..100 {
        let token = UnguessableToken::new();
        assert!(!token.is_empty(), "Token must never be zero");
        assert!(token.high() != 0 || token.low() != 0);
    }
}

#[test]
fn tier2_feat06_unguessable_token_boundary_entropy_distribution() {
    let mut set = FxHashSet::default();
    for _ in 0..5000 {
        let token = UnguessableToken::new();
        assert!(set.insert(token), "Collision detected in UnguessableToken");
    }
    assert_eq!(set.len(), 5000);
}

#[test]
fn tier2_feat06_unguessable_token_boundary_from_raw_extremes() {
    let zero_token = UnguessableToken::from_raw(0, 0);
    assert!(zero_token.is_empty());
    assert_eq!(zero_token.to_hex(), "00000000000000000000000000000000");

    let max_token = UnguessableToken::from_raw(u64::MAX, u64::MAX);
    assert!(!max_token.is_empty());
    assert_eq!(max_token.to_hex(), "ffffffffffffffffffffffffffffffff");

    let single_bit = UnguessableToken::from_raw(1, 0);
    assert!(!single_bit.is_empty());
    assert_eq!(single_bit.to_hex(), "00000000000000010000000000000000");
}

#[test]
fn tier2_feat06_unguessable_token_boundary_format_serialization() {
    let token = UnguessableToken::new();
    let hex = token.to_hex();
    assert_eq!(hex.len(), 32);
    assert_eq!(format!("{}", token), hex);
    assert_eq!(format!("{:?}", token), format!("UnguessableToken({})", hex));

    let restored = UnguessableToken::from_raw(token.high(), token.low());
    assert_eq!(token, restored);
}

#[test]
fn tier2_feat06_unguessable_token_boundary_ordering_and_equality() {
    let t1 = UnguessableToken::from_raw(10, 20);
    let t2 = UnguessableToken::from_raw(10, 20);
    let t3 = UnguessableToken::from_raw(10, 21);
    let t4 = UnguessableToken::from_raw(11, 0);

    assert_eq!(t1, t2);
    assert!(t1 < t3);
    assert!(t3 < t4);
}

// ============================================================================
// Feature 7: compute_referrer RFC 9110 / CWE-200 (5+ tests)
// ============================================================================

#[test]
fn tier2_feat07_compute_referrer_boundary_malformed_urls() {
    let origin = Origin::parse("https://example.com").unwrap();
    assert_eq!(
        compute_referrer(&origin, "not_a_url", "https://example.com/api", ReferrerPolicy::SameOrigin),
        None
    );
    assert_eq!(
        compute_referrer(&origin, "javascript:void(0)", "https://example.com", ReferrerPolicy::UnsafeUrl),
        None
    );
    assert_eq!(
        compute_referrer(&origin, "", "https://example.com", ReferrerPolicy::UnsafeUrl),
        None
    );
}

#[test]
fn tier2_feat07_compute_referrer_boundary_missing_or_non_http_schemes() {
    let origin = Origin::parse("file:///local/path").unwrap_or_else(|_| Origin::new_opaque());
    assert_eq!(
        compute_referrer(&origin, "file:///C:/Users/file.html", "https://example.com", ReferrerPolicy::UnsafeUrl),
        None
    );
    assert_eq!(
        compute_referrer(&origin, "data:text/html,test", "https://example.com", ReferrerPolicy::UnsafeUrl),
        None
    );
}

#[test]
fn tier2_feat07_compute_referrer_boundary_ipv6_origins() {
    let origin = Origin::parse("https://[::1]:8080").unwrap();
    let current_url = "https://[::1]:8080/dashboard#section";
    let target_same = "https://[::1]:8080/api/v1";
    let target_cross = "https://[::1]:9090/api/v1";

    let ref_same = compute_referrer(&origin, current_url, target_same, ReferrerPolicy::SameOrigin);
    assert_eq!(ref_same, Some("https://[::1]:8080/dashboard".to_string()));

    let ref_cross = compute_referrer(&origin, current_url, target_cross, ReferrerPolicy::SameOrigin);
    assert_eq!(ref_cross, None);
}

#[test]
fn tier2_feat07_compute_referrer_boundary_extreme_lengths_and_fragments() {
    let origin = Origin::parse("https://example.com").unwrap();
    let long_path = "a".repeat(4000);
    let current_url = format!("https://example.com/{}#frag1#frag2", long_path);
    let target_url = "https://example.com/landing";

    let result = compute_referrer(&origin, &current_url, target_url, ReferrerPolicy::SameOrigin);
    assert_eq!(result, Some(format!("https://example.com/{}", long_path)));
}

#[test]
fn tier2_feat07_compute_referrer_boundary_downgrade_matrix() {
    let origin = Origin::parse("https://secure.example.com").unwrap();
    let secure_url = "https://secure.example.com/checkout";
    let insecure_target = "http://insecure.example.com/tracker";

    assert_eq!(
        compute_referrer(&origin, secure_url, insecure_target, ReferrerPolicy::NoReferrer),
        None
    );
    assert_eq!(
        compute_referrer(&origin, secure_url, insecure_target, ReferrerPolicy::NoReferrerWhenDowngrade),
        None
    );
    assert_eq!(
        compute_referrer(&origin, secure_url, insecure_target, ReferrerPolicy::StrictOrigin),
        None
    );
    assert_eq!(
        compute_referrer(&origin, secure_url, insecure_target, ReferrerPolicy::StrictOriginWhenCrossOrigin),
        None
    );
    assert_eq!(
        compute_referrer(&origin, secure_url, insecure_target, ReferrerPolicy::SameOrigin),
        None
    );
    assert_eq!(
        compute_referrer(&origin, secure_url, insecure_target, ReferrerPolicy::Origin),
        Some("https://secure.example.com".to_string())
    );
}

// ============================================================================
// Feature 8: Origin Serialization & File Isolation (5+ tests)
// ============================================================================

#[test]
fn tier2_feat08_origin_boundary_opaque_uniqueness() {
    let op1 = Origin::new_opaque();
    let op2 = Origin::new_opaque();

    assert!(op1.is_opaque());
    assert!(op2.is_opaque());
    assert_ne!(op1, op2);
    assert!(!op1.same_origin(&op2));
    assert_eq!(op1.ascii_serialization(), "null");
    assert_eq!(op2.ascii_serialization(), "null");
}

#[test]
fn tier2_feat08_origin_boundary_data_and_about_blank() {
    let data_origin = Origin::parse("data:text/html,<h1>Hello</h1>").unwrap();
    let blank_origin = Origin::parse("about:blank").unwrap();

    assert!(data_origin.is_opaque());
    assert!(blank_origin.is_opaque());
    assert_ne!(data_origin, blank_origin);
    assert_eq!(data_origin.ascii_serialization(), "null");
    assert_eq!(blank_origin.ascii_serialization(), "null");
}

#[test]
fn tier2_feat08_origin_boundary_port_extremes() {
    let max_port = Origin::parse("https://example.com:65535/").unwrap();
    assert_eq!(max_port.ascii_serialization(), "https://example.com:65535");

    let default_https = Origin::parse("https://example.com:443/").unwrap();
    assert_eq!(default_https.ascii_serialization(), "https://example.com");

    let non_default_http = Origin::parse("http://example.com:8080/").unwrap();
    assert_eq!(non_default_http.ascii_serialization(), "http://example.com:8080");

    let default_http = Origin::parse("http://example.com:80/").unwrap();
    assert_eq!(default_http.ascii_serialization(), "http://example.com");
}

#[test]
fn tier2_feat08_origin_boundary_file_scheme() {
    let file_origin = Origin::parse("file:///C:/Users/app/index.html").unwrap();
    let ascii = file_origin.ascii_serialization();
    assert!(!ascii.contains(":0"), "Port :0 must not appear in file origin serialization");
    assert!(ascii.starts_with("file://"));
}

#[test]
fn tier2_feat08_origin_boundary_localhost_and_loopbacks() {
    let local1 = Origin::parse("http://localhost:3000/").unwrap();
    let local2 = Origin::parse("http://127.0.0.1:8080/").unwrap();
    let local3 = Origin::parse("http://[::1]:9000/").unwrap();

    assert!(local1.is_secure());
    assert!(local2.is_secure());
    assert!(local3.is_secure());
}

// ============================================================================
// Feature 9: matches_domain_pattern CSP3 §6.7.2 (5+ tests)
// ============================================================================

#[test]
fn tier2_feat09_matches_domain_pattern_boundary_wildcard_star() {
    assert!(matches_domain_pattern("*", "example.com"));
    assert!(matches_domain_pattern("*", "sub.deep.example.com"));
    assert!(matches_domain_pattern("*", "localhost"));
    assert!(matches_domain_pattern("*", "127.0.0.1"));
}

#[test]
fn tier2_feat09_matches_domain_pattern_boundary_subdomain_matching() {
    assert!(matches_domain_pattern("*.example.com", "sub.example.com"));
    assert!(matches_domain_pattern("*.example.com", "deep.nested.sub.example.com"));
    assert!(!matches_domain_pattern("*.example.com", "notexample.com"));
    assert!(!matches_domain_pattern("*.example.com", "example.org"));
}

#[test]
fn tier2_feat09_matches_domain_pattern_boundary_apex_domain_behavior() {
    // Exact domain matching
    assert!(matches_domain_pattern("example.com", "example.com"));
    assert!(!matches_domain_pattern("example.com", "sub.example.com"));
}

#[test]
fn tier2_feat09_matches_domain_pattern_boundary_case_insensitivity_and_spaces() {
    assert!(matches_domain_pattern(" *.EXAMPLE.COM ", "Sub.Example.Com"));
    assert!(matches_domain_pattern("EXAMPLE.COM", "example.com"));
    assert!(matches_domain_pattern("  *  ", "foo.bar"));
}

#[test]
fn tier2_feat09_matches_domain_pattern_boundary_internationalized_domains() {
    assert!(matches_domain_pattern("*.xn--e1afmkfd.xn--p1ai", "sub.xn--e1afmkfd.xn--p1ai"));
    assert!(!matches_domain_pattern("*.xn--e1afmkfd.xn--p1ai", "other.xn--p1ai"));
}

// ============================================================================
// Feature 10: percent_decode Byte Preservation (5+ tests)
// ============================================================================

#[test]
fn tier2_feat10_percent_decode_boundary_percent_at_end() {
    assert_eq!(percent_decode("100%"), "100%");
    assert_eq!(percent_decode("progress: 50% done"), "progress: 50% done");
    assert_eq!(percent_decode("end%"), "end%");
}

#[test]
fn tier2_feat10_percent_decode_boundary_invalid_hex_sequences() {
    assert_eq!(percent_decode("%G1"), "%G1");
    assert_eq!(percent_decode("%1Z"), "%1Z");
    assert_eq!(percent_decode("%ZZ"), "%ZZ");
    assert_eq!(percent_decode("100%_concluido"), "100%_concluido");
}

#[test]
fn tier2_feat10_percent_decode_boundary_consecutive_percents() {
    assert_eq!(percent_decode("%%%"), "%%%");
    assert_eq!(percent_decode("%%%%"), "%%%%");
    assert_eq!(percent_decode("%25%25"), "%%");
}

#[test]
fn tier2_feat10_percent_decode_boundary_empty_and_no_escape() {
    assert_eq!(percent_decode(""), "");
    assert_eq!(percent_decode("normal plain text 123"), "normal plain text 123");
}

#[test]
fn tier2_feat10_percent_decode_boundary_multibyte_utf8() {
    assert_eq!(percent_decode("%C3%A9"), "é");
    assert_eq!(percent_decode("%F0%9F%94%A5"), "🔥");
    assert_eq!(
        percent_decode("Status: 100%_completo %E2%9C%93"),
        "Status: 100%_completo ✓"
    );
}

// ============================================================================
// Feature 11: sniff_mime_type UTF-8 Boundary (5+ tests)
// ============================================================================

#[test]
fn tier2_feat11_sniff_mime_type_boundary_empty_buffer() {
    assert_eq!(sniff_mime_type(&[]), "text/plain");
}

#[test]
fn tier2_feat11_sniff_mime_type_boundary_all_null_bytes() {
    let null_buf = [0u8; 512];
    let result = sniff_mime_type(&null_buf);
    assert!(result == "text/plain" || result == "application/octet-stream");
}

#[test]
fn tier2_feat11_sniff_mime_type_boundary_exact_512_bytes_html() {
    let mut buf = vec![b' '; 512];
    let prefix = b"<!DOCTYPE html><html><body></body></html>";
    buf[..prefix.len()].copy_from_slice(prefix);

    assert_eq!(sniff_mime_type(&buf), "text/html");
}

#[test]
fn tier2_feat11_sniff_mime_type_boundary_multi_byte_utf8_split() {
    // 511 bytes of ASCII followed by 0xC3 (start of 2-byte UTF-8 char)
    let mut buf = vec![b'A'; 511];
    buf.push(0xC3);

    // Should not panic when inspecting 512-byte window
    let result = sniff_mime_type(&buf);
    assert!(result == "text/plain" || result == "application/octet-stream");
}

#[test]
fn tier2_feat11_sniff_mime_type_boundary_short_magic_prefixes() {
    assert_eq!(sniff_mime_type(b"BM"), "image/bmp");
    assert_eq!(sniff_mime_type(b"GIF89a"), "image/gif");
    assert_eq!(sniff_mime_type(b"%PDF-"), "application/pdf");
    assert_eq!(sniff_mime_type(b"wOFF"), "font/woff");
    assert_eq!(sniff_mime_type(b"OTTO"), "font/otf");
}

// ============================================================================
// Feature 12: Event Loop Starvation Prevention (5+ tests)
// ============================================================================

#[test]
fn tier2_feat12_event_loop_starvation_boundary_ui_stream_with_network_task() {
    let el = EventLoop::new();
    let q = el.handle();
    let network_executed = Arc::new(AtomicBool::new(false));
    let ui_counter = Arc::new(AtomicUsize::new(0));

    let net_clone = Arc::clone(&network_executed);
    q.queue_network(move || {
        net_clone.store(true, Ordering::SeqCst);
    });

    for _ in 0..50 {
        let u = Arc::clone(&ui_counter);
        q.queue_user_interaction(move || {
            u.fetch_add(1, Ordering::SeqCst);
        });
    }

    // Step 51 times to drain all UI tasks and the network task
    for _ in 0..51 {
        el.step();
    }

    assert_eq!(ui_counter.load(Ordering::SeqCst), 50);
    assert!(network_executed.load(Ordering::SeqCst));
}

#[test]
fn tier2_feat12_event_loop_starvation_boundary_empty_event_loop_step() {
    let el = EventLoop::new();
    assert!(!el.step());
    assert_eq!(el.drain_microtasks(), 0);
    assert_eq!(el.process_animation_frame(), 0);
}

#[test]
fn tier2_feat12_event_loop_starvation_boundary_all_task_sources_interleaved() {
    let el = EventLoop::new();
    let q = el.handle();
    let order = Arc::new(parking_lot::Mutex::new(Vec::new()));

    for source in TaskSource::ALL_SOURCES {
        let o = Arc::clone(&order);
        q.queue_task(source, move || {
            o.lock().push(source);
        });
    }

    for _ in 0..TaskSource::ALL_SOURCES.len() {
        assert!(el.step());
    }
    assert!(!el.step());

    let recorded = order.lock().clone();
    assert_eq!(recorded.len(), 7);
    assert_eq!(recorded[0], TaskSource::UserInteraction);
    // All sources executed
    for src in TaskSource::ALL_SOURCES {
        assert!(recorded.contains(&src));
    }
}

#[test]
fn tier2_feat12_event_loop_starvation_boundary_stop_flag() {
    let el = EventLoop::new();
    el.stop();
    // step() should still be deterministic
    assert!(!el.step());
}

#[test]
fn tier2_feat12_event_loop_starvation_boundary_idle_tasks_zero_budget() {
    let el = EventLoop::new();
    let q = el.handle();
    let idle_ran = Arc::new(AtomicBool::new(false));
    let i_clone = Arc::clone(&idle_ran);

    q.post_idle_task(None, move |_deadline| {
        i_clone.store(true, Ordering::SeqCst);
    });

    let executed = el.process_idle_tasks(Duration::from_millis(0));
    assert_eq!(executed, 0);
    assert!(!idle_ran.load(Ordering::SeqCst));
}

// ============================================================================
// Feature 13: Microtask Checkpoint Reentrancy Guard (5+ tests)
// ============================================================================

#[test]
fn tier2_feat13_microtask_checkpoint_boundary_nested_depth_100() {
    let el = EventLoop::new();
    let q = el.handle();
    let counter = Arc::new(AtomicUsize::new(0));

    fn schedule_recurse(q: ace_core::event_loop::TaskQueue, counter: Arc<AtomicUsize>, depth: usize) {
        if depth > 0 {
            let q_clone = q.clone();
            let c_clone = Arc::clone(&counter);
            q.queue_microtask(move || {
                c_clone.fetch_add(1, Ordering::SeqCst);
                schedule_recurse(q_clone, c_clone, depth - 1);
            });
        }
    }

    schedule_recurse(q.clone(), Arc::clone(&counter), 150);
    let drained = el.drain_microtasks();
    assert_eq!(drained, 150);
    assert_eq!(counter.load(Ordering::SeqCst), 150);
}

#[test]
fn tier2_feat13_microtask_checkpoint_boundary_microtask_enqueues_macrotask() {
    let el = EventLoop::new();
    let q = el.handle();
    let macro_ran = Arc::new(AtomicBool::new(false));
    let micro_ran = Arc::new(AtomicBool::new(false));

    let m_clone = Arc::clone(&macro_ran);
    let q_clone = q.clone();
    let u_clone = Arc::clone(&micro_ran);

    q.queue_microtask(move || {
        u_clone.store(true, Ordering::SeqCst);
        q_clone.queue_dom(move || {
            m_clone.store(true, Ordering::SeqCst);
        });
    });

    // Step 1: microtask executes and schedules DOM macrotask
    assert!(el.step());
    assert!(micro_ran.load(Ordering::SeqCst));
    assert!(!macro_ran.load(Ordering::SeqCst)); // Macrotask runs on subsequent step

    // Step 2: DOM macrotask executes
    assert!(el.step());
    assert!(macro_ran.load(Ordering::SeqCst));
}

#[test]
fn tier2_feat13_microtask_checkpoint_boundary_fifo_order_execution() {
    let el = EventLoop::new();
    let q = el.handle();
    let log = Arc::new(parking_lot::Mutex::new(Vec::new()));

    for i in 0..100 {
        let l = Arc::clone(&log);
        q.queue_microtask(move || {
            l.lock().push(i);
        });
    }

    assert_eq!(el.drain_microtasks(), 100);
    let expected: Vec<usize> = (0..100).collect();
    assert_eq!(*log.lock(), expected);
}

#[test]
fn tier2_feat13_microtask_checkpoint_boundary_empty_drain() {
    let el = EventLoop::new();
    assert_eq!(el.drain_microtasks(), 0);
}

#[test]
fn tier2_feat13_microtask_checkpoint_boundary_interleaved_with_raf() {
    let el = EventLoop::new();
    let q = el.handle();
    let counter = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&counter);
    let q_clone = q.clone();
    q.request_animation_frame(move || {
        c1.store(1, Ordering::SeqCst);
        let c2 = Arc::clone(&c1);
        q_clone.queue_microtask(move || {
            c2.store(2, Ordering::SeqCst);
        });
    });

    let raf_count = el.process_animation_frame();
    assert_eq!(raf_count, 1);
    assert_eq!(counter.load(Ordering::SeqCst), 2); // RAF drains microtasks after callbacks
}

// ============================================================================
// Feature 14: Atomic Timer Macrotask Dispatch (5+ tests)
// ============================================================================

#[test]
fn tier2_feat14_atomic_timer_boundary_zero_delay_timer() {
    let clock = Arc::new(MockClock::new(100));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f_clone = Arc::clone(&fired);
    q.schedule_timer(Duration::from_millis(0), move || {
        f_clone.store(true, Ordering::SeqCst);
    });

    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn tier2_feat14_atomic_timer_boundary_immediate_cancellation() {
    let clock = Arc::new(MockClock::new(100));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f_clone = Arc::clone(&fired);
    let (_id, cancel_handle) = q.schedule_timer(Duration::from_millis(50), move || {
        f_clone.store(true, Ordering::SeqCst);
    });

    cancel_handle.store(true, Ordering::SeqCst);
    clock.advance_millis(100);

    assert!(el.step());
    assert!(!fired.load(Ordering::SeqCst), "Cancelled timer must not execute");
}

#[test]
fn tier2_feat14_atomic_timer_boundary_concurrent_expiration_same_ms() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let counter = Arc::new(AtomicUsize::new(0));

    for _ in 0..20 {
        let c = Arc::clone(&counter);
        q.schedule_timer(Duration::from_millis(50), move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    clock.advance_millis(50);
    for _ in 0..20 {
        assert!(el.step());
    }
    assert_eq!(counter.load(Ordering::SeqCst), 20);
}

#[test]
fn tier2_feat14_atomic_timer_boundary_clock_far_future_jump() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    q.schedule_timer(Duration::from_millis(100), move || {
        f.store(true, Ordering::SeqCst);
    });

    clock.advance_millis(1_000_000);
    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn tier2_feat14_atomic_timer_boundary_cancelled_in_callback() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let t2_ran = Arc::new(AtomicBool::new(false));

    let t2_clone = Arc::clone(&t2_ran);
    let (_t2_id, t2_cancel) = q.schedule_timer(Duration::from_millis(20), move || {
        t2_clone.store(true, Ordering::SeqCst);
    });

    q.schedule_timer(Duration::from_millis(10), move || {
        t2_cancel.store(true, Ordering::SeqCst);
    });

    clock.advance_millis(15);
    assert!(el.step()); // t1 runs and cancels t2

    clock.advance_millis(15);
    assert!(el.step()); // t2 is popped as expired macrotask but cancelled
    assert!(!t2_ran.load(Ordering::SeqCst));
}

// ============================================================================
// Feature 15: Dynamic Timer Nesting Clamping (5+ tests)
// ============================================================================

#[test]
fn tier2_feat15_dynamic_timer_nesting_boundary_depth_1_unclamped() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    q.schedule_timer_with_nesting(Duration::from_millis(0), 1, move || {
        f.store(true, Ordering::SeqCst);
    });

    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn tier2_feat15_dynamic_timer_nesting_boundary_depth_4_unclamped() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    q.schedule_timer_with_nesting(Duration::from_millis(1), 4, move || {
        f.store(true, Ordering::SeqCst);
    });

    clock.advance_millis(1);
    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn tier2_feat15_dynamic_timer_nesting_boundary_depth_5_clamped_4ms() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    q.schedule_timer_with_nesting(Duration::from_millis(0), 5, move || {
        f.store(true, Ordering::SeqCst);
    });

    clock.advance_millis(3);
    assert!(!el.step());
    assert!(!fired.load(Ordering::SeqCst));

    clock.advance_millis(1);
    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn tier2_feat15_dynamic_timer_nesting_boundary_depth_5_above_clamp() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    q.schedule_timer_with_nesting(Duration::from_millis(10), 5, move || {
        f.store(true, Ordering::SeqCst);
    });

    clock.advance_millis(9);
    assert!(!el.step());
    assert!(!fired.load(Ordering::SeqCst));

    clock.advance_millis(1);
    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn tier2_feat15_dynamic_timer_nesting_boundary_depth_50_clamped_4ms() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    q.schedule_timer_with_nesting(Duration::from_millis(2), 50, move || {
        f.store(true, Ordering::SeqCst);
    });

    clock.advance_millis(3);
    assert!(!el.step());
    assert!(!fired.load(Ordering::SeqCst));

    clock.advance_millis(1);
    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

// ============================================================================
// Feature 16: Bradford Chromatic Adaptation / Color Conversions (5+ tests)
// ============================================================================

#[test]
fn tier2_feat16_bradford_color_boundary_pure_black_roundtrip() {
    let black = Color::BLACK;
    let oklab = black.to_oklab();
    assert_eq!(oklab.l, 0.0);

    let roundtrip = Color::from_oklab(oklab);
    assert_eq!(roundtrip, Color::BLACK);
}

#[test]
fn tier2_feat16_bradford_color_boundary_pure_white_roundtrip() {
    let white = Color::WHITE;
    let oklab = white.to_oklab();
    assert!((oklab.l - 1.0).abs() < 1e-3);

    let roundtrip = Color::from_oklab(oklab);
    assert_eq!(roundtrip, Color::WHITE);
}

#[test]
fn tier2_feat16_bradford_color_boundary_out_of_gamut_clamping() {
    let color = Color::from_rgba_f32(-1.0, 2.5, 0.5, 1.2);
    assert_eq!(color.r, 0);
    assert_eq!(color.g, 255);
    assert_eq!(color.b, 128);
    assert_eq!(color.a, 255);
}

#[test]
fn tier2_feat16_bradford_color_boundary_display_p3_extremes() {
    let p3_green = Color::parse("color(display-p3 0 1 0)").unwrap();
    assert_eq!(p3_green.g, 255);
    assert_eq!(p3_green.a, 255);

    let p3_black = Color::parse("color(display-p3 0 0 0)").unwrap();
    assert_eq!(p3_black, Color::BLACK);

    let p3_white = Color::parse("color(display-p3 1 1 1)").unwrap();
    assert_eq!(p3_white, Color::WHITE);
}

#[test]
fn tier2_feat16_bradford_color_boundary_premultiplied_alpha_extremes() {
    let transparent = Color::TRANSPARENT;
    let (r, g, b, a) = transparent.to_premultiplied_f32();
    assert_eq!((r, g, b, a), (0.0, 0.0, 0.0, 0.0));

    let opaque_red = Color::RED;
    let (r, g, b, a) = opaque_red.to_premultiplied_f32();
    assert_eq!((r, g, b, a), (1.0, 0.0, 0.0, 1.0));

    let semi_white = Color::from_rgba(255, 255, 255, 128);
    let (r, _g, _b, a) = semi_white.to_premultiplied_f32();
    let expected_alpha = 128.0 / 255.0;
    assert!((a - expected_alpha).abs() < 1e-4);
    assert!((r - expected_alpha).abs() < 1e-4);
}

// ============================================================================
// Feature 17: CSS Color 4 Polar Interpolation (5+ tests)
// ============================================================================

#[test]
fn tier2_feat17_css_color_polar_boundary_hue_wrap_around_0_360() {
    let c1 = Color::from_oklch(Oklch::new(0.5, 0.2, 350.0, 1.0));
    let c2 = Color::from_oklch(Oklch::new(0.5, 0.2, 10.0, 1.0));

    let mid = c1.lerp_oklch(c2, 0.5);
    let mid_oklch = mid.to_oklch();

    // Shortest angular arc between 350° and 10° crosses 0°/360°, midpoint is 0° (or 360°)
    let h_diff = (mid_oklch.h - 0.0).abs().min((mid_oklch.h - 360.0).abs());
    assert!(h_diff < 5.0, "Expected hue ~0deg, got {:.2}deg", mid_oklch.h);
}

#[test]
fn tier2_feat17_css_color_polar_boundary_powerless_chroma_grey() {
    let grey = Color::from_oklch(Oklch::new(0.5, 0.0, 0.0, 1.0));
    let saturated = Color::from_oklch(Oklch::new(0.5, 0.2, 180.0, 1.0));

    let mid = grey.lerp_oklch(saturated, 0.5);
    let mid_oklch = mid.to_oklch();
    assert!((mid_oklch.c - 0.1).abs() < 0.05);
}

#[test]
fn tier2_feat17_css_color_polar_boundary_alpha_extremes_interpolation() {
    let transparent = Color::from_rgba(255, 0, 0, 0);
    let opaque = Color::from_rgba(255, 0, 0, 255);

    let mid = transparent.lerp_oklch(opaque, 0.5);
    assert_eq!(mid.a, 128);
}

#[test]
fn tier2_feat17_css_color_polar_boundary_t_factor_extremes() {
    let red = Color::RED;
    let blue = Color::BLUE;

    assert_eq!(red.lerp_oklch(blue, 0.0), red);
    assert_eq!(red.lerp_oklch(blue, 1.0), blue);
    assert_eq!(red.lerp_oklch(blue, -0.5), red);
    assert_eq!(red.lerp_oklch(blue, 1.5), blue);
}

#[test]
fn tier2_feat17_css_color_polar_boundary_color_mix_weights_zero_sum() {
    let mixed = Color::parse("color-mix(in srgb, red 0%, blue 100%)").unwrap();
    assert_eq!(mixed, Color::BLUE);

    let mixed_zero = Color::parse("color-mix(in srgb, red 0%, blue 0%)").unwrap();
    assert_eq!(mixed_zero, Color::TRANSPARENT);
}

// ============================================================================
// Feature 18: LayoutUnit Box Snapping (5+ tests)
// ============================================================================

#[test]
fn tier2_feat18_layout_unit_snapping_boundary_zero_size() {
    let zero = ZERO;
    assert_eq!(zero.raw(), 0);
    assert_eq!(zero.to_f32_px(), 0.0);
    assert_eq!(zero.floor_px(), 0);
    assert_eq!(zero.ceil_px(), 0);
    assert_eq!(zero.round_px(), 0);
    assert_eq!(zero.abs(), zero);
}

#[test]
fn tier2_feat18_layout_unit_snapping_boundary_negative_coordinates() {
    let neg = LayoutUnit::from_f32_px(-10.5);
    assert_eq!(neg.raw(), -630);
    assert_eq!(neg.floor_px(), -11);
    assert_eq!(neg.ceil_px(), -10);
    assert_eq!(neg.round_px(), -11);
    assert_eq!(neg.abs().to_f32_px(), 10.5);
}

#[test]
fn tier2_feat18_layout_unit_snapping_boundary_subpixel_fractions() {
    // 1 raw unit = 1/60 px
    let one_unit = LayoutUnit::from_raw(1);
    assert_eq!(one_unit.raw(), 1);
    assert!((one_unit.to_f32_px() - 1.0 / 60.0).abs() < 1e-5);

    // 1/3 px = 20 raw units
    let third_px = LayoutUnit::from_raw(20);
    assert_eq!(third_px * 3, LayoutUnit::from_px(1));

    // 1/2 px = 30 raw units
    let half_px = LayoutUnit::from_raw(30);
    assert_eq!(half_px.round_px(), 1);
    assert_eq!(half_px.floor_px(), 0);
    assert_eq!(half_px.ceil_px(), 1);
}

#[test]
fn tier2_feat18_layout_unit_snapping_boundary_adjacent_boxes_no_gap() {
    let origin = LayoutUnit::from_px(0);
    let box1_size = LayoutUnit::from_raw(2000); // 33.3333px
    let box2_size = LayoutUnit::from_raw(4000); // 66.6667px

    let box1_end = origin + box1_size;
    let box2_start = box1_end;
    let box2_end = box2_start + box2_size;

    assert_eq!(box1_end, box2_start, "Adjacent boxes must share identical boundary");
    assert_eq!(box2_end, LayoutUnit::from_px(100));
    assert_eq!(box2_end.round_px(), 100);
}

#[test]
fn tier2_feat18_layout_unit_snapping_boundary_overflow_saturation() {
    let max = MAX;
    assert_eq!(max + LayoutUnit::from_px(1), max);
    assert_eq!(max.saturating_add(LayoutUnit::from_px(1)), max);

    let min = MIN;
    assert_eq!(min - LayoutUnit::from_px(1), min);
    assert_eq!(min.saturating_sub(LayoutUnit::from_px(1)), min);

    // Division by zero in mul_div
    assert_eq!(LayoutUnit::from_px(10).mul_div(1, 0), MAX);
    assert_eq!(LayoutUnit::from_px(-10).mul_div(1, 0), MIN);
}

// ============================================================================
// Feature 19: style_hint_to_node_flags Mapping (5+ tests)
// ============================================================================

#[test]
fn tier2_feat19_style_hint_flags_boundary_empty_none_hint() {
    let flags = style_hint_to_node_flags(StyleChangeHint::NONE);
    assert_eq!(flags, NodeFlags::empty());
    assert!(!is_node_dirty(flags));
}

#[test]
fn tier2_feat19_style_hint_flags_boundary_all_hints_combined() {
    let all_hints = StyleChangeHint::all();
    let flags = style_hint_to_node_flags(all_hints);

    assert!(flags.contains(NodeFlags::DIRTY_STYLE));
    assert!(flags.contains(NodeFlags::DIRTY_LAYOUT));
    assert!(flags.contains(NodeFlags::DIRTY_PAINT));
    assert!(flags.contains(NodeFlags::SUBTREE_DIRTY));
    assert!(is_node_dirty(flags));
}

#[test]
fn tier2_feat19_style_hint_flags_boundary_subtree_recalc_mapping() {
    let flags = style_hint_to_node_flags(StyleChangeHint::SUBTREE_RECALC);
    assert!(flags.contains(NodeFlags::SUBTREE_DIRTY));
    assert!(flags.contains(NodeFlags::DIRTY_STYLE));
    assert!(!flags.contains(NodeFlags::DIRTY_LAYOUT));
    assert!(is_node_dirty(flags));
}

#[test]
fn tier2_feat19_style_hint_flags_boundary_repaint_vs_reflow() {
    let repaint_flags = style_hint_to_node_flags(StyleChangeHint::REPAINT);
    assert_eq!(repaint_flags, NodeFlags::DIRTY_PAINT);
    assert!(is_node_dirty(repaint_flags));

    let reflow_flags = style_hint_to_node_flags(StyleChangeHint::REFLOW_LAYOUT);
    assert!(reflow_flags.contains(NodeFlags::DIRTY_LAYOUT));
    assert!(reflow_flags.contains(NodeFlags::DIRTY_PAINT));
    assert!(!reflow_flags.contains(NodeFlags::DIRTY_STYLE));
    assert!(is_node_dirty(reflow_flags));
}

#[test]
fn tier2_feat19_style_hint_flags_boundary_is_node_dirty_exhaustive() {
    assert!(!is_node_dirty(NodeFlags::IS_ELEMENT | NodeFlags::IS_CONNECTED));
    assert!(is_node_dirty(NodeFlags::DIRTY_STYLE));
    assert!(is_node_dirty(NodeFlags::DIRTY_LAYOUT));
    assert!(is_node_dirty(NodeFlags::DIRTY_PAINT));
    assert!(is_node_dirty(NodeFlags::SUBTREE_DIRTY));
}
