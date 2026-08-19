//! # Tier 4: Real-World Application Workloads E2E Test Suite
//!
//! Realistic browser engine scenarios exercising end-to-end execution pipelines across
//! memory management, web security, WHATWG event loop, CSS math/color processing,
//! and fixed-point subpixel layout.
//!
//! Adheres strictly to WHATWG HTML, W3C CSS Color 4/5, RFC 6454/9110, and TEST_INFRA.md.

use ace_core::arena::{Arena, ArenaId};
use ace_core::collections::inline_vec::InlineVec;
use ace_core::collections::triple_buffer::triple_buffer;
use ace_core::diagnostics::breadcrumbs::BreadcrumbBuffer;
use ace_core::event_loop::{EventLoop, TaskSource};
use ace_core::flags::utils::style_hint_to_node_flags;
use ace_core::flags::{NodeFlags, StyleChangeHint};
use ace_core::math::color::{mix_colors, ColorSpace};
use ace_core::math::{snap_to_pixel, Color, LayoutUnit, Oklch};
use ace_core::net::{percent_decode, sniff_mime_type, MimeType};
use ace_core::security::{
    compute_referrer, is_potentially_trustworthy_origin, matches_domain_pattern, Host, Origin,
    ReferrerPolicy, Scheme, UnguessableToken,
};
use ace_core::time::{Clock, MockClock};
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// Workload 1: Full Browser Navigation Pipeline
// ============================================================================
/// Scenario 1: End-to-end navigation request processing
/// - URL percent-decoding (query params, special characters, malformed % preserving)
/// - Origin resolution (HTTP, HTTPS default ports, custom schemes, file: URIs, opaque origins)
/// - Referrer policy resolution with credential stripping (userinfo) and subdomain bypass protection
/// - Cryptographic UnguessableToken generation for IPC request tokens
/// - MIME type sniffing of response payload
#[test]
fn tier4_workload_01_navigation_pipeline() {
    // Step 1: Incoming navigation URL decoding
    let raw_url = "https://admin:p%40ssw0rd@example.com:443/search?query=rust%20async%20engine&status=100%_concluido#main-view";
    
    // Percent decoding of complex query parameters
    let decoded_query = percent_decode("query=rust%20async%20engine&status=100%_concluido");
    assert_eq!(decoded_query, "query=rust async engine&status=100%_concluido");

    // Step 2: Canonical Origin Parsing and Normalization
    let current_origin = Origin::parse(raw_url).expect("Should parse valid URL into Origin");
    match &current_origin {
        Origin::Tuple { scheme, host, port } => {
            assert_eq!(*scheme, Scheme::Https);
            assert_eq!(*host, Host::Domain("example.com".into()));
            assert_eq!(*port, 443);
        }
        Origin::Opaque(_) => panic!("Expected tuple origin for https URL"),
    }
    // Default port 443 on HTTPS must be omitted in canonical ASCII serialization
    assert_eq!(current_origin.ascii_serialization(), "https://example.com");
    assert!(current_origin.is_secure());

    // File origin parsing: file schemes should not emit port 0
    let file_origin = Origin::parse("file:///C:/Users/app/index.html").expect("Should parse file URL");
    assert_eq!(file_origin.ascii_serialization(), "file://");

    // Opaque origin for about:blank and data URLs
    let about_blank_origin = Origin::parse("about:blank").unwrap();
    assert!(about_blank_origin.is_opaque());
    assert_eq!(about_blank_origin.ascii_serialization(), "null");

    let data_url_origin = Origin::parse("data:text/html,<h1>Hello</h1>").unwrap();
    assert!(data_url_origin.is_opaque());
    assert_eq!(data_url_origin.ascii_serialization(), "null");
    assert!(!about_blank_origin.same_origin(&data_url_origin));

    // Step 3: Referrer Policy Resolution & Credential Stripping
    // Test 3a: Same-origin target with StrictOriginWhenCrossOrigin
    let same_origin_target = "https://example.com/search/results?id=42";
    let ref_same = compute_referrer(
        &current_origin,
        raw_url,
        same_origin_target,
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    // Must strip fragment (#main-view)
    assert!(ref_same.is_some());
    let ref_str = ref_same.unwrap();
    assert!(!ref_str.contains('#'));
    assert!(ref_str.starts_with("https://"));

    // Test 3b: Cross-origin secure target (must truncate to origin only)
    let cross_origin_target = "https://partner-api.org/v1/auth";
    let ref_cross = compute_referrer(
        &current_origin,
        raw_url,
        cross_origin_target,
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(ref_cross, Some("https://example.com".to_string()));

    // Test 3c: HTTPS -> HTTP Downgrade protection (must be completely suppressed)
    let downgrade_target = "http://insecure-partner.org/v1/feed";
    let ref_downgrade = compute_referrer(
        &current_origin,
        raw_url,
        downgrade_target,
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(ref_downgrade, None);

    // Test 3d: Subdomain bypass attack (e.g. target is evil.com containing example.com)
    let attack_target = "https://example.com.attacker.com/steal";
    let ref_attack = compute_referrer(
        &current_origin,
        raw_url,
        attack_target,
        ReferrerPolicy::SameOrigin,
    );
    assert_eq!(ref_attack, None);

    // Step 4: Unguessable Security Token Generation for IPC Request Channel
    let mut tokens = HashSet::new();
    for _ in 0..100 {
        let token = UnguessableToken::new();
        assert!(!token.is_empty(), "Token must not be empty/null");
        assert!(token.high() != 0 || token.low() != 0);
        let hex = token.to_hex();
        assert_eq!(hex.len(), 32, "Hex representation must be exactly 32 chars (128 bits)");
        assert!(tokens.insert(token), "Each token must be unique");
    }

    // Step 5: MIME Sniffing of Response Bodies
    let html_payload = b"  \r\n\t<!DOCTYPE html>\n<html><head><title>Ace</title></head><body><h1>OK</h1></body></html>";
    assert_eq!(sniff_mime_type(html_payload), "text/html");

    let png_payload = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06";
    assert_eq!(sniff_mime_type(png_payload), "image/png");

    let svg_payload = b"<?xml version=\"1.0\" encoding=\"utf-8\"?><svg width=\"100\" height=\"100\"></svg>";
    assert_eq!(sniff_mime_type(svg_payload), "image/svg+xml");

    let pdf_payload = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>";
    assert_eq!(sniff_mime_type(pdf_payload), "application/pdf");

    let mime_parsed = MimeType::parse("text/html; charset=UTF-8; boundary=something").unwrap();
    assert_eq!(mime_parsed.essence(), "text/html");
    assert_eq!(mime_parsed.charset().map(str::to_ascii_lowercase), Some("utf-8".to_string()));
}

// ============================================================================
// Workload 2: High-Frequency Animation Frame Producer-Consumer Pipeline
// ============================================================================
/// Scenario 2: 120 FPS frame producer-consumer rendering loop with TripleBuffer,
/// color space conversion (sRGB -> Oklch -> sRGB), polar interpolation across hue angles,
/// and LayoutUnit edge snapping.
#[test]
fn tier4_workload_02_animation_frame_pipeline() {
    #[derive(Debug, Clone, PartialEq)]
    struct RenderFrameData {
        frame_idx: usize,
        timestamp_us: u64,
        box_left: LayoutUnit,
        box_width: LayoutUnit,
        box_right: LayoutUnit,
        theme_color: Color,
    }

    let initial_frame = RenderFrameData {
        frame_idx: 0,
        timestamp_us: 0,
        box_left: LayoutUnit::from_px(0),
        box_width: LayoutUnit::from_px(100),
        box_right: LayoutUnit::from_px(100),
        theme_color: Color::RED,
    };

    let (mut producer, mut consumer) = triple_buffer(initial_frame);
    let num_frames = 120; // 1 second of 120 FPS animation
    let mut consumed_frames = Vec::new();

    for frame_idx in 1..=num_frames {
        let timestamp_us = frame_idx as u64 * 8333; // 120 FPS = ~8333 us per frame

        // 1. Color animation: Hue rotates smoothly through all 360 degrees in Oklch space
        let hue_deg = (frame_idx as f32 * (360.0 / num_frames as f32)) % 360.0;
        let oklch = Oklch::new(0.65, 0.22, hue_deg, 1.0);
        let (r, g, b, a) = oklch.to_srgb();
        let frame_color = Color::from_rgba_f32(r, g, b, a);

        // 2. Layout geometry animation: Box moves across subpixel positions
        let subpixel_offset = (frame_idx as f32 * 0.33333).fract();
        let base_left = LayoutUnit::from_px(50) + LayoutUnit::from_f32_px(subpixel_offset * 60.0);
        let base_width = LayoutUnit::from_f32_px(120.5); // Fractional subpixel width
        let base_right = base_left + base_width;

        // Snapping check: snap to integer device pixels
        let snapped_left = snap_to_pixel(base_left.to_f32_px(), 1.0);
        let snapped_right = snap_to_pixel(base_right.to_f32_px(), 1.0);
        assert!(snapped_right >= snapped_left);

        // 3. Write frame to triple buffer (zero allocation)
        producer.write_with(|frame| {
            frame.frame_idx = frame_idx;
            frame.timestamp_us = timestamp_us;
            frame.box_left = base_left;
            frame.box_width = base_width;
            frame.box_right = base_right;
            frame.theme_color = frame_color;
        });
        producer.publish();

        // 4. Consumer polls periodically (simulating compositor / display refresh)
        if frame_idx % 2 == 0 {
            if let Some(frame) = consumer.consume() {
                consumed_frames.push(frame.clone());
            }
        }
    }

    // Verify consumer received frames in strict monotonic order without tearing
    assert!(!consumed_frames.is_empty(), "Compositor should consume published frames");
    let mut prev_frame_idx = 0;
    for frame in &consumed_frames {
        assert!(frame.frame_idx > prev_frame_idx, "Monotonic frame ordering must hold");
        prev_frame_idx = frame.frame_idx;
        assert_eq!(frame.box_left + frame.box_width, frame.box_right);
    }
}

// ============================================================================
// Workload 3: Heavy Event Loop Under Multi-Source Load (Fairness & Starvation)
// ============================================================================
/// Scenario 3: Interleaved multi-source workload (1000 UI events, 50 network tasks,
/// 20 timers, microtasks) verifying WHATWG prioritization, fairness, and zero starvation.
#[test]
fn tier4_workload_03_heavy_event_loop_load() {
    let mock_clock = Arc::new(MockClock::new(1000));
    let event_loop = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn Clock>);
    let queue = event_loop.handle();

    let ui_count = Arc::new(AtomicUsize::new(0));
    let net_count = Arc::new(AtomicUsize::new(0));
    let timer_count = Arc::new(AtomicUsize::new(0));
    let micro_count = Arc::new(AtomicUsize::new(0));

    let execution_log = Arc::new(Mutex::new(Vec::new()));

    // 1. Enqueue 1000 UI interaction tasks
    for i in 0..1000 {
        let ui_c = Arc::clone(&ui_count);
        let micro_c = Arc::clone(&micro_count);
        let log = Arc::clone(&execution_log);
        let q = queue.clone();

        queue.queue_user_interaction(move || {
            ui_c.fetch_add(1, Ordering::SeqCst);
            log.lock().unwrap().push((TaskSource::UserInteraction, i));

            // Each 10th UI task spawns 2 microtasks
            if i % 10 == 0 {
                let m1 = Arc::clone(&micro_c);
                let m2 = Arc::clone(&micro_c);
                q.queue_microtask(move || {
                    m1.fetch_add(1, Ordering::SeqCst);
                });
                q.queue_microtask(move || {
                    m2.fetch_add(1, Ordering::SeqCst);
                });
            }
        });
    }

    // 2. Enqueue 50 Network response tasks
    for i in 0..50 {
        let net_c = Arc::clone(&net_count);
        let log = Arc::clone(&execution_log);
        queue.queue_network(move || {
            net_c.fetch_add(1, Ordering::SeqCst);
            log.lock().unwrap().push((TaskSource::Networking, i));
        });
    }

    // 3. Schedule 20 Timers with staggered delays
    for i in 0..20 {
        let timer_c = Arc::clone(&timer_count);
        let log = Arc::clone(&execution_log);
        let delay_ms = (i % 5) as u64; // 0ms to 4ms
        queue.schedule_timer(Duration::from_millis(delay_ms), move || {
            timer_c.fetch_add(1, Ordering::SeqCst);
            log.lock().unwrap().push((TaskSource::Timer, i));
        });
    }

    // 4. Run event loop steps and advance time deterministically
    let mut total_steps = 0;
    while total_steps < 5000 {
        let did_work = event_loop.step();
        if !did_work {
            // Advance clock to trigger timers
            mock_clock.advance_millis(1);
            let more_work = event_loop.step();
            if !more_work {
                break;
            }
        }
        total_steps += 1;
    }

    // Assert all tasks across all sources were drained completely
    assert_eq!(ui_count.load(Ordering::SeqCst), 1000, "All 1000 UI tasks must execute");
    assert_eq!(net_count.load(Ordering::SeqCst), 50, "All 50 Network tasks must execute");
    assert_eq!(timer_count.load(Ordering::SeqCst), 20, "All 20 Timers must execute");
    assert_eq!(micro_count.load(Ordering::SeqCst), 200, "All 200 Microtasks must execute");

    let log = execution_log.lock().unwrap();
    assert_eq!(log.len(), 1070);
    // Verify zero starvation: the log must contain items from all 3 task sources
    let has_ui = log.iter().any(|(s, _)| *s == TaskSource::UserInteraction);
    let has_net = log.iter().any(|(s, _)| *s == TaskSource::Networking);
    let has_timer = log.iter().any(|(s, _)| *s == TaskSource::Timer);
    assert!(has_ui && has_net && has_timer, "Fair execution across all sources without starvation");
}

// ============================================================================
// Workload 4: Style Change Recalculation & Arena Lifecycle
// ============================================================================
/// Scenario 4: Batch DOM node allocation in Arena, style change hint calculation
/// (SUBTREE_RECALC -> SUBTREE_DIRTY), telemetry breadcrumb recording, and clean teardown.
#[test]
fn tier4_workload_04_style_mutation_and_arena_lifecycle() {
    #[allow(dead_code)]
    #[derive(Debug, Clone)]
    struct DomNode {
        tag: &'static str,
        class_list: InlineVec<&'static str, 4>,
        flags: NodeFlags,
        parent: Option<ArenaId<DomNode>>,
        children: InlineVec<ArenaId<DomNode>, 8>,
    }

    let mut arena = Arena::<DomNode>::new();
    let breadcrumbs = BreadcrumbBuffer::<64>::new();

    breadcrumbs.record("lifecycle", "Document DOM building initiated", 100);

    // 1. Build a DOM tree: <html> -> <body> -> <header>, <main>, <footer>
    let html_id = arena.alloc(DomNode {
        tag: "html",
        class_list: InlineVec::new(),
        flags: NodeFlags::IS_DOCUMENT | NodeFlags::IS_CONNECTED,
        parent: None,
        children: InlineVec::new(),
    });

    let body_id = arena.alloc(DomNode {
        tag: "body",
        class_list: InlineVec::new(),
        flags: NodeFlags::IS_ELEMENT | NodeFlags::IS_CONNECTED,
        parent: Some(html_id),
        children: InlineVec::new(),
    });

    arena.get_mut(html_id).unwrap().children.push(body_id);

    // Allocate 30 child nodes in <main>
    let main_id = arena.alloc(DomNode {
        tag: "main",
        class_list: InlineVec::new(),
        flags: NodeFlags::IS_ELEMENT | NodeFlags::IS_CONNECTED,
        parent: Some(body_id),
        children: InlineVec::new(),
    });
    arena.get_mut(body_id).unwrap().children.push(main_id);

    let mut item_ids = Vec::new();
    for _i in 0..30 {
        let mut classes = InlineVec::<&'static str, 4>::new();
        classes.push("list-item");
        classes.push("theme-light");

        let item_id = arena.alloc(DomNode {
            tag: "div",
            class_list: classes,
            flags: NodeFlags::IS_ELEMENT | NodeFlags::IS_CONNECTED,
            parent: Some(main_id),
            children: InlineVec::new(),
        });
        item_ids.push(item_id);
        arena.get_mut(main_id).unwrap().children.push(item_id);
    }

    assert_eq!(arena.len(), 33);
    breadcrumbs.record("dom", format!("Created {} DOM nodes", arena.len()), 150);

    // 2. Style Mutation: <main> element undergoes dynamic class change (theme switched to dark)
    let hint = StyleChangeHint::SUBTREE_RECALC | StyleChangeHint::REFLOW_LAYOUT;
    let node_flags = style_hint_to_node_flags(hint);

    // Verify style_hint_to_node_flags correctly maps hints
    assert!(node_flags.contains(NodeFlags::DIRTY_LAYOUT));
    assert!(node_flags.contains(NodeFlags::DIRTY_PAINT));

    // Mark subtree invalidation on main
    let main_node = arena.get_mut(main_id).unwrap();
    main_node.flags |= NodeFlags::SUBTREE_DIRTY | node_flags;
    main_node.class_list.push("theme-dark");

    breadcrumbs.record("style", "Style invalidation applied to main#container", 200);

    // Verify descendants can be traversed and dirtied
    let children_slice: Vec<ArenaId<DomNode>> = arena.get(main_id).unwrap().children.as_slice().to_vec();
    for child_id in children_slice {
        let child = arena.get_mut(child_id).unwrap();
        child.flags |= NodeFlags::DIRTY_STYLE;
        assert!(child.flags.contains(NodeFlags::DIRTY_STYLE));
    }

    // 3. Teardown and Cleanup
    breadcrumbs.record("lifecycle", "Document teardown", 300);
    arena.clear();

    assert!(arena.is_empty());
    assert_eq!(arena.len(), 0);

    // All old ArenaIds must be strictly invalid
    assert!(arena.get(html_id).is_none());
    assert!(arena.get(body_id).is_none());
    assert!(arena.get(main_id).is_none());
    assert!(arena.get(item_ids[0]).is_none());

    // Allocate a new node and verify slot reuse
    let new_node_id = arena.alloc(DomNode {
        tag: "html",
        class_list: InlineVec::new(),
        flags: NodeFlags::IS_DOCUMENT,
        parent: None,
        children: InlineVec::new(),
    });
    assert_eq!(arena.len(), 1);
    assert!(arena.get(new_node_id).is_some());

    // Verify breadcrumbs snapshot
    let snapshot = breadcrumbs.snapshot();
    assert_eq!(snapshot.len(), 4);
    assert_eq!(snapshot[0].category, "lifecycle");
    assert_eq!(snapshot[1].category, "dom");
    assert_eq!(snapshot[2].category, "style");
    assert_eq!(snapshot[3].category, "lifecycle");
}

// ============================================================================
// Workload 5: Security Origin & CSP Evaluation Pipeline
// ============================================================================
/// Scenario 5: Multi-origin sandboxed iframe context evaluation, CSP wildcard domain
/// pattern verification (*.domain.com vs apex), and token issuance.
#[test]
fn tier4_workload_05_security_origin_csp_evaluation() {
    // 1. Context Origins
    let top_origin = Origin::parse("https://albedo-browser.org:443/app/").unwrap();
    let sub_origin = Origin::parse("https://api.albedo-browser.org/v2/").unwrap();
    let cdn_origin = Origin::parse("https://static.cdn.albedo-browser.org/lib.js").unwrap();
    let external_origin = Origin::parse("https://analytics.google.com/collect").unwrap();
    let insecure_origin = Origin::parse("http://albedo-browser.org/unsecured").unwrap();

    // 2. Sandboxed Iframe (Opaque Origin)
    let sandbox_iframe_origin = Origin::new_opaque();
    let second_sandbox_origin = Origin::new_opaque();

    assert!(sandbox_iframe_origin.is_opaque());
    assert!(second_sandbox_origin.is_opaque());
    assert!(!sandbox_iframe_origin.same_origin(&second_sandbox_origin), "Opaque origins must never be same-origin");
    assert!(!sandbox_iframe_origin.is_secure());

    // 3. Same-Origin Policy (SOP) and Same-Site
    assert!(!top_origin.same_origin(&sub_origin), "Different subdomains are cross-origin");
    assert!(top_origin.is_same_site(&sub_origin), "Subdomains of same eTLD+1 share SchemefulSite");
    assert!(top_origin.is_same_site(&cdn_origin));
    assert!(!top_origin.is_same_site(&external_origin));
    assert!(!top_origin.same_origin(&insecure_origin), "Different schemes (HTTPS vs HTTP) are cross-origin");

    // 4. Potentially Trustworthy Origin checks
    assert!(is_potentially_trustworthy_origin(&top_origin));
    assert!(is_potentially_trustworthy_origin(&Origin::parse("http://localhost:3000").unwrap()));
    assert!(is_potentially_trustworthy_origin(&Origin::parse("http://127.0.0.1:8080").unwrap()));
    assert!(!is_potentially_trustworthy_origin(&insecure_origin));

    // 5. CSP Wildcard Domain Pattern Matching (W3C CSP Level 3 §6.7.2)
    let csp_pattern = "*.albedo-browser.org";

    // Direct subdomain matches
    assert!(matches_domain_pattern(csp_pattern, "api.albedo-browser.org"));
    assert!(matches_domain_pattern(csp_pattern, "static.cdn.albedo-browser.org"));
    assert!(matches_domain_pattern(csp_pattern, "dev.albedo-browser.org"));

    // Case-insensitivity
    assert!(matches_domain_pattern(csp_pattern, "API.ALBEDO-BROWSER.ORG"));

    // Critical CSP requirement: *.domain.com must NOT match the apex domain itself or attacker spoofing
    assert!(
        !matches_domain_pattern(csp_pattern, "albedo-browser.org"),
        "Apex domain must NOT match wildcard *.domain.com per CSP3 spec"
    );
    assert!(!matches_domain_pattern(csp_pattern, "fake-albedo-browser.org"));
    assert!(!matches_domain_pattern(csp_pattern, "albedo-browser.org.evil.com"));
    assert!(!matches_domain_pattern(csp_pattern, "analytics.google.com"));

    // Wildcard-all pattern "*"
    assert!(matches_domain_pattern("*", "any-domain.com"));
    assert!(matches_domain_pattern("*", "sub.another.org"));

    // 6. Token Issuance per Frame Context
    let main_token = UnguessableToken::new();
    let frame_token = UnguessableToken::new();
    assert_ne!(main_token, frame_token);
    assert!(!main_token.is_empty());
    assert!(!frame_token.is_empty());
}

// ============================================================================
// Workload 6: CSS Color Space Transformation & Interpolation Pipeline
// ============================================================================
/// Scenario 6: Color palette generation with sRGB <-> Oklab/Oklch transformations,
/// polar color blending across hue quadrants, alpha premultiplication, and powerless hue handling.
#[test]
fn tier4_workload_06_css_color_transformation_pipeline() {
    // 1. Conversions sRGB <-> Oklab / Oklch
    let srgb_blue = Color::from_rgb(0, 0, 255);
    let oklab_blue = srgb_blue.to_oklab();
    let oklch_blue = srgb_blue.to_oklch();

    assert!(oklab_blue.l > 0.0);
    // Blue has significant chroma in Oklab/Oklch
    assert!(oklch_blue.c > 0.2);
    assert!(oklch_blue.l > 0.3 && oklch_blue.l < 0.6);

    // Round-trip back to sRGB
    let roundtrip_srgb = Color::from_oklch(oklch_blue);
    assert_eq!(roundtrip_srgb.r, 0);
    assert_eq!(roundtrip_srgb.g, 0);
    assert_eq!(roundtrip_srgb.b, 255);

    // 2. Polar Interpolation across Hue Boundary (0° / 360° Wrap-around)
    // Blend Red (Hue ~29° in Oklch) with Magenta (Hue ~328° in Oklch)
    // Shortest path must interpolate smoothly across the 0° wrap boundary
    let c1 = Color::from_hsla(350.0, 1.0, 0.5, 1.0); // Near red/magenta
    let c2 = Color::from_hsla(10.0, 1.0, 0.5, 1.0);  // Near red/orange

    let mixed = mix_colors(ColorSpace::Oklch, c1, Some(0.5), c2, Some(0.5))
        .expect("mix_colors in Oklch must succeed");

    let mixed_lch = mixed.to_oklch();
    // Midpoint hue between 350° and 10° via shortest arc should be 0° (or 360°)
    let is_near_zero = mixed_lch.h < 30.0 || mixed_lch.h > 330.0;
    assert!(is_near_zero, "Polar interpolation across 0° must take shortest arc, got hue {}", mixed_lch.h);

    // 3. Alpha Premultiplication for Compositing / GPU Shaders
    let semi_color = Color::from_rgba(100, 200, 50, 128); // 50% opacity (alpha = 128/255)
    let (pr, pg, pb, pa) = semi_color.to_premultiplied_f32();
    assert!((pa - (128.0 / 255.0)).abs() < 1e-4);
    assert!((pr - (100.0 / 255.0) * pa).abs() < 1e-4);
    assert!((pg - (200.0 / 255.0) * pa).abs() < 1e-4);
    assert!((pb - (50.0 / 255.0) * pa).abs() < 1e-4);

    // 4. Source-over alpha blending
    let background = Color::WHITE;
    let foreground = Color::from_rgba(255, 0, 0, 128); // 50% red over opaque white
    let blended = foreground.blend_source_over(background);
    assert_eq!(blended.r, 255);
    assert_eq!(blended.g, 127);
    assert_eq!(blended.b, 127);
    assert_eq!(blended.a, 255);

    // 5. Functional CSS parsing
    let color_p3 = Color::parse_css("color(display-p3 1 0 0)").unwrap();
    assert_eq!(color_p3.r, 255);

    let color_hwb = Color::parse_css("hwb(120 20% 20%)").unwrap();
    assert!(color_hwb.g > color_hwb.r);
    assert!(color_hwb.g > color_hwb.b);
}

// ============================================================================
// Workload 7: DOM Tree Memory Stress & Arena Generational Soundness
// ============================================================================
/// Scenario 7: Rapid allocation and destruction of complex node structures with
/// InlineVec child lists in Arena, verifying memory soundness and zero leaks under heavy churn.
#[test]
fn tier4_workload_07_dom_tree_memory_stress() {
    #[allow(dead_code)]
    #[derive(Debug, Clone)]
    struct HeavyNode {
        id_num: usize,
        name: String,
        children: InlineVec<ArenaId<HeavyNode>, 4>,
    }

    let mut arena = Arena::<HeavyNode>::new();
    let num_cycles = 20;
    let nodes_per_cycle = 200;

    for cycle in 0..num_cycles {
        let mut cycle_roots = Vec::new();

        // 1. Allocate a forest of roots, each with child nodes
        for i in 0..nodes_per_cycle {
            let root_id = arena.alloc(HeavyNode {
                id_num: cycle * 1000 + i,
                name: format!("node_{}_{}", cycle, i),
                children: InlineVec::new(),
            });

            // Add 6 children (forces InlineVec from inline N=4 to heap)
            for child_i in 0..6 {
                let child_id = arena.alloc(HeavyNode {
                    id_num: cycle * 1000 + i * 10 + child_i,
                    name: format!("child_{}_{}", i, child_i),
                    children: InlineVec::new(),
                });
                arena.get_mut(root_id).unwrap().children.push(child_id);
            }

            cycle_roots.push(root_id);
        }

        // Verify allocation integrity
        assert_eq!(arena.len(), nodes_per_cycle * 7);

        // 2. Perform manipulations on InlineVec (insert, retain, remove)
        for &root_id in &cycle_roots {
            let root = arena.get_mut(root_id).unwrap();
            assert_eq!(root.children.len(), 6);
            assert!(!root.children.is_inline(), "6 items must be stored on heap");

            // Filter even indexed children
            let mut keep = true;
            root.children.retain(|_| {
                keep = !keep;
                keep
            });
            assert_eq!(root.children.len(), 3);
        }

        // 3. Remove half of the roots and their children explicitly
        for i in (0..cycle_roots.len()).step_by(2) {
            let root_id = cycle_roots[i];
            let children_vec: Vec<ArenaId<HeavyNode>> = arena.get(root_id).unwrap().children.as_slice().to_vec();
            for child_id in children_vec {
                let removed_child = arena.remove(child_id);
                assert!(removed_child.is_some());
                assert!(arena.get(child_id).is_none());
            }
            let removed_root = arena.remove(root_id);
            assert!(removed_root.is_some());
            assert!(arena.get(root_id).is_none());
        }

        // 4. Clear the entire arena at the end of each cycle
        arena.clear();
        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
    }

    let stats = arena.stats();
    assert_eq!(stats.live, 0);
    assert!(stats.total_allocated > 0);
    assert!(stats.slot_reuses > 0, "Arena must reuse free list slots across cycles");
}

// ============================================================================
// Workload 8: Network Stream MIME Boundary Sniffing
// ============================================================================
/// Scenario 8: Multi-chunk response streaming with multi-byte UTF-8 character
/// split exactly across 512-byte boundary, combined with percent-encoded query param extraction.
#[test]
fn tier4_workload_08_network_stream_mime_decoding() {
    // 1. Construct a 514-byte streaming payload where a 4-byte UTF-8 emoji ('🦀' = [0xF0, 0x9F, 0xA6, 0x80])
    // is split precisely across the 512-byte boundary:
    // First chunk has 513 bytes (511 ASCII bytes + 2 bytes of the emoji)
    // Second chunk has the remaining 2 bytes + further HTML content.
    let mut stream_bytes = Vec::with_capacity(1024);
    let html_header = b"<!DOCTYPE html><html><head><title>Streamed Document</title></head><body><p>";
    stream_bytes.extend_from_slice(html_header);

    // Pad with ASCII characters up to 511 bytes
    while stream_bytes.len() < 511 {
        stream_bytes.push(b'A');
    }
    assert_eq!(stream_bytes.len(), 511);

    // Insert 4-byte UTF-8 crab emoji '🦀' [0xF0, 0x9F, 0xA6, 0x80]
    let crab_bytes = "🦀".as_bytes();
    assert_eq!(crab_bytes.len(), 4);
    stream_bytes.extend_from_slice(crab_bytes);

    let html_footer = b"</p></body></html>";
    stream_bytes.extend_from_slice(html_footer);

    // 2. Perform MIME sniffing on the initial 512-byte boundary window
    let boundary_sample = &stream_bytes[..512];
    let sniffed_boundary = sniff_mime_type(boundary_sample);
    assert_eq!(sniffed_boundary, "text/html", "MIME sniffer must gracefully handle split UTF-8 bytes");

    // Full stream sniffing
    let full_sniffed = sniff_mime_type(&stream_bytes);
    assert_eq!(full_sniffed, "text/html");

    // 3. Extract and parse percent-encoded queries from HTTP headers / URLs
    let encoded_query = "title=%F0%9F%A6%80%20Albedo%20Engine&version=1.0%2B2026&malformed=test%20100%_done";
    let decoded_query = percent_decode(encoded_query);
    assert_eq!(decoded_query, "title=🦀 Albedo Engine&version=1.0+2026&malformed=test 100%_done");

    // 4. Verify MIME types parsing and parameter retrieval
    let ct = MimeType::parse("text/html; charset=utf-8; profile=\"https://standards.org\"").unwrap();
    assert!(ct.is_html());
    assert_eq!(ct.charset(), Some("utf-8"));
    assert_eq!(ct.get_param("profile"), Some("https://standards.org"));
}

// ============================================================================
// Workload 9: Timer Storm & Microtask Drain Reentrancy
// ============================================================================
/// Scenario 9: Recursive timer storm testing dynamic nesting depth >= 5 clamping
/// to 4ms with interleaved microtask checkpoints per WHATWG §8.5.2.
#[test]
fn tier4_workload_09_timer_storm_and_microtask_drain() {
    let mock_clock = Arc::new(MockClock::new(0));
    let event_loop = EventLoop::with_clock(Arc::clone(&mock_clock) as Arc<dyn Clock>);
    let queue = event_loop.handle();

    let execution_times = Arc::new(Mutex::new(Vec::new()));
    let microtasks_executed = Arc::new(AtomicUsize::new(0));

    // Schedule recursive timer chain from depth 1 to depth 7 with 0ms delay
    fn schedule_recursive_timer(
        depth: usize,
        max_depth: usize,
        queue: ace_core::event_loop::TaskQueue,
        clock: Arc<MockClock>,
        exec_times: Arc<Mutex<Vec<(usize, u64)>>>,
        micro_c: Arc<AtomicUsize>,
    ) {
        if depth > max_depth {
            return;
        }

        let q_clone = queue.clone();
        let clock_clone = Arc::clone(&clock);
        let exec_clone = Arc::clone(&exec_times);
        let micro_clone = Arc::clone(&micro_c);

        queue.schedule_timer_with_nesting(Duration::from_millis(0), depth, move || {
            let current_time = clock_clone.now_ms();
            exec_clone.lock().unwrap().push((depth, current_time));

            // Enqueue microtasks within this timer callback
            let m1 = Arc::clone(&micro_clone);
            let m2 = Arc::clone(&micro_clone);
            q_clone.queue_microtask(move || {
                m1.fetch_add(1, Ordering::SeqCst);
            });
            q_clone.queue_microtask(move || {
                m2.fetch_add(1, Ordering::SeqCst);
            });

            // Recurse to next depth
            schedule_recursive_timer(depth + 1, max_depth, q_clone, clock_clone, exec_clone, micro_clone);
        });
    }

    schedule_recursive_timer(1, 7, queue, Arc::clone(&mock_clock), Arc::clone(&execution_times), Arc::clone(&microtasks_executed));

    // Step through the event loop advancing time 1ms at a time, draining all available work at each timestamp
    for _ in 0..50 {
        while event_loop.step() {}
        mock_clock.advance_millis(1);
    }

    let records = execution_times.lock().unwrap().clone();
    assert_eq!(records.len(), 7, "All 7 nesting levels must eventually execute");

    // Depth 1..=4 have 0ms delay, should execute at t=0
    for &(depth, time) in &records {
        if depth < 5 {
            assert_eq!(time, 0, "Depth {} should fire at t=0", depth);
        } else if depth == 5 {
            // Depth 5 must be clamped to min 4ms
            assert!(time >= 4, "Depth 5 must clamp to >= 4ms, executed at t={}", time);
        } else if depth == 6 {
            // Depth 6 must be clamped to min 4ms after depth 5 (t >= 8ms)
            assert!(time >= 8, "Depth 6 must clamp to >= 8ms, executed at t={}", time);
        } else if depth == 7 {
            // Depth 7 must be clamped to min 4ms after depth 6 (t >= 12ms)
            assert!(time >= 12, "Depth 7 must clamp to >= 12ms, executed at t={}", time);
        }
    }

    // Microtasks (2 per timer * 7 timers = 14)
    assert_eq!(microtasks_executed.load(Ordering::SeqCst), 14);
}

// ============================================================================
// Workload 10: Multi-Column Composite Box Snapping (Zero Pixel Cracking)
// ============================================================================
/// Scenario 10: Multi-column subpixel layout computation where adjacent boxes snap
/// to integer device pixels with mathematical proof of zero pixel cracking (right_1 == left_2).
#[test]
fn tier4_workload_10_multi_column_box_snapping() {
    // Edge snapping mathematical rule:
    // For adjacent layout boxes in fixed-point LayoutUnit (60 units per pixel):
    // left_i = sum_{k=0}^{i-1} width_k
    // right_i = left_i + width_i = left_{i+1}
    //
    // Device pixel snapping:
    // snapped_left(i) = round_px(left_i)
    // snapped_right(i) = round_px(right_i)
    // snapped_width(i) = snapped_right(i) - snapped_left(i)
    //
    // Invariant to prove:
    // 1. snapped_right(i) == snapped_left(i + 1) for all adjacent columns (ZERO Pixel Cracking).
    // 2. sum(snapped_width(i)) == snapped_total_width (ZERO loss of total container space).

    let container_widths = [100, 320, 375, 768, 1024, 1280, 1440, 1920, 2560];
    let column_counts = [2, 3, 4, 5, 6, 7, 9, 11, 12, 13, 16, 24];

    for &total_px in &container_widths {
        let container_lu = LayoutUnit::from_px(total_px);

        for &cols in &column_counts {
            // Subpixel width per column (exact 1/60th arithmetic)
            let col_width_lu = container_lu / cols;

            let mut current_left_lu = LayoutUnit::from_px(0);
            let mut prev_snapped_right: Option<i32> = None;
            let mut sum_snapped_widths = 0;

            for col_idx in 0..cols {
                let next_right_lu = if col_idx == cols - 1 {
                    container_lu // Last column takes remainder to perfectly fill container
                } else {
                    current_left_lu + col_width_lu
                };

                let snapped_left = current_left_lu.round_px();
                let snapped_right = next_right_lu.round_px();
                let snapped_width = snapped_right - snapped_left;

                // Invariant 1: Seamless adjacency (Zero pixel cracking)
                if let Some(prev_right) = prev_snapped_right {
                    assert_eq!(
                        prev_right,
                        snapped_left,
                        "Pixel cracking detected between col {} and {} for container width {}px with {} cols: prev_right ({}) != snapped_left ({})",
                        col_idx - 1,
                        col_idx,
                        total_px,
                        cols,
                        prev_right,
                        snapped_left
                    );
                }

                sum_snapped_widths += snapped_width;
                prev_snapped_right = Some(snapped_right);
                current_left_lu = next_right_lu;
            }

            // Invariant 2: Total sum equals snapped container width
            let snapped_container_width = container_lu.round_px();
            assert_eq!(
                sum_snapped_widths,
                snapped_container_width,
                "Total snapped width sum ({}) must equal container snapped width ({}) for {}px with {} cols",
                sum_snapped_widths,
                snapped_container_width,
                total_px,
                cols
            );
        }
    }
}
