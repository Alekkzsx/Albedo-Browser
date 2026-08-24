//! # Tier 1 E2E Test Suite - Feature Coverage (All 19 Features)
//!
//! Comprehensive isolated functional tests for all 19 foundational features of `ace_core`
//! conforming strictly to W3C, WHATWG, RFC 6454/9110, and PROJECT.md specifications.
//!
//! Naming convention: `tier1_featXX_<feature_name>_<scenario>`
//! Total test cases: 100+ (5+ tests per feature).

use ace_core::arena::Arena;
use ace_core::collections::{triple_buffer, InlineVec};
use ace_core::diagnostics::BreadcrumbBuffer;
use ace_core::event_loop::{EventLoop, TaskSource};
use ace_core::flags::{is_node_dirty, style_hint_to_node_flags, NodeFlags, StyleChangeHint};
use ace_core::math::color::{ColorSpace, HueInterpolation};
use ace_core::math::layout_unit::UNITS_PER_PIXEL;
use ace_core::math::{almost_equal, snap_to_pixel, Color, LayoutUnit, Oklch};
use ace_core::net::{percent_decode, sniff_mime_type};
use ace_core::security::{
    compute_referrer, matches_domain_pattern, Origin, ReferrerPolicy, UnguessableToken,
};
use ace_core::time::MockClock;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Feature 01: `InlineVec` Memory Shift Fix
// ============================================================================

#[test]
fn tier1_feat01_inline_vec_shift_insert_at_start_shifts_elements_correctly() {
    let mut vec: InlineVec<String, 4> = InlineVec::new();
    vec.push("second".to_string());
    vec.push("third".to_string());

    vec.insert(0, "first".to_string());

    assert_eq!(vec.len(), 3);
    assert!(vec.is_inline());
    assert_eq!(vec[0], "first");
    assert_eq!(vec[1], "second");
    assert_eq!(vec[2], "third");
}

#[test]
fn tier1_feat01_inline_vec_shift_insert_in_middle_preserves_order() {
    let mut vec: InlineVec<i32, 5> = InlineVec::new();
    vec.push(10);
    vec.push(30);
    vec.push(40);

    vec.insert(1, 20);

    assert_eq!(vec.len(), 4);
    assert!(vec.is_inline());
    assert_eq!(vec.as_slice(), &[10, 20, 30, 40]);
}

#[test]
fn tier1_feat01_inline_vec_shift_insert_at_end_equivalent_to_push() {
    let mut vec: InlineVec<&'static str, 4> = InlineVec::new();
    vec.push("alpha");
    vec.push("beta");

    vec.insert(2, "gamma");

    assert_eq!(vec.len(), 3);
    assert_eq!(vec[2], "gamma");
    assert_eq!(vec.as_slice(), &["alpha", "beta", "gamma"]);
}

#[test]
fn tier1_feat01_inline_vec_shift_multiple_sequential_inserts_inline() {
    let mut vec: InlineVec<u32, 6> = InlineVec::new();
    vec.insert(0, 100); // [100]
    vec.insert(0, 50); // [50, 100]
    vec.insert(1, 75); // [50, 75, 100]
    vec.insert(3, 125); // [50, 75, 100, 125]
    vec.insert(2, 85); // [50, 75, 85, 100, 125]

    assert_eq!(vec.len(), 5);
    assert!(vec.is_inline());
    assert_eq!(vec.as_slice(), &[50, 75, 85, 100, 125]);
}

#[test]
fn tier1_feat01_inline_vec_shift_insert_triggers_heap_spill_seamlessly() {
    let mut vec: InlineVec<usize, 3> = InlineVec::new();
    vec.push(1);
    vec.push(2);
    vec.push(3);
    assert!(vec.is_inline());

    // Inserção no meio excedendo capacidade inline N=3 -> migra para Heap
    vec.insert(1, 99);

    assert_eq!(vec.len(), 4);
    assert!(!vec.is_inline());
    assert_eq!(vec.as_slice(), &[1, 99, 2, 3]);
}

#[test]
fn tier1_feat01_inline_vec_shift_insert_non_copy_drop_tracking() {
    #[derive(Clone, Debug)]
    struct DropCounter(Arc<AtomicUsize>);
    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    let drop_count = Arc::new(AtomicUsize::new(0));
    {
        let mut vec: InlineVec<DropCounter, 4> = InlineVec::new();
        vec.push(DropCounter(Arc::clone(&drop_count)));
        vec.push(DropCounter(Arc::clone(&drop_count)));
        vec.insert(1, DropCounter(Arc::clone(&drop_count)));
        assert_eq!(vec.len(), 3);
        assert_eq!(drop_count.load(Ordering::SeqCst), 0);
    }
    // Todos os 3 itens descartados exatamente uma vez
    assert_eq!(drop_count.load(Ordering::SeqCst), 3);
}

// ============================================================================
// Feature 02: `InlineVec` Double Free Fix & Retain
// ============================================================================

#[test]
fn tier1_feat02_inline_vec_retain_all_elements_kept() {
    let mut vec: InlineVec<i32, 4> = InlineVec::new();
    vec.extend([1, 2, 3, 4]);

    vec.retain(|_| true);

    assert_eq!(vec.len(), 4);
    assert_eq!(vec.as_slice(), &[1, 2, 3, 4]);
    assert!(vec.is_inline());
}

#[test]
fn tier1_feat02_inline_vec_retain_drops_matching_elements_inline() {
    let mut vec: InlineVec<i32, 6> = InlineVec::new();
    vec.extend([1, 2, 3, 4, 5, 6]);

    // Retém apenas pares
    vec.retain(|&x| x % 2 == 0);

    assert_eq!(vec.len(), 3);
    assert_eq!(vec.as_slice(), &[2, 4, 6]);
    assert!(vec.is_inline());
}

#[test]
fn tier1_feat02_inline_vec_retain_drops_all_elements() {
    let mut vec: InlineVec<String, 4> = InlineVec::new();
    vec.push("a".to_string());
    vec.push("b".to_string());
    vec.push("c".to_string());

    vec.retain(|_| false);

    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
    assert!(vec.is_inline());
}

#[test]
fn tier1_feat02_inline_vec_retain_on_heap_spilled_vec() {
    let mut vec: InlineVec<i32, 2> = InlineVec::new();
    vec.extend([10, 15, 20, 25, 30]); // Spilled to heap
    assert!(!vec.is_inline());

    vec.retain(|&x| x >= 20);

    assert_eq!(vec.len(), 3);
    assert_eq!(vec.as_slice(), &[20, 25, 30]);
}

#[test]
fn tier1_feat02_inline_vec_retain_drop_tracking_exact_destruction() {
    #[derive(Clone, Debug)]
    struct Tracked(i32, Arc<AtomicUsize>);
    impl Drop for Tracked {
        fn drop(&mut self) {
            self.1.fetch_add(1, Ordering::SeqCst);
        }
    }

    let dropped = Arc::new(AtomicUsize::new(0));
    {
        let mut vec: InlineVec<Tracked, 5> = InlineVec::new();
        for i in 1..=5 {
            vec.push(Tracked(i, Arc::clone(&dropped)));
        }

        // Drop items 2 and 4
        vec.retain(|t| t.0 % 2 != 0);
        assert_eq!(vec.len(), 3);
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
    }
    // Remaining 3 items dropped on scope exit -> total 5 dropped exactly once
    assert_eq!(dropped.load(Ordering::SeqCst), 5);
}

// ============================================================================
// Feature 03: `TripleBuffer` Lock-Free Zero-Copy
// ============================================================================

#[test]
fn tier1_feat03_triple_buffer_initial_state_read_latest() {
    let (_producer, consumer) = triple_buffer(42);
    assert_eq!(*consumer.read_latest(), 42);
}

#[test]
fn tier1_feat03_triple_buffer_single_write_publish_and_consume() {
    let (mut producer, mut consumer) = triple_buffer("frame_0".to_string());
    assert_eq!(consumer.consume(), None);

    producer.write("frame_1".to_string());
    producer.publish();

    let frame = consumer.consume();
    assert_eq!(frame, Some(&"frame_1".to_string()));
    assert_eq!(*consumer.read_latest(), "frame_1");
    // Segundo consume sem nova publicação retorna None
    assert_eq!(consumer.consume(), None);
}

#[test]
fn tier1_feat03_triple_buffer_multiple_publishes_latest_frame_only() {
    let (mut producer, mut consumer) = triple_buffer(0);

    producer.write(1);
    producer.publish();
    producer.write(2);
    producer.publish();
    producer.write(3);
    producer.publish();

    // O consumidor lê o frame mais recente publicado (Zero-Copy)
    let latest = consumer.consume();
    assert_eq!(latest, Some(&3));
    assert_eq!(consumer.consume(), None);
}

#[test]
fn tier1_feat03_triple_buffer_write_with_in_place_mutation() {
    #[derive(Clone, Debug, PartialEq)]
    struct RenderPayload {
        draw_calls: usize,
        fps: f32,
    }

    let (mut producer, mut consumer) = triple_buffer(RenderPayload {
        draw_calls: 0,
        fps: 60.0,
    });

    producer.write_with(|payload| {
        payload.draw_calls = 150;
        payload.fps = 120.0;
    });
    producer.publish();

    let consumed = consumer.consume().unwrap();
    assert_eq!(consumed.draw_calls, 150);
    assert_eq!(consumed.fps, 120.0);
}

#[test]
fn tier1_feat03_triple_buffer_threaded_producer_consumer_exchange() {
    let (mut producer, mut consumer) = triple_buffer(0usize);

    let handle = std::thread::spawn(move || {
        for i in 1..=100 {
            producer.write(i);
            producer.publish();
            std::thread::yield_now();
        }
    });

    let mut last_seen = 0;
    while last_seen < 100 {
        if let Some(&frame) = consumer.consume() {
            assert!(frame >= last_seen, "Frame backwards movement detected");
            last_seen = frame;
        }
        std::thread::yield_now();
    }

    handle.join().unwrap();
    assert_eq!(last_seen, 100);
}

// ============================================================================
// Feature 04: `Arena<T>` Deterministic clear()
// ============================================================================

#[test]
fn tier1_feat04_arena_clear_invalidates_all_previous_ids() {
    let mut arena: Arena<String> = Arena::new();
    let id1 = arena.alloc("item_1".to_string());
    let id2 = arena.alloc("item_2".to_string());
    let id3 = arena.alloc("item_3".to_string());

    assert_eq!(arena.len(), 3);
    arena.clear();

    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());
    assert_eq!(arena.get(id1), None);
    assert_eq!(arena.get(id2), None);
    assert_eq!(arena.get(id3), None);
}

#[test]
fn tier1_feat04_arena_clear_realloc_uses_new_versions() {
    let mut arena: Arena<u32> = Arena::new();
    let old_id = arena.alloc(100);
    arena.clear();

    let new_id = arena.alloc(200);
    assert_ne!(old_id, new_id);
    assert_eq!(arena.get(old_id), None);
    assert_eq!(arena.get(new_id), Some(&200));
}

#[test]
fn tier1_feat04_arena_clear_on_empty_arena_is_noop() {
    let mut arena: Arena<i32> = Arena::new();
    arena.clear();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());

    let id = arena.alloc(42);
    assert_eq!(arena.get(id), Some(&42));
}

#[test]
fn tier1_feat04_arena_clear_stats_consistency() {
    let mut arena: Arena<u64> = Arena::new();
    let _a = arena.alloc(10);
    let _b = arena.alloc(20);
    arena.clear();

    let stats = arena.stats();
    assert_eq!(stats.live, 0);
    assert_eq!(stats.bytes_allocated, 0);
    assert_eq!(stats.total_allocated, 2);
}

#[test]
fn tier1_feat04_arena_clear_iter_and_values_empty() {
    let mut arena: Arena<&'static str> = Arena::new();
    arena.alloc("node1");
    arena.alloc("node2");
    arena.clear();

    let count_iter = arena.iter().count();
    let count_values = arena.values().count();
    assert_eq!(count_iter, 0);
    assert_eq!(count_values, 0);
}

// ============================================================================
// Feature 05: `BreadcrumbBuffer` O(1) Deque & Zero Guard
// ============================================================================

#[test]
fn tier1_feat05_breadcrumb_buffer_record_and_snapshot_ordering() {
    let buffer: BreadcrumbBuffer<5> = BreadcrumbBuffer::new();
    buffer.record("NET", "Request started", 1000);
    buffer.record("DOM", "Parser attached", 1010);
    buffer.record("STYLE", "CSS Recalc", 1020);

    let snap = buffer.snapshot();
    assert_eq!(snap.len(), 3);
    assert_eq!(snap[0].category, "NET");
    assert_eq!(snap[0].timestamp_ms, 1000);
    assert_eq!(snap[2].category, "STYLE");
    assert_eq!(snap[2].timestamp_ms, 1020);
}

#[test]
fn tier1_feat05_breadcrumb_buffer_rotation_evicts_oldest() {
    let buffer: BreadcrumbBuffer<3> = BreadcrumbBuffer::new();
    buffer.record("1", "First", 1);
    buffer.record("2", "Second", 2);
    buffer.record("3", "Third", 3);
    buffer.record("4", "Fourth", 4); // Expulsa "1" via pop_front()

    let snap = buffer.snapshot();
    assert_eq!(snap.len(), 3);
    assert_eq!(snap[0].category, "2");
    assert_eq!(snap[1].category, "3");
    assert_eq!(snap[2].category, "4");
}

#[test]
fn tier1_feat05_breadcrumb_buffer_clear_resets_snapshot() {
    let buffer: BreadcrumbBuffer<4> = BreadcrumbBuffer::new();
    buffer.record("A", "Alpha", 10);
    buffer.record("B", "Beta", 20);
    assert_eq!(buffer.snapshot().len(), 2);

    buffer.clear();
    assert_eq!(buffer.snapshot().len(), 0);
}

#[test]
fn tier1_feat05_breadcrumb_buffer_zero_capacity_guard_safe_noop() {
    let zero_buffer: BreadcrumbBuffer<0> = BreadcrumbBuffer::new();
    zero_buffer.record("TEST", "Should not panic or store", 100);
    assert_eq!(zero_buffer.len(), 0);
    assert!(zero_buffer.is_empty());
    assert_eq!(zero_buffer.snapshot().len(), 0);
}

#[test]
fn tier1_feat05_breadcrumb_buffer_global_singleton_usage() {
    let initial_len = BreadcrumbBuffer::<32>::global().snapshot().len();
    BreadcrumbBuffer::<32>::add("GLOBAL_TEST", "Testing global buffer", 9999);

    let snap = BreadcrumbBuffer::<32>::global().snapshot();
    assert!(snap.len() >= initial_len);
    assert!(snap
        .iter()
        .any(|e| e.category == "GLOBAL_TEST" && e.timestamp_ms == 9999));
}

// ============================================================================
// Feature 06: `UnguessableToken` CSPRNG
// ============================================================================

#[test]
fn tier1_feat06_unguessable_token_generation_non_empty() {
    let token = UnguessableToken::new();
    assert!(!token.is_empty());
    assert!(token.high() != 0 || token.low() != 0);
}

#[test]
fn tier1_feat06_unguessable_token_uniqueness_across_batch() {
    let mut set = HashSet::new();
    for _ in 0..1_000 {
        let token = UnguessableToken::new();
        assert!(
            set.insert(token),
            "Colisão de token gerada: duplicate UnguessableToken detected"
        );
    }
    assert_eq!(set.len(), 1_000);
}

#[test]
fn tier1_feat06_unguessable_token_hex_serialization_32_chars() {
    let token = UnguessableToken::new();
    let hex = token.to_hex();
    assert_eq!(hex.len(), 32);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(format!("{}", token), hex);
}

#[test]
fn tier1_feat06_unguessable_token_raw_reconstruction_roundtrip() {
    let token = UnguessableToken::new();
    let high = token.high();
    let low = token.low();

    let reconstructed = UnguessableToken::from_raw(high, low);
    assert_eq!(token, reconstructed);
    assert_eq!(token.to_hex(), reconstructed.to_hex());
}

#[test]
fn tier1_feat06_unguessable_token_empty_token_behavior() {
    let zero_token = UnguessableToken::from_raw(0, 0);
    assert!(zero_token.is_empty());
    assert_eq!(
        zero_token.to_hex(),
        "00000000000000000000000000000000"
    );

    let default_token = UnguessableToken::default();
    assert!(default_token.is_empty());
}

// ============================================================================
// Feature 07: `compute_referrer` RFC 9110 / CWE-200
// ============================================================================

#[test]
fn tier1_feat07_compute_referrer_same_origin_policy_success_and_cross_block() {
    let origin = Origin::parse("https://example.com/section/index.html").unwrap();
    let current_url = "https://example.com/section/index.html";
    let same_target = "https://example.com/api/v1/resource";
    let cross_target = "https://other-domain.com/api";

    let ref_same = compute_referrer(
        &origin,
        current_url,
        same_target,
        ReferrerPolicy::SameOrigin,
    );
    assert_eq!(
        ref_same,
        Some("https://example.com/section/index.html".to_string())
    );

    let ref_cross = compute_referrer(
        &origin,
        current_url,
        cross_target,
        ReferrerPolicy::SameOrigin,
    );
    assert_eq!(ref_cross, None);
}

#[test]
fn tier1_feat07_compute_referrer_downgrade_https_to_http_blocked() {
    let origin = Origin::parse("https://secure.bank.com").unwrap();
    let current_url = "https://secure.bank.com/transfer";
    let insecure_target = "http://insecure-analytics.com/log";

    let ref_strict = compute_referrer(
        &origin,
        current_url,
        insecure_target,
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(ref_strict, None);

    let ref_downgrade = compute_referrer(
        &origin,
        current_url,
        insecure_target,
        ReferrerPolicy::NoReferrerWhenDowngrade,
    );
    assert_eq!(ref_downgrade, None);
}

#[test]
fn tier1_feat07_compute_referrer_strips_credentials_userinfo() {
    let origin = Origin::parse("https://secret.org").unwrap();
    let current_url = "https://admin:password123@secret.org/dashboard";
    let same_target = "https://secret.org/api/status";

    let referrer = compute_referrer(
        &origin,
        current_url,
        same_target,
        ReferrerPolicy::SameOrigin,
    );
    // RFC 9110: userinfo DEVE ser expurgado
    assert_eq!(referrer, Some("https://secret.org/dashboard".to_string()));
}

#[test]
fn tier1_feat07_compute_referrer_origin_only_policy_strips_path_and_query() {
    let origin = Origin::parse("https://albedo.org").unwrap();
    let current_url = "https://albedo.org/secret/report?token=abc123xyz";
    let cross_target = "https://partner.com/webhook";

    let ref_origin = compute_referrer(
        &origin,
        current_url,
        cross_target,
        ReferrerPolicy::Origin,
    );
    assert_eq!(ref_origin, Some("https://albedo.org".to_string()));
}

#[test]
fn tier1_feat07_compute_referrer_strips_fragment_anchor() {
    let origin = Origin::parse("https://docs.rs").unwrap();
    let current_url = "https://docs.rs/ace_core/0.1.0/index.html#trait-impls";
    let same_target = "https://docs.rs/ace_core/0.1.0/struct.Color.html";

    let referrer = compute_referrer(
        &origin,
        current_url,
        same_target,
        ReferrerPolicy::SameOrigin,
    );
    assert_eq!(
        referrer,
        Some("https://docs.rs/ace_core/0.1.0/index.html".to_string())
    );
}

#[test]
fn tier1_feat07_compute_referrer_non_http_schemes_return_none() {
    let origin = Origin::parse("about:blank").unwrap();
    let current_url = "data:text/html,<h1>Hello</h1>";
    let target = "https://example.com";

    let ref_data = compute_referrer(&origin, current_url, target, ReferrerPolicy::UnsafeUrl);
    assert_eq!(ref_data, None);
}

// ============================================================================
// Feature 08: `Origin` Serialization & File Isolation
// ============================================================================

#[test]
fn tier1_feat08_origin_standard_http_https_default_port_serialization() {
    let http_orig = Origin::parse("http://example.org:80/path").unwrap();
    assert_eq!(http_orig.ascii_serialization(), "http://example.org");

    let https_orig = Origin::parse("https://example.org:443/secure").unwrap();
    assert_eq!(https_orig.ascii_serialization(), "https://example.org");
}

#[test]
fn tier1_feat08_origin_custom_port_included_in_serialization() {
    let custom_http = Origin::parse("http://localhost:8080/dev").unwrap();
    assert_eq!(custom_http.ascii_serialization(), "http://localhost:8080");

    let custom_https = Origin::parse("https://internal.test:9443/").unwrap();
    assert_eq!(
        custom_https.ascii_serialization(),
        "https://internal.test:9443"
    );
}

#[test]
fn tier1_feat08_origin_opaque_isolation_and_null_serialization() {
    let op1 = Origin::new_opaque();
    let op2 = Origin::new_opaque();

    assert!(op1.is_opaque());
    assert!(op2.is_opaque());
    assert_eq!(op1.ascii_serialization(), "null");
    assert_eq!(op2.ascii_serialization(), "null");
    // Origens opacas NUNCA compartilham mesma origem
    assert!(!op1.same_origin(&op2));
}

#[test]
fn tier1_feat08_origin_data_and_about_blank_are_opaque() {
    let data_orig = Origin::parse("data:text/html,<b>Test</b>").unwrap();
    assert!(data_orig.is_opaque());
    assert_eq!(data_orig.ascii_serialization(), "null");

    let blank_orig = Origin::parse("about:blank").unwrap();
    assert!(blank_orig.is_opaque());
}

#[test]
fn tier1_feat08_origin_localhost_and_loopback_security() {
    let local = Origin::parse("http://localhost:4000/").unwrap();
    assert!(local.is_secure());

    let ipv4_loopback = Origin::parse("http://127.0.0.1:3000/").unwrap();
    assert!(ipv4_loopback.is_secure());

    let remote_http = Origin::parse("http://example.com/").unwrap();
    assert!(!remote_http.is_secure());
}

#[test]
fn tier1_feat08_origin_file_scheme_port_zero_omitted_serialization() {
    let file_origin = Origin::parse("file:///C:/Users/test/index.html").unwrap();
    // RFC 6454: omit :0 on file schemes
    assert_eq!(file_origin.ascii_serialization(), "file://");
}

// ============================================================================
// Feature 09: `matches_domain_pattern` CSP3 §6.7.2
// ============================================================================

#[test]
fn tier1_feat09_matches_domain_pattern_exact_match() {
    assert!(matches_domain_pattern("example.com", "example.com"));
    assert!(matches_domain_pattern("api.service.io", "api.service.io"));
    assert!(!matches_domain_pattern("example.com", "other.com"));
}

#[test]
fn tier1_feat09_matches_domain_pattern_case_insensitive() {
    assert!(matches_domain_pattern("EXAMPLE.COM", "example.com"));
    assert!(matches_domain_pattern("sub.Example.Com", "SUB.EXAMPLE.COM"));
}

#[test]
fn tier1_feat09_matches_domain_pattern_wildcard_all() {
    assert!(matches_domain_pattern("*", "anything.com"));
    assert!(matches_domain_pattern("*", "sub.domain.org"));
}

#[test]
fn tier1_feat09_matches_domain_pattern_wildcard_matches_subdomains() {
    assert!(matches_domain_pattern("*.example.com", "sub.example.com"));
    assert!(matches_domain_pattern(
        "*.example.com",
        "deep.nested.example.com"
    ));
    assert!(matches_domain_pattern("*.albedo.dev", "cdn.albedo.dev"));
}

#[test]
fn tier1_feat09_matches_domain_pattern_wildcard_rejects_apex_domain() {
    // W3C CSP3 §6.7.2: *.example.com casa APENAS com subdomínios, e NÃO com o apex domain example.com
    assert!(!matches_domain_pattern("*.example.com", "example.com"));
    assert!(!matches_domain_pattern("*.albedo.dev", "albedo.dev"));
}

#[test]
fn tier1_feat09_matches_domain_pattern_rejects_malicious_subdomain_prefixes() {
    assert!(!matches_domain_pattern(
        "*.example.com",
        "evil-example.com"
    ));
    assert!(!matches_domain_pattern(
        "*.example.com",
        "notexample.com"
    ));
    assert!(!matches_domain_pattern("*.example.com", "example.org"));
}

// ============================================================================
// Feature 10: `percent_decode` Byte Preservation
// ============================================================================

#[test]
fn tier1_feat10_percent_decode_standard_ascii_sequences() {
    assert_eq!(percent_decode("Hello%20World"), "Hello World");
    assert_eq!(percent_decode("%21%23%24"), "!#$");
    assert_eq!(percent_decode("a%2Fb%3Fc%3Dd"), "a/b?c=d");
}

#[test]
fn tier1_feat10_percent_decode_utf8_multibyte_sequences() {
    // á = %C3%A1, é = %C3%A9
    assert_eq!(percent_decode("caf%C3%A9"), "café");
    assert_eq!(percent_decode("ol%C3%A1"), "olá");
    // Emoji 🚀 (%F0%9F%9A%80)
    assert_eq!(percent_decode("space%20%F0%9F%9A%80"), "space 🚀");
}

#[test]
fn tier1_feat10_percent_decode_plain_string_unmodified() {
    assert_eq!(percent_decode("plain_text_without_escapes"), "plain_text_without_escapes");
    assert_eq!(percent_decode("1234567890"), "1234567890");
    assert_eq!(percent_decode(""), "");
}

#[test]
fn tier1_feat10_percent_decode_preserves_malformed_percent_sequences() {
    // "100%_concluido" deve preservar o '%' sem corromper ou descartar caracteres
    let res = percent_decode("100%_concluido");
    assert_eq!(res, "100%_concluido");

    let res_end = percent_decode("discount_50%");
    assert_eq!(res_end, "discount_50%");
}

#[test]
fn tier1_feat10_percent_decode_case_insensitive_hex_digits() {
    assert_eq!(percent_decode("%2f"), "/");
    assert_eq!(percent_decode("%2F"), "/");
    assert_eq!(percent_decode("%3a%3A"), "::");
}

// ============================================================================
// Feature 11: `sniff_mime_type` UTF-8 Boundary
// ============================================================================

#[test]
fn tier1_feat11_sniff_mime_type_image_magic_signatures() {
    assert_eq!(
        sniff_mime_type(b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR"),
        "image/png"
    );
    assert_eq!(
        sniff_mime_type(b"\xFF\xD8\xFF\xE0\x00\x10JFIF"),
        "image/jpeg"
    );
    assert_eq!(sniff_mime_type(b"GIF89a\x01\x00\x01\x00"), "image/gif");
    assert_eq!(sniff_mime_type(b"BM\x36\x00\x00\x00"), "image/bmp");
    assert_eq!(
        sniff_mime_type(b"RIFF\x24\x00\x00\x00WEBPVP8 "),
        "image/webp"
    );
    assert_eq!(
        sniff_mime_type(b"\x00\x00\x01\x00\x01\x00\x20\x20"),
        "image/x-icon"
    );
}

#[test]
fn tier1_feat11_sniff_mime_type_font_and_media_signatures() {
    assert_eq!(sniff_mime_type(b"wOFF\x00\x01\x00\x00"), "font/woff");
    assert_eq!(sniff_mime_type(b"wOF2\x00\x01\x00\x00"), "font/woff2");
    assert_eq!(sniff_mime_type(b"\x00\x01\x00\x00\x00\x01"), "font/ttf");
    assert_eq!(sniff_mime_type(b"OTTO\x00\x01\x00\x00"), "font/otf");
    assert_eq!(sniff_mime_type(b"%PDF-1.7\n%..."), "application/pdf");
    assert_eq!(
        sniff_mime_type(b"RIFF\x24\x00\x00\x00WAVEfmt "),
        "audio/wav"
    );
    assert_eq!(
        sniff_mime_type(b"\x1A\x45\xDF\xA3\x93\x42\x82\x88"),
        "video/webm"
    );
}

#[test]
fn tier1_feat11_sniff_mime_type_html_documents() {
    assert_eq!(
        sniff_mime_type(b"<!DOCTYPE html><html><body>Hello</body></html>"),
        "text/html"
    );
    assert_eq!(
        sniff_mime_type(b"   <html lang=\"en\"><head><title>Test</title></head></html>"),
        "text/html"
    );
    assert_eq!(
        sniff_mime_type(b"<head><meta charset=\"utf-8\"></head>"),
        "text/html"
    );
    assert_eq!(
        sniff_mime_type(b"<body><h1>Direct Body</h1></body>"),
        "text/html"
    );
}

#[test]
fn tier1_feat11_sniff_mime_type_utf8_split_at_512_bytes_boundary() {
    // Cria um buffer de 511 bytes de espaços/ASCII seguido por um caractere UTF-8 multibyte de 3 bytes
    let mut buffer = vec![b' '; 511];
    // Adiciona caractere UTF-8 de 3 bytes (ex: '€' = [0xE2, 0x82, 0xAC])
    buffer.extend_from_slice(&[0xE2, 0x82, 0xAC]);
    // Sniffing de 512 bytes truncaria o '€' no meio. A função deve lidar com isso sem retornar erro.
    let mime = sniff_mime_type(&buffer);
    assert_eq!(mime, "text/plain");
}

#[test]
fn tier1_feat11_sniff_mime_type_svg_and_xml() {
    assert_eq!(
        sniff_mime_type(b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\"></svg>"),
        "image/svg+xml"
    );
    assert_eq!(
        sniff_mime_type(b"<?xml version=\"1.0\"?><root><data>123</data></root>"),
        "application/xml"
    );
}

// ============================================================================
// Feature 12: Event Loop Starvation Prevention
// ============================================================================

#[test]
fn tier1_feat12_event_loop_starvation_task_source_priority_order() {
    let el = EventLoop::new();
    let q = el.handle();
    let execution_order = Arc::new(Mutex::new(Vec::new()));

    let e1 = Arc::clone(&execution_order);
    let e2 = Arc::clone(&execution_order);
    let e3 = Arc::clone(&execution_order);

    // Enfileira de trás para frente (Internal -> Network -> UserInteraction)
    q.queue_task(TaskSource::Internal, move || {
        e1.lock().push("internal");
    });
    q.queue_network(move || {
        e2.lock().push("network");
    });
    q.queue_user_interaction(move || {
        e3.lock().push("user");
    });

    // 1º deve ser UserInteraction
    assert!(el.step());
    assert_eq!(*execution_order.lock(), vec!["user"]);

    // 2º deve ser Network
    assert!(el.step());
    assert_eq!(*execution_order.lock(), vec!["user", "network"]);

    // 3º deve ser Internal
    assert!(el.step());
    assert_eq!(
        *execution_order.lock(),
        vec!["user", "network", "internal"]
    );
}

#[test]
fn tier1_feat12_event_loop_starvation_dom_before_rendering_and_history() {
    let el = EventLoop::new();
    let q = el.handle();
    let order = Arc::new(Mutex::new(Vec::new()));

    let o1 = Arc::clone(&order);
    let o2 = Arc::clone(&order);
    let o3 = Arc::clone(&order);

    q.queue_history(move || o1.lock().push("history"));
    q.queue_rendering(move || o2.lock().push("rendering"));
    q.queue_dom(move || o3.lock().push("dom"));

    assert!(el.step());
    assert_eq!(*order.lock(), vec!["dom"]);

    assert!(el.step());
    assert_eq!(*order.lock(), vec!["dom", "rendering"]);

    assert!(el.step());
    assert_eq!(*order.lock(), vec!["dom", "rendering", "history"]);
}

#[test]
fn tier1_feat12_event_loop_starvation_fifo_within_same_source() {
    let el = EventLoop::new();
    let q = el.handle();
    let list = Arc::new(Mutex::new(Vec::new()));

    for i in 1..=5 {
        let l = Arc::clone(&list);
        q.queue_user_interaction(move || {
            l.lock().push(i);
        });
    }

    for _ in 1..=5 {
        assert!(el.step());
    }
    assert_eq!(*list.lock(), vec![1, 2, 3, 4, 5]);
}

#[test]
fn tier1_feat12_event_loop_starvation_step_returns_false_when_idle() {
    let el = EventLoop::new();
    assert!(!el.step());
    assert!(!el.step());
}

#[test]
fn tier1_feat12_event_loop_starvation_scoped_queue_lifecycle() {
    let el = EventLoop::new();
    let q = el.handle();
    let (scope, scoped_q) = q.create_scope();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    let task_id = scoped_q.queue_task(TaskSource::DomManipulation, move || {
        f.store(true, Ordering::SeqCst);
    });
    assert!(task_id.is_some());

    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));

    // Agora invalida o escopo e tenta enfileirar nova tarefa (deve retornar None e não enfileirar)
    scope.invalidate();
    let fired_after = Arc::new(AtomicBool::new(false));
    let fa = Arc::clone(&fired_after);
    let rejected_id = scoped_q.queue_task(TaskSource::DomManipulation, move || {
        fa.store(true, Ordering::SeqCst);
    });

    assert_eq!(rejected_id, None);
    assert!(!el.step());
    assert!(
        !fired_after.load(Ordering::SeqCst),
        "Invalidated scope should not execute task"
    );
}

// ============================================================================
// Feature 13: Microtask Checkpoint Reentrancy Guard
// ============================================================================

#[test]
fn tier1_feat13_microtask_guard_drained_immediately_after_macrotask() {
    let el = EventLoop::new();
    let q = el.handle();
    let sequence = Arc::new(Mutex::new(Vec::new()));

    let s1 = Arc::clone(&sequence);
    let q1 = q.clone();

    q.queue_user_interaction(move || {
        s1.lock().push("macro_1");

        let s_micro = Arc::clone(&s1);
        q1.queue_microtask(move || {
            s_micro.lock().push("micro_1");
        });
    });

    let s2 = Arc::clone(&sequence);
    q.queue_user_interaction(move || {
        s2.lock().push("macro_2");
    });

    // Passo 1: Macrotask 1 roda e imediatamente ao final drena microtask 1
    assert!(el.step());
    assert_eq!(*sequence.lock(), vec!["macro_1", "micro_1"]);

    // Passo 2: Macrotask 2 roda
    assert!(el.step());
    assert_eq!(*sequence.lock(), vec!["macro_1", "micro_1", "macro_2"]);
}

#[test]
fn tier1_feat13_microtask_guard_nested_microtasks_drained_in_same_checkpoint() {
    let el = EventLoop::new();
    let q = el.handle();
    let count = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&count);
    let q1 = q.clone();

    q.queue_microtask(move || {
        c1.fetch_add(1, Ordering::SeqCst);

        let c2 = Arc::clone(&c1);
        let q2 = q1.clone();
        q1.queue_microtask(move || {
            c2.fetch_add(1, Ordering::SeqCst);

            let c3 = Arc::clone(&c2);
            q2.queue_microtask(move || {
                c3.fetch_add(1, Ordering::SeqCst);
            });
        });
    });

    let drained = el.drain_microtasks();
    assert_eq!(drained, 3);
    assert_eq!(count.load(Ordering::SeqCst), 3);
}

#[test]
fn tier1_feat13_microtask_guard_batch_execution_fifo() {
    let el = EventLoop::new();
    let q = el.handle();
    let output = Arc::new(Mutex::new(Vec::new()));

    for i in 0..100 {
        let out = Arc::clone(&output);
        q.queue_microtask(move || {
            out.lock().push(i);
        });
    }

    let drained = el.drain_microtasks();
    assert_eq!(drained, 100);
    let expected: Vec<i32> = (0..100).collect();
    assert_eq!(*output.lock(), expected);
}

#[test]
fn tier1_feat13_microtask_guard_empty_drain_returns_zero() {
    let el = EventLoop::new();
    assert_eq!(el.drain_microtasks(), 0);
}

#[test]
fn tier1_feat13_microtask_guard_checkpoint_after_animation_frame() {
    let el = EventLoop::new();
    let q = el.handle();
    let executed = Arc::new(AtomicBool::new(false));

    let ex = Arc::clone(&executed);
    let q_clone = q.clone();
    q.request_animation_frame(move || {
        let ex_inner = Arc::clone(&ex);
        q_clone.queue_microtask(move || {
            ex_inner.store(true, Ordering::SeqCst);
        });
    });

    let count = el.process_animation_frame();
    assert_eq!(count, 1);
    assert!(
        executed.load(Ordering::SeqCst),
        "Microtask should be drained after animation frame"
    );
}

// ============================================================================
// Feature 14: Atomic Timer Macrotask Dispatch
// ============================================================================

#[test]
fn tier1_feat14_atomic_timer_dispatch_min_heap_earlier_deadline_fires_first() {
    let clock = Arc::new(MockClock::new(100));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let log = Arc::new(Mutex::new(Vec::new()));

    let l1 = Arc::clone(&log);
    let l2 = Arc::clone(&log);
    let l3 = Arc::clone(&log);

    q.schedule_timer(Duration::from_millis(500), move || l1.lock().push(500));
    q.schedule_timer(Duration::from_millis(100), move || l2.lock().push(100));
    q.schedule_timer(Duration::from_millis(300), move || l3.lock().push(300));

    // Avança 600ms para todos expirarem
    clock.advance_millis(600);
    // Cada step processa os timers expirados e despacha 1 macrotask atômica
    assert!(el.step()); // timer 100ms
    assert!(el.step()); // timer 300ms
    assert!(el.step()); // timer 500ms
    assert_eq!(*log.lock(), vec![100, 300, 500]);
}

#[test]
fn tier1_feat14_atomic_timer_dispatch_simultaneous_deadlines_all_processed() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let counter = Arc::new(AtomicUsize::new(0));

    for _ in 0..5 {
        let c = Arc::clone(&counter);
        q.schedule_timer(Duration::from_millis(200), move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }

    clock.advance_millis(200);
    // 5 macrotasks de timer despachadas
    for _ in 0..5 {
        assert!(el.step());
    }
    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

#[test]
fn tier1_feat14_atomic_timer_dispatch_future_deadlines_remain_pending() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired_early = Arc::new(AtomicBool::new(false));
    let fired_late = Arc::new(AtomicBool::new(false));

    let fe = Arc::clone(&fired_early);
    let fl = Arc::clone(&fired_late);

    q.schedule_timer(Duration::from_millis(100), move || {
        fe.store(true, Ordering::SeqCst);
    });
    q.schedule_timer(Duration::from_millis(500), move || {
        fl.store(true, Ordering::SeqCst);
    });

    clock.advance_millis(150);
    assert!(el.step()); // Executa apenas o timer de 100ms
    assert!(fired_early.load(Ordering::SeqCst));
    assert!(!fired_late.load(Ordering::SeqCst));

    // Timer de 500ms ainda não expirou
    assert!(!el.step());
}

#[test]
fn tier1_feat14_atomic_timer_dispatch_cancellation_skips_execution() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let executed = Arc::new(AtomicBool::new(false));

    let ex = Arc::clone(&executed);
    let (_id, cancel) = q.schedule_timer(Duration::from_millis(50), move || {
        ex.store(true, Ordering::SeqCst);
    });

    // Cancela antes do clock avançar
    cancel.store(true, Ordering::SeqCst);

    clock.advance_millis(100);
    assert!(el.step());
    assert!(
        !executed.load(Ordering::SeqCst),
        "Cancelled timer must not execute callback"
    );
}

#[test]
fn tier1_feat14_atomic_timer_dispatch_zero_delay_fires_immediately_on_step() {
    let clock = Arc::new(MockClock::new(50));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    q.schedule_timer(Duration::ZERO, move || {
        f.store(true, Ordering::SeqCst);
    });

    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

// ============================================================================
// Feature 15: Dynamic Timer Nesting Clamping
// ============================================================================

#[test]
fn tier1_feat15_dynamic_timer_clamping_nesting_below_five_unclamped() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    q.schedule_timer_with_nesting(Duration::from_millis(0), 4, move || {
        f.store(true, Ordering::SeqCst);
    });

    // Em t = 0ms: dispara porque nesting < 5
    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn tier1_feat15_dynamic_timer_clamping_nesting_at_five_clamped_to_4ms() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    q.schedule_timer_with_nesting(Duration::from_millis(0), 5, move || {
        f.store(true, Ordering::SeqCst);
    });

    // Em t = 2ms: ainda NÃO deve disparar
    clock.advance_millis(2);
    assert!(!el.step());
    assert!(!fired.load(Ordering::SeqCst));

    // Em t = 4ms: deve disparar
    clock.advance_millis(2);
    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn tier1_feat15_dynamic_timer_clamping_nesting_deep_recursion_clamped() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    q.schedule_timer_with_nesting(Duration::from_millis(1), 10, move || {
        f.store(true, Ordering::SeqCst);
    });

    clock.advance_millis(3);
    assert!(!el.step());

    clock.advance_millis(1);
    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn tier1_feat15_dynamic_timer_clamping_delay_above_4ms_unaffected() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    let fired = Arc::new(AtomicBool::new(false));

    let f = Arc::clone(&fired);
    q.schedule_timer_with_nesting(Duration::from_millis(20), 5, move || {
        f.store(true, Ordering::SeqCst);
    });

    clock.advance_millis(19);
    assert!(!el.step());

    clock.advance_millis(1);
    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

#[test]
fn tier1_feat15_dynamic_timer_clamping_background_throttling_enforces_1000ms() {
    let clock = Arc::new(MockClock::new(0));
    let el = EventLoop::with_clock(Arc::clone(&clock) as Arc<dyn ace_core::time::Clock>);
    let q = el.handle();
    q.set_background_throttling(true);
    assert!(q.is_background_throttling());

    let fired = Arc::new(AtomicBool::new(false));
    let f = Arc::clone(&fired);
    q.schedule_timer(Duration::from_millis(5), move || {
        f.store(true, Ordering::SeqCst);
    });

    clock.advance_millis(500);
    assert!(!el.step());

    clock.advance_millis(500);
    assert!(el.step());
    assert!(fired.load(Ordering::SeqCst));
}

// ============================================================================
// Feature 16: Bradford Chromatic Adaptation
// ============================================================================

#[test]
fn tier1_feat16_bradford_adaptation_display_p3_to_srgb_primaries() {
    let p3_white = Color::parse("color(display-p3 1 1 1)").unwrap();
    assert_eq!(p3_white, Color::WHITE);

    let p3_black = Color::parse("color(display-p3 0 0 0)").unwrap();
    assert_eq!(p3_black, Color::BLACK);
}

#[test]
fn tier1_feat16_bradford_adaptation_srgb_to_lab_d50_roundtrip() {
    let red = Color::RED;
    let (l, a, b, alpha) = red.to_lab();
    let reconstructed = Color::from_lab(l, a, b, alpha);

    // Delta E (CIE76) entre o original e o reconstruído deve ser desprezível (< 1.0)
    let delta = red.delta_e_76(reconstructed);
    assert!(
        delta < 1.0,
        "Delta E roundtrip too high: {} (original: {:?}, reconstructed: {:?})",
        delta,
        red,
        reconstructed
    );
}

#[test]
fn tier1_feat16_bradford_adaptation_srgb_to_lch_d50_roundtrip() {
    let green = Color::LIME;
    let (l, c, h, alpha) = green.to_lch();
    let reconstructed = Color::from_lch(l, c, h, alpha);

    let delta = green.delta_e_76(reconstructed);
    assert!(delta < 1.0, "Lch roundtrip Delta E too high: {}", delta);
}

#[test]
fn tier1_feat16_bradford_adaptation_srgb_linear_gamma_transfer() {
    let linear_gray = Color::parse("color(srgb-linear 0.5 0.5 0.5)").unwrap();
    // 0.5 linear sRGB mapeia para ~188 em sRGB gama-corrigido
    assert!(linear_gray.r > 180 && linear_gray.r < 195);
    assert_eq!(linear_gray.r, linear_gray.g);
    assert_eq!(linear_gray.g, linear_gray.b);
}

#[test]
fn tier1_feat16_bradford_adaptation_color_space_parse_supported_identifiers() {
    assert_eq!(ColorSpace::parse("srgb"), Some(ColorSpace::Srgb));
    assert_eq!(
        ColorSpace::parse("srgb-linear"),
        Some(ColorSpace::SrgbLinear)
    );
    assert_eq!(ColorSpace::parse("display-p3"), Some(ColorSpace::DisplayP3));
    assert_eq!(ColorSpace::parse("a98-rgb"), Some(ColorSpace::A98Rgb));
    assert_eq!(
        ColorSpace::parse("prophoto-rgb"),
        Some(ColorSpace::ProPhotoRgb)
    );
    assert_eq!(ColorSpace::parse("rec2020"), Some(ColorSpace::Rec2020));
    assert_eq!(ColorSpace::parse("oklab"), Some(ColorSpace::Oklab));
    assert_eq!(ColorSpace::parse("oklch"), Some(ColorSpace::Oklch));
    assert_eq!(ColorSpace::parse("lab"), Some(ColorSpace::Lab));
    assert_eq!(ColorSpace::parse("lch"), Some(ColorSpace::Lch));
    assert_eq!(ColorSpace::parse("hsl"), Some(ColorSpace::Hsl));
    assert_eq!(ColorSpace::parse("hwb"), Some(ColorSpace::Hwb));
    assert_eq!(ColorSpace::parse("unknown-space"), None);
}

#[test]
fn tier1_feat16_bradford_adaptation_oklab_d65_white_point_neutrality() {
    let oklab_white = Color::WHITE.to_oklab();
    assert!(almost_equal(oklab_white.l, 1.0, 0.01));
    assert!(almost_equal(oklab_white.a, 0.0, 0.01));
    assert!(almost_equal(oklab_white.b, 0.0, 0.01));

    let oklab_black = Color::BLACK.to_oklab();
    assert!(almost_equal(oklab_black.l, 0.0, 0.01));
}

// ============================================================================
// Feature 17: CSS Color 4 Polar Interpolation
// ============================================================================

#[test]
fn tier1_feat17_polar_interpolation_oklch_shortest_hue_arc() {
    let c1 = Color::from_oklch(Oklch::new(0.7, 0.2, 10.0, 1.0));
    let c2 = Color::from_oklch(Oklch::new(0.7, 0.2, 350.0, 1.0));

    let mid = c1.interpolate_oklch(c2, 0.5, HueInterpolation::Shorter);
    let mid_oklch = mid.to_oklch();

    // O matiz deve cruzar pelo 0° (ou 360°), nunca por 180°
    let h = mid_oklch.h;
    assert!(
        !(20.0..=340.0).contains(&h),
        "Hue should interpolate across 0/360 boundary, got {}",
        h
    );
}

#[test]
fn tier1_feat17_polar_interpolation_oklch_longer_hue_arc() {
    let c1 = Color::from_oklch(Oklch::new(0.7, 0.2, 10.0, 1.0));
    let c2 = Color::from_oklch(Oklch::new(0.7, 0.2, 350.0, 1.0));

    let mid = c1.interpolate_oklch(c2, 0.5, HueInterpolation::Longer);
    let mid_oklch = mid.to_oklch();

    // O matiz mais longo DEVE cruzar por 180°
    let h = mid_oklch.h;
    assert!(
        (h - 180.0).abs() < 20.0,
        "Longer hue should interpolate through 180°, got {}",
        h
    );
}

#[test]
fn tier1_feat17_polar_interpolation_oklab_perceptual_midpoint() {
    let red = Color::RED;
    let blue = Color::BLUE;

    let mid = red.lerp_oklab(blue, 0.5);
    // Em Oklab, a mistura vermelho + azul preserva brilho uniforme
    assert!(mid.r > 100);
    assert!(mid.b > 100);
    assert_eq!(mid.a, 255);
}

#[test]
fn tier1_feat17_polar_interpolation_color_mix_in_oklch_with_percentages() {
    let mixed = Color::parse("color-mix(in oklch, red 80%, blue 20%)").unwrap();
    assert!(mixed.r > mixed.b);
    assert_eq!(mixed.a, 255);
}

#[test]
fn tier1_feat17_polar_interpolation_color_mix_in_oklab_with_alpha_scaling() {
    let mixed = Color::parse("color-mix(in oklab, red 50%, transparent 50%)").unwrap();
    assert_eq!(mixed.r, 255);
    assert_eq!(mixed.a, 128);
}

#[test]
fn tier1_feat17_polar_interpolation_hwb_hue_whiteness_blackness_conversions() {
    let hwb_red = Color::parse("hwb(0deg 0% 0%)").unwrap();
    assert_eq!(hwb_red, Color::RED);

    let hwb_white = Color::parse("hwb(120deg 100% 0%)").unwrap();
    assert_eq!(hwb_white, Color::WHITE);

    let hwb_black = Color::parse("hwb(240deg 0% 100%)").unwrap();
    assert_eq!(hwb_black, Color::BLACK);
}

// ============================================================================
// Feature 18: `LayoutUnit` Box Snapping
// ============================================================================

#[test]
fn tier1_feat18_layout_unit_snapping_exact_subpixel_divisions() {
    let one_px = LayoutUnit::from_px(1);
    assert_eq!(one_px.raw(), UNITS_PER_PIXEL);

    // Divisão inteira exata sem resíduos por fatores de 60
    assert_eq!((one_px / 2).raw(), 30);
    assert_eq!((one_px / 3).raw(), 20);
    assert_eq!((one_px / 4).raw(), 15);
    assert_eq!((one_px / 5).raw(), 12);
    assert_eq!((one_px / 6).raw(), 10);
    assert_eq!((one_px / 10).raw(), 6);
    assert_eq!((one_px / 12).raw(), 5);
    assert_eq!((one_px / 15).raw(), 4);
    assert_eq!((one_px / 20).raw(), 3);
    assert_eq!((one_px / 30).raw(), 2);
    assert_eq!((one_px / 60).raw(), 1);
}

#[test]
fn tier1_feat18_layout_unit_snapping_floor_ceil_round_px_consistency() {
    let val_pos = LayoutUnit::from_f32_px(4.6);
    assert_eq!(val_pos.floor_px(), 4);
    assert_eq!(val_pos.ceil_px(), 5);
    assert_eq!(val_pos.round_px(), 5);

    let val_neg = LayoutUnit::from_f32_px(-3.2);
    assert_eq!(val_neg.floor_px(), -4);
    assert_eq!(val_neg.ceil_px(), -3);
    assert_eq!(val_neg.round_px(), -3);
}

#[test]
fn tier1_feat18_layout_unit_snapping_snap_box_zero_pixel_cracking() {
    // Invariante Box Snapping: snapped_origin(B1) + snapped_size(B1) == snapped_origin(B2)
    let origin = LayoutUnit::from_f32_px(10.333);
    let size1 = LayoutUnit::from_f32_px(33.333);
    let size2 = LayoutUnit::from_f32_px(50.667);

    let (o1, s1) = origin.snap_box(size1);
    let (o2, s2) = (origin + size1).snap_box(size2);

    assert_eq!(
        o1 + s1,
        o2,
        "Box snapping invariant violated: right_1 ({}) != left_2 ({})",
        o1 + s1,
        o2
    );
    assert!(s1 > 0);
    assert!(s2 > 0);
}

#[test]
fn tier1_feat18_layout_unit_snapping_saturating_overflow_and_underflow() {
    let max = LayoutUnit::from_raw(i32::MAX);
    let extra = LayoutUnit::from_px(5);
    assert_eq!(max.saturating_add(extra), max);

    let min = LayoutUnit::from_raw(i32::MIN);
    assert_eq!(min.saturating_sub(extra), min);
}

#[test]
fn tier1_feat18_layout_unit_snapping_pixel_snapping_across_dpi_scales() {
    // 1.0x (Standard 96 DPI)
    assert_eq!(snap_to_pixel(10.3, 1.0), 10.0);
    assert_eq!(snap_to_pixel(10.7, 1.0), 11.0);

    // 2.0x (Retina)
    assert_eq!(snap_to_pixel(10.25, 2.0), 10.5);
    assert_eq!(snap_to_pixel(10.3, 2.0), 10.5);

    // Scale 0.0 safety
    assert_eq!(snap_to_pixel(15.75, 0.0), 15.75);
}

// ============================================================================
// Feature 19: `style_hint_to_node_flags` Mapping
// ============================================================================

#[test]
fn tier1_feat19_style_hint_mapping_none_produces_clean_flags() {
    let flags = style_hint_to_node_flags(StyleChangeHint::NONE);
    assert_eq!(flags, NodeFlags::empty());
    assert!(!is_node_dirty(flags));
}

#[test]
fn tier1_feat19_style_hint_mapping_repaint_produces_dirty_paint() {
    let flags = style_hint_to_node_flags(StyleChangeHint::REPAINT);
    assert_eq!(flags, NodeFlags::DIRTY_PAINT);
    assert!(is_node_dirty(flags));
}

#[test]
fn tier1_feat19_style_hint_mapping_reflow_produces_layout_and_paint() {
    let flags = style_hint_to_node_flags(StyleChangeHint::REFLOW_LAYOUT);
    assert_eq!(flags, NodeFlags::DIRTY_LAYOUT | NodeFlags::DIRTY_PAINT);
    assert!(is_node_dirty(flags));
}

#[test]
fn tier1_feat19_style_hint_mapping_recalc_style_and_reconstruct_flags() {
    let flags_recalc = style_hint_to_node_flags(StyleChangeHint::RECALC_STYLE);
    assert_eq!(
        flags_recalc,
        NodeFlags::DIRTY_STYLE | NodeFlags::DIRTY_LAYOUT | NodeFlags::DIRTY_PAINT
    );

    let flags_reconstruct = style_hint_to_node_flags(StyleChangeHint::RECONSTRUCT_FRAME);
    assert_eq!(
        flags_reconstruct,
        NodeFlags::DIRTY_STYLE | NodeFlags::DIRTY_LAYOUT | NodeFlags::DIRTY_PAINT
    );
}

#[test]
fn tier1_feat19_style_hint_mapping_subtree_recalc_maps_to_subtree_dirty() {
    let flags = style_hint_to_node_flags(StyleChangeHint::SUBTREE_RECALC);
    assert!(flags.contains(NodeFlags::SUBTREE_DIRTY));
    assert!(flags.contains(NodeFlags::DIRTY_STYLE));
    assert!(is_node_dirty(flags));
}

#[test]
fn tier1_feat19_style_hint_mapping_is_node_dirty_evaluates_all_dirty_bits() {
    assert!(is_node_dirty(NodeFlags::DIRTY_STYLE));
    assert!(is_node_dirty(NodeFlags::DIRTY_LAYOUT));
    assert!(is_node_dirty(NodeFlags::DIRTY_PAINT));
    assert!(is_node_dirty(NodeFlags::SUBTREE_DIRTY));
    assert!(!is_node_dirty(
        NodeFlags::IS_ELEMENT | NodeFlags::IS_CONNECTED
    ));
}
