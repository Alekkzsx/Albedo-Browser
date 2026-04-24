use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::{Duration, Instant};

use albedo::ace::html::{parse_document, parse_document_with_errors, StreamingHtmlParser};

fn generate_html(target_bytes: usize) -> String {
    let mut html = String::from("<!DOCTYPE html><html><head><title>v</title></head><body>");

    let mut i = 0usize;
    while html.len() < target_bytes.saturating_sub(32) {
        html.push_str("<div class=\"item\"><span>content ");
        html.push_str(&i.to_string());
        html.push_str("</span></div>");
        i += 1;
    }

    html.push_str("</body></html>");
    html
}

#[test]
fn validation_reports_observed_throughput() {
    let html = generate_html(1_200_000); // ~1.2 MB
    let iterations = 10usize;

    for _ in 0..2 {
        let _ = parse_document(&html);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = parse_document(&html);
    }
    let elapsed = start.elapsed();

    let avg_secs = elapsed.as_secs_f64() / iterations as f64;
    let mb = html.len() as f64 / 1_000_000.0;
    let throughput_mbps = mb / avg_secs;

    println!(
        "ACE throughput observed: {:.2} MB/s (doc={} bytes, avg={:.3}ms)",
        throughput_mbps,
        html.len(),
        avg_secs * 1000.0
    );

    assert!(throughput_mbps > 0.0);
}

#[test]
fn validation_streaming_p99_latency_16kb() {
    let html = generate_html(2 * 1024 * 1024); // >= 128 chunks of 16 KB
    let mut parser = StreamingHtmlParser::new();

    let chunk_size = 16 * 1024;
    for chunk in html.as_bytes().chunks(chunk_size) {
        // Input is ASCII-only from generator.
        let chunk_str = std::str::from_utf8(chunk).expect("chunk should be valid UTF-8");
        let _ = parser.feed(chunk_str);
    }

    let _ = parser.end();
    let p50 = parser.p50_latency();
    let p99 = parser.p99_latency();

    println!("Streaming latency observed: p50={:?}, p99={:?}", p50, p99);

    // Task target: p99 < 1ms for 16KB chunks.
    assert!(
        p99 < Duration::from_millis(1),
        "p99 latency exceeded 1ms: {:?}",
        p99
    );
}

#[test]
fn validation_fuzz_smoke_no_panics() {
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let iterations = 2_000usize;

    for _ in 0..iterations {
        // xorshift64*
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;

        let len = (state as usize % 256) + 1;
        let mut bytes = vec![0u8; len];

        for b in &mut bytes {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *b = (state & 0xFF) as u8;
        }

        let input = String::from_utf8_lossy(&bytes).to_string();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = parse_document_with_errors(&input);
        }));

        assert!(result.is_ok(), "panic detected on fuzz input");
    }
}
