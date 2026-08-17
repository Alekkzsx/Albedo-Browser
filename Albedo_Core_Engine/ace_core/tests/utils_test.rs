use ace_core::arena::utils as arena_utils;
use ace_core::collections::utils as col_utils;
use ace_core::cursor::utils as cursor_utils;
use ace_core::cursor::CharCursor;
use ace_core::event_loop::utils as el_utils;
use ace_core::features::utils as feature_utils;
use ace_core::features::Feature;
use ace_core::flags::utils as flag_utils;
use ace_core::flags::{NodeFlags, StyleChangeHint};
use ace_core::math::utils as math_utils;
use ace_core::net::utils as net_utils;
use ace_core::performance::utils as perf_utils;
use ace_core::security::utils as sec_utils;
use ace_core::text::utils as text_utils;
use ace_core::utils as root_utils;
use std::time::Duration;

#[test]
fn test_arena_utils() {
    assert_eq!(arena_utils::align_up(0, 8), 0);
    assert_eq!(arena_utils::align_up(1, 8), 8);
    assert_eq!(arena_utils::align_up(8, 8), 8);
    assert_eq!(arena_utils::align_up(9, 8), 16);

    assert!(arena_utils::is_aligned(16, 8));
    assert!(!arena_utils::is_aligned(17, 8));

    assert_eq!(arena_utils::calc_growth_capacity(10, 12), 18);
    assert_eq!(arena_utils::calc_growth_capacity(10, 50), 50);

    assert_eq!(arena_utils::format_bytes(500), "500 B");
    assert_eq!(arena_utils::format_bytes(1024), "1.00 KB");
    assert_eq!(arena_utils::format_bytes(1024 * 1024), "1.00 MB");
}

#[test]
fn test_math_utils() {
    assert_eq!(math_utils::lerp(0.0, 100.0, 0.5), 50.0);
    assert_eq!(math_utils::clamp(150.0, 0.0, 100.0), 100.0);
    assert_eq!(math_utils::saturate(-0.5), 0.0);
    assert_eq!(math_utils::saturate(1.5), 1.0);
    assert!(math_utils::almost_equal(1.00001, 1.00002, 1e-4));

    assert_eq!(math_utils::deg_to_rad(180.0), std::f32::consts::PI);
    assert_eq!(math_utils::rad_to_deg(std::f32::consts::PI), 180.0);

    assert_eq!(math_utils::snap_to_pixel(10.333, 1.0), 10.0);
    assert_eq!(math_utils::snap_to_pixel(10.666, 1.0), 11.0);

    assert_eq!(math_utils::min3(10.0, 5.0, 20.0), 5.0);
    assert_eq!(math_utils::max3(10.0, 5.0, 20.0), 20.0);
    assert_eq!(math_utils::min4(10.0, 5.0, 20.0, 2.0), 2.0);
    assert_eq!(math_utils::max4(10.0, 5.0, 20.0, 25.0), 25.0);
}

#[test]
fn test_collections_utils() {
    assert_eq!(col_utils::next_power_of_two(0), 1);
    assert_eq!(col_utils::next_power_of_two(1), 1);
    assert_eq!(col_utils::next_power_of_two(5), 8);
    assert_eq!(col_utils::next_power_of_two(8), 8);
    assert_eq!(col_utils::next_power_of_two(9), 16);

    assert_eq!(col_utils::popcount_u64(0b10110), 3);
    assert_eq!(col_utils::trailing_zeros_u64(0b1000), 3);
    assert_eq!(col_utils::leading_zeros_u64(0), 64);
    assert!(col_utils::has_single_bit(16));
    assert!(!col_utils::has_single_bit(18));

    let data = vec![1, 2, 3, 4, 5];
    let chunks: Vec<_> = col_utils::chunk_slice(&data, 2).collect();
    assert_eq!(chunks, vec![&[1, 2][..], &[3, 4][..], &[5][..]]);
}

#[test]
fn test_event_loop_utils() {
    assert_eq!(el_utils::fps_to_interval(60), Duration::from_nanos(16_666_666));
    assert_eq!(el_utils::ms_to_duration(100), Duration::from_millis(100));
    assert_eq!(el_utils::duration_to_ms(Duration::from_millis(250)), 250);

    let deadline = el_utils::compute_deadline(1000, Duration::from_millis(500));
    assert_eq!(deadline, 1500);
    assert!(!el_utils::is_deadline_passed(1499, deadline));
    assert!(el_utils::is_deadline_passed(1500, deadline));
    assert!(el_utils::is_deadline_passed(1501, deadline));
}

#[test]
fn test_cursor_utils() {
    let mut c = CharCursor::new("  \t\n -42 12.75 #FFA0");
    assert_eq!(cursor_utils::skip_ascii_whitespace(&mut c), 5);
    assert_eq!(cursor_utils::parse_i32(&mut c), Some(-42));
    assert_eq!(cursor_utils::skip_ascii_whitespace(&mut c), 1);
    assert_eq!(cursor_utils::parse_f32(&mut c), Some(12.75));
    assert_eq!(cursor_utils::skip_ascii_whitespace(&mut c), 1);
    assert_eq!(c.advance(), Some('#'));
    assert_eq!(cursor_utils::parse_hex_u32(&mut c, 4), Some(0xFFA0));
}

#[test]
fn test_net_utils() {
    let data_uri = "data:text/plain;base64,SGVsbG8gV29ybGQ=";
    let (mime, bytes) = net_utils::parse_data_uri(data_uri).unwrap();
    assert_eq!(mime.essence(), "text/plain");
    assert_eq!(String::from_utf8(bytes).unwrap(), "Hello World");

    assert_eq!(net_utils::percent_decode("Hello%20Albedo%21"), "Hello Albedo!");
    assert!(net_utils::is_safe_url_scheme("https"));
    assert!(!net_utils::is_safe_url_scheme("javascript"));
}

#[test]
fn test_security_utils() {
    assert!(sec_utils::matches_domain_pattern("*.example.com", "api.example.com"));
    assert!(sec_utils::matches_domain_pattern("*.example.com", "example.com"));
    assert!(!sec_utils::matches_domain_pattern("*.example.com", "other.org"));
    assert!(sec_utils::matches_domain_pattern("*", "anything.io"));
}

#[test]
fn test_text_utils() {
    assert_eq!(text_utils::escape_html("<script>alert('xss')</script>"), "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;");
    assert_eq!(text_utils::escape_css_identifier("btn:primary"), "btn\\:primary");
    assert!(text_utils::is_ascii_case_insensitive_equal("Content-Type", "content-type"));
}

#[test]
fn test_performance_utils() {
    let samples = [10.0, 20.0, 30.0, 40.0, 50.0];
    let p = perf_utils::calculate_percentiles(&samples).unwrap();
    assert_eq!(p.min, 10.0);
    assert_eq!(p.max, 50.0);
    assert_eq!(p.mean, 30.0);
    assert_eq!(p.p50, 30.0);

    assert_eq!(perf_utils::format_duration_human(1500.0), "1.50 s");
    assert_eq!(perf_utils::format_duration_human(12.5), "12.50 ms");
    assert_eq!(perf_utils::format_duration_human(0.45), "450.00 µs");
}

#[test]
fn test_feature_and_flag_utils() {
    let overrides = feature_utils::parse_feature_overrides("CssSubgrid,-WebAssembly");
    assert_eq!(overrides, vec![(Feature::CssSubgrid, true), (Feature::WebAssembly, false)]);

    let hint = StyleChangeHint::REFLOW_LAYOUT;
    let flags = flag_utils::style_hint_to_node_flags(hint);
    assert!(flags.contains(NodeFlags::DIRTY_LAYOUT));
    assert!(flag_utils::is_node_dirty(flags));
}

#[test]
fn test_root_utils() {
    let hash1 = root_utils::fast_hash("hello");
    let hash2 = root_utils::fast_hash("hello");
    let hash3 = root_utils::fast_hash("world");
    assert_eq!(hash1, hash2);
    assert_ne!(hash1, hash3);

    let data = b"Hello, Albedo Browser!";
    let dump = root_utils::hexdump(data, 10);
    assert!(dump.contains("48 65 6C 6C 6F"));
    assert!(dump.contains("omitidos"));

    let sanitized = root_utils::sanitize_ascii("Hello\x00\x07World\n!");
    assert_eq!(sanitized, "Hello  World\n!");
}
