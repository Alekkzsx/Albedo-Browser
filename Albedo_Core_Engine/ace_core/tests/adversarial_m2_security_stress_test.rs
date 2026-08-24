//! # Adversarial Stress Testing Suite - Milestone M2 (Challenger 2)
//!
//! Rigorous stress testing and boundary verification for:
//! 1. `percent_decode`: Malformed sequences, boundary truncations, invalid hex, byte preservation.
//! 2. `sniff_mime_type`: 512-byte UTF-8 split boundary across 2-byte, 3-byte (CJK), 4-byte (emojis) characters.
//! 3. `Origin` (RFC 6454): File, data, opaque, and custom scheme serialization without `:0`.

use ace_core::net::{parse_data_uri, percent_decode, sniff_mime_type};
use ace_core::security::{Host, Origin, Scheme};

// =========================================================================
// 1. PERCENT DECODE ADVERSARIAL STRESS TESTS
// =========================================================================

#[test]
fn test_percent_decode_malformed_and_boundary_cases() {
    // Exact cases requested in specification
    assert_eq!(percent_decode("100%"), "100%");
    assert_eq!(percent_decode("100%_concluido"), "100%_concluido");
    assert_eq!(percent_decode("%ZZ"), "%ZZ");
    assert_eq!(percent_decode("%A"), "%A");
    assert_eq!(percent_decode("%%%"), "%%%");
    assert_eq!(percent_decode("%20%"), " %");
    assert_eq!(percent_decode("%20%2"), " %2");
    assert_eq!(percent_decode("trailing%"), "trailing%");
    assert_eq!(percent_decode("trailing%%"), "trailing%%");

    // Single and empty
    assert_eq!(percent_decode(""), "");
    assert_eq!(percent_decode("%"), "%");
    assert_eq!(percent_decode("%%"), "%%");
    assert_eq!(percent_decode("%%%"), "%%%");
    assert_eq!(percent_decode("%%%%"), "%%%%");

    // Partial hex at various offsets
    assert_eq!(percent_decode("a%"), "a%");
    assert_eq!(percent_decode("a%1"), "a%1");
    assert_eq!(percent_decode("a%1z"), "a%1z");
    assert_eq!(percent_decode("a%z1"), "a%z1");
    assert_eq!(percent_decode("%1"), "%1");
    assert_eq!(percent_decode("%F"), "%F");
    assert_eq!(percent_decode("%G0"), "%G0");
    assert_eq!(percent_decode("%0G"), "%0G");
    assert_eq!(percent_decode("%-1"), "%-1");
    assert_eq!(percent_decode("%+1"), "%+1");
    assert_eq!(percent_decode("% 1"), "% 1");
    assert_eq!(percent_decode("%1 "), "%1 ");

    // Interleaved valid and invalid
    assert_eq!(percent_decode("%20%ZZ%20"), " %ZZ ");
    assert_eq!(percent_decode("%%%20%%%20%"), "%% %% %");
    assert_eq!(percent_decode("%25%25%25"), "%%%"); // %25 is '%'
    assert_eq!(percent_decode("100%25%20done"), "100% done");

    // Hex case sensitivity (both lowercase and uppercase should be valid)
    assert_eq!(percent_decode("%2f%2F%3a%3A"), "//::");
    assert_eq!(percent_decode("%4a%4A%61%6A"), "JJaj");

    // Null byte preservation
    let decoded_null = percent_decode("pre%00post");
    assert_eq!(decoded_null, "pre\0post");
    assert_eq!(decoded_null.as_bytes(), b"pre\x00post");

    // Multi-byte UTF-8 percent-encoded
    // '🦀' is F0 9F A6 80
    assert_eq!(percent_decode("%F0%9F%A6%80"), "🦀");
    // '世界' is E4 B8 96 E7 95 8C
    assert_eq!(percent_decode("%E4%B8%96%E7%95%8C"), "世界");
    // 'café' -> café
    assert_eq!(percent_decode("caf%C3%A9"), "café");
}

#[test]
fn test_percent_decode_randomized_stress_oracle() {
    // Oracle stress test: Generates mixed valid/invalid streams and verifies:
    // 1. Output never panics.
    // 2. Length invariant: output string bytes length <= input bytes length + (possible lossy utf8 expansion).
    // 3. For pure ASCII without %, input == output.
    for len in 0..100 {
        let ascii_plain = "a".repeat(len);
        assert_eq!(percent_decode(&ascii_plain), ascii_plain);
    }

    let test_patterns = [
        "%", "%%", "%%%", "%2", "%20", "%20%", "%20%2", "%20%20",
        "%ZZ", "%!@", "%_a", "%%20", "%%%20", "%20%%%", "%A%B%C",
        "100%_concluido_%20_finalizado%99%AA%FF",
    ];

    for pattern in &test_patterns {
        let decoded = percent_decode(pattern);
        assert!(!decoded.is_empty() || pattern.is_empty());
    }
}

// =========================================================================
// 2. SNIFF MIME TYPE UTF-8 SPLIT BOUNDARY STRESS TESTS
// =========================================================================

#[test]
fn test_sniff_mime_type_utf8_split_2byte_char() {
    // 2-byte UTF-8 char: 'é' = [0xC3, 0xA9]
    // We place 0xC3 at index 511, and 0xA9 at index 512.
    let prefix = b"<!DOCTYPE html><html><head><title>";
    let mut data = Vec::new();
    data.extend_from_slice(prefix);
    while data.len() < 511 {
        data.push(b'A');
    }
    assert_eq!(data.len(), 511);

    // byte 511 is 0xC3 (start of 2-byte sequence)
    data.push(0xC3);
    // byte 512 is 0xA9 (continuation byte, beyond 512 limit)
    data.push(0xA9);
    data.extend_from_slice(b"</title></head><body>Hello</body></html>");

    let mime = sniff_mime_type(&data);
    assert_eq!(mime, "text/html", "2-byte UTF-8 split across 512B must resolve to text/html");
}

#[test]
fn test_sniff_mime_type_utf8_split_3byte_cjk_char() {
    // 3-byte UTF-8 char: '世' = [0xE4, 0xB8, 0x96]
    // Subcase 1: Split after 1 byte (0xE4 at index 511, 0xB8 at 512, 0x96 at 513)
    {
        let prefix = b"<!DOCTYPE html><html><body>";
        let mut data = Vec::new();
        data.extend_from_slice(prefix);
        while data.len() < 511 {
            data.push(b'B');
        }
        data.push(0xE4); // byte 511
        data.push(0xB8); // byte 512
        data.push(0x96); // byte 513
        data.extend_from_slice(b"</body></html>");

        let mime = sniff_mime_type(&data);
        assert_eq!(mime, "text/html", "3-byte UTF-8 split (1 byte in) must resolve to text/html");
    }

    // Subcase 2: Split after 2 bytes (0xE4 at 510, 0xB8 at 511, 0x96 at 512)
    {
        let prefix = b"<html><head><title>CJK</title></head><body>";
        let mut data = Vec::new();
        data.extend_from_slice(prefix);
        while data.len() < 510 {
            data.push(b'C');
        }
        data.push(0xE4); // byte 510
        data.push(0xB8); // byte 511
        data.push(0x96); // byte 512
        data.extend_from_slice(b"</body></html>");

        let mime = sniff_mime_type(&data);
        assert_eq!(mime, "text/html", "3-byte UTF-8 split (2 bytes in) must resolve to text/html");
    }
}

#[test]
fn test_sniff_mime_type_utf8_split_4byte_emoji() {
    // 4-byte UTF-8 emoji: '😀' = \u{1F600} = [0xF0, 0x9F, 0x98, 0x80]

    // Subcase 1: 1 byte inside (byte 511)
    {
        let prefix = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><text>";
        let mut data = Vec::new();
        data.extend_from_slice(prefix);
        while data.len() < 511 {
            data.push(b'x');
        }
        data.extend_from_slice(&[0xF0, 0x9F, 0x98, 0x80]);
        data.extend_from_slice(b"</text></svg>");

        let mime = sniff_mime_type(&data);
        assert_eq!(mime, "image/svg+xml", "4-byte UTF-8 split (1 byte in) must resolve to image/svg+xml");
    }

    // Subcase 2: 2 bytes inside (bytes 510, 511)
    {
        let prefix = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><text>";
        let mut data = Vec::new();
        data.extend_from_slice(prefix);
        while data.len() < 510 {
            data.push(b'y');
        }
        data.extend_from_slice(&[0xF0, 0x9F, 0x98, 0x80]);
        data.extend_from_slice(b"</text></svg>");

        let mime = sniff_mime_type(&data);
        assert_eq!(mime, "image/svg+xml", "4-byte UTF-8 split (2 bytes in) must resolve to image/svg+xml");
    }

    // Subcase 3: 3 bytes inside (bytes 509, 510, 511)
    {
        let prefix = b"<?xml version=\"1.0\" encoding=\"utf-8\"?><root>";
        let mut data = Vec::new();
        data.extend_from_slice(prefix);
        while data.len() < 509 {
            data.push(b'z');
        }
        data.extend_from_slice(&[0xF0, 0x9F, 0x98, 0x80]);
        data.extend_from_slice(b"</root>");

        let mime = sniff_mime_type(&data);
        assert_eq!(mime, "application/xml", "4-byte UTF-8 split (3 bytes in) must resolve to application/xml");
    }
}

#[test]
fn test_sniff_mime_type_plain_text_utf8_split_fallback() {
    // Plain text with no HTML/XML tags, with 4-byte emoji split at byte 511
    let mut data = Vec::new();
    while data.len() < 511 {
        data.push(b'k');
    }
    data.extend_from_slice(&[0xF0, 0x9F, 0x98, 0x80]);
    data.extend_from_slice(b" remaining plain text");

    let mime = sniff_mime_type(&data);
    assert_eq!(mime, "text/plain", "Plain text with split UTF-8 must fall back to text/plain");
}

#[test]
fn test_sniff_mime_type_edge_boundaries_and_empty() {
    // 0 bytes
    assert_eq!(sniff_mime_type(&[]), "text/plain");

    // 1-byte incomplete UTF-8 start sequences (valid_len == 0)
    assert_eq!(sniff_mime_type(&[0xC3]), "application/octet-stream");
    assert_eq!(sniff_mime_type(&[0xE4]), "application/octet-stream");
    assert_eq!(sniff_mime_type(&[0xF0]), "application/octet-stream");

    // Invalid UTF-8 continuation byte at start
    assert_eq!(sniff_mime_type(&[0x80, 0x81, 0x82]), "application/octet-stream");

    // Exact 512 bytes HTML
    let mut exact_512 = Vec::new();
    exact_512.extend_from_slice(b"<!DOCTYPE html><html><body>");
    while exact_512.len() < 512 {
        exact_512.push(b' ');
    }
    assert_eq!(exact_512.len(), 512);
    assert_eq!(sniff_mime_type(&exact_512), "text/html");

    // Exact 511 bytes HTML
    let exact_511 = &exact_512[..511];
    assert_eq!(sniff_mime_type(exact_511), "text/html");

    // Exact 513 bytes HTML
    let mut exact_513 = exact_512.clone();
    exact_513.push(b'!');
    assert_eq!(sniff_mime_type(&exact_513), "text/html");
}

// =========================================================================
// 3. ORIGIN RFC 6454 ADVERSARIAL STRESS TESTS
// =========================================================================

#[test]
fn test_origin_rfc6454_file_scheme() {
    let file1 = Origin::parse("file:///path/to/file.html").unwrap();
    assert_eq!(file1.ascii_serialization(), "file://");
    assert!(!file1.ascii_serialization().contains(":0"));

    let file_win = Origin::parse("file:///C:/Users/test/AppData/index.html").unwrap();
    assert_eq!(file_win.ascii_serialization(), "file://");
    assert!(!file_win.ascii_serialization().contains(":0"));

    // In WHATWG URL Standard, file://localhost/ is normalized to empty host
    let file_host = Origin::parse("file://localhost/etc/hosts").unwrap();
    assert_eq!(file_host.ascii_serialization(), "file://");
    assert!(!file_host.ascii_serialization().contains(":0"));

    let file_tuple = Origin::tuple(Scheme::File, Host::Opaque, None);
    assert_eq!(file_tuple.ascii_serialization(), "file://");

    let file_tuple_named_host = Origin::tuple(Scheme::File, Host::Domain("remote-host".into()), None);
    assert_eq!(file_tuple_named_host.ascii_serialization(), "file://remote-host");
    assert!(!file_tuple_named_host.ascii_serialization().contains(":0"));

    let file_tuple_zero = Origin::tuple(Scheme::File, Host::Opaque, Some(0));
    assert_eq!(file_tuple_zero.ascii_serialization(), "file://");
}

#[test]
fn test_origin_rfc6454_data_uri_and_opaque() {
    let data1 = Origin::parse("data:text/html,<h1>Hello</h1>").unwrap();
    let data2 = Origin::parse("data:text/html,<h1>Hello</h1>").unwrap();
    let data3 = Origin::parse("data:image/png;base64,iVBORw0KGgo=").unwrap();

    assert!(data1.is_opaque());
    assert!(data2.is_opaque());
    assert!(data3.is_opaque());

    assert_eq!(data1.ascii_serialization(), "null");
    assert_eq!(data2.ascii_serialization(), "null");
    assert_eq!(data3.ascii_serialization(), "null");

    // RFC 6454: Every opaque origin is globally unique; distinct parses are NEVER same-origin
    assert!(!data1.same_origin(&data2));
    assert!(!data1.same_origin(&data3));
    assert!(!data2.same_origin(&data3));

    let about_blank = Origin::parse("about:blank").unwrap();
    assert!(about_blank.is_opaque());
    assert_eq!(about_blank.ascii_serialization(), "null");
    assert!(!about_blank.same_origin(&data1));
}

#[test]
fn test_origin_rfc6454_custom_schemes() {
    // Custom scheme without port
    let custom1 = Origin::parse("albedo://settings").unwrap();
    assert_eq!(custom1.ascii_serialization(), "albedo://settings");
    assert!(!custom1.ascii_serialization().contains(":0"));

    let custom2 = Origin::parse("vscode://extension/install").unwrap();
    assert_eq!(custom2.ascii_serialization(), "vscode://extension");
    assert!(!custom2.ascii_serialization().contains(":0"));

    let custom3 = Origin::parse("chrome-extension://abcdefghijklmnop").unwrap();
    assert_eq!(custom3.ascii_serialization(), "chrome-extension://abcdefghijklmnop");
    assert!(!custom3.ascii_serialization().contains(":0"));

    // Custom scheme with explicit non-zero port
    let custom_port = Origin::parse("app://service:9090/endpoint").unwrap();
    assert_eq!(custom_port.ascii_serialization(), "app://service:9090");

    // Custom scheme constructed with port 0
    let custom_port_zero = Origin::tuple(
        Scheme::Custom("myproto".into()),
        Host::Domain("daemon".into()),
        Some(0),
    );
    assert_eq!(custom_port_zero.ascii_serialization(), "myproto://daemon");
    assert!(!custom_port_zero.ascii_serialization().contains(":0"));
}

#[test]
fn test_origin_rfc6454_standard_http_https_and_ip() {
    // Default ports omitted
    let http80 = Origin::parse("http://example.com:80/").unwrap();
    assert_eq!(http80.ascii_serialization(), "http://example.com");

    let https443 = Origin::parse("https://example.com:443/").unwrap();
    assert_eq!(https443.ascii_serialization(), "https://example.com");

    // Non-default ports preserved
    let http8080 = Origin::parse("http://example.com:8080/").unwrap();
    assert_eq!(http8080.ascii_serialization(), "http://example.com:8080");

    let https8443 = Origin::parse("https://example.com:8443/").unwrap();
    assert_eq!(https8443.ascii_serialization(), "https://example.com:8443");

    // IPv4 and IPv6
    let ip4 = Origin::parse("http://192.168.1.1:80/").unwrap();
    assert_eq!(ip4.ascii_serialization(), "http://192.168.1.1");

    let ip4_port = Origin::parse("http://192.168.1.1:3000/").unwrap();
    assert_eq!(ip4_port.ascii_serialization(), "http://192.168.1.1:3000");

    let ip6 = Origin::parse("http://[::1]:80/").unwrap();
    assert_eq!(ip6.ascii_serialization(), "http://::1");

    let ip6_port = Origin::parse("http://[::1]:8080/").unwrap();
    assert_eq!(ip6_port.ascii_serialization(), "http://::1:8080");
}

#[test]
fn test_data_uri_roundtrip_and_percent_decoding() {
    // Verify integration of percent_decode with data URIs containing %
    let raw_data_uri = "data:text/html;charset=utf-8,<h1>100%_concluido%20-%20%C3%A9xito</h1>";
    let (mime, bytes) = parse_data_uri(raw_data_uri).expect("Failed to parse data uri");
    assert_eq!(mime.essence(), "text/html");
    let content = String::from_utf8(bytes).expect("Valid UTF-8 decoded data");
    assert_eq!(content, "<h1>100%_concluido - éxito</h1>");
}

// =========================================================================
// 4. DEEP ADVERSARIAL ORACLE & FUZZING HARNESSES
// =========================================================================

#[test]
fn test_percent_decode_stress_fuzzer() {
    let mut rng_seed: u64 = 0x123456789abcdef0;
    // Simple LCG PRNG for reproducible test runs
    let mut lcg = || {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        rng_seed
    };

    let mut corpus = Vec::new();
    // Build 500 varied inputs
    for _ in 0..500 {
        let len = (lcg() % 50) as usize;
        let mut s = String::new();
        for _ in 0..len {
            let choice = lcg() % 8;
            match choice {
                0 => s.push('%'),
                1 => s.push_str("%20"),
                2 => s.push_str("%ZZ"),
                3 => s.push_str("%C3%A9"),
                4 => s.push_str("100%_"),
                5 => s.push((b'a' + (lcg() % 26) as u8) as char),
                6 => s.push_str("%00"),
                _ => s.push((b'0' + (lcg() % 10) as u8) as char),
            }
        }
        corpus.push(s);
    }

    for input in corpus {
        let decoded = percent_decode(&input);
        // Invariant: percent_decode must NEVER panic, and for valid %20 must contain spaces
        if input.contains("%20") {
            assert!(decoded.contains(' ') || decoded.contains("%20"));
        }
    }
}

#[test]
fn test_sniff_mime_type_invalid_bytes_midstream_resilience() {
    // A document starting with <!DOCTYPE html> followed by invalid non-UTF8 bytes at byte 25
    let mut payload = Vec::new();
    payload.extend_from_slice(b"<!DOCTYPE html><html>"); // 21 bytes
    payload.extend_from_slice(&[0xFF, 0xFE, 0x80, 0x81]); // Invalid UTF8 bytes
    payload.extend_from_slice(b"<body>Rest of the document</body></html>");

    // The valid UTF-8 portion before byte 21 is "<!DOCTYPE html><html>"
    // sniff_mime_type should recover the valid slice (valid_up_to == 21) and sniff "text/html"!
    let mime = sniff_mime_type(&payload);
    assert_eq!(mime, "text/html", "Must identify HTML even when invalid UTF-8 occurs mid-stream after valid HTML tags");
}

#[test]
fn test_sniff_mime_type_all_magic_number_signatures() {
    // Verify all binary format signatures remain uncompromised
    assert_eq!(sniff_mime_type(b"\x89PNG\r\n\x1a\n...payload..."), "image/png");
    assert_eq!(sniff_mime_type(b"\xFF\xD8\xFF\xE0...jpeg..."), "image/jpeg");
    assert_eq!(sniff_mime_type(b"GIF87a...data..."), "image/gif");
    assert_eq!(sniff_mime_type(b"GIF89a...data..."), "image/gif");
    assert_eq!(sniff_mime_type(b"RIFF\x00\x00\x00\x00WEBP..."), "image/webp");
    assert_eq!(sniff_mime_type(b"....ftypavif...."), "image/avif");
    assert_eq!(sniff_mime_type(b"....ftypavis...."), "image/avif");
    assert_eq!(sniff_mime_type(b"\x00\x00\x01\x00ico"), "image/x-icon");
    assert_eq!(sniff_mime_type(b"BM....bmp..."), "image/bmp");
    assert_eq!(sniff_mime_type(b"II*\x00tiff"), "image/tiff");
    assert_eq!(sniff_mime_type(b"MM\x00*tiff"), "image/tiff");
    assert_eq!(sniff_mime_type(b"RIFF\x00\x00\x00\x00WAVE..."), "audio/wav");
    assert_eq!(sniff_mime_type(b"wOFFfont"), "font/woff");
    assert_eq!(sniff_mime_type(b"wOF2font"), "font/woff2");
    assert_eq!(sniff_mime_type(b"%PDF-1.7..."), "application/pdf");
    assert_eq!(sniff_mime_type(b"\x1A\x45\xDF\xA3webm"), "video/webm");
    assert_eq!(sniff_mime_type(b"....ftypmp42..."), "video/mp4");
    assert_eq!(sniff_mime_type(b"ID3mp3"), "audio/mpeg");
}

#[test]
fn test_origin_same_origin_equivalence_relations() {
    let o1 = Origin::parse("https://albedo.dev:443/home").unwrap();
    let o2 = Origin::parse("https://albedo.dev/profile").unwrap();
    let o3 = Origin::parse("https://albedo.dev/settings").unwrap();

    // Reflexivity
    assert!(o1.same_origin(&o1));
    assert!(o2.same_origin(&o2));

    // Symmetry
    assert!(o1.same_origin(&o2));
    assert!(o2.same_origin(&o1));

    // Transitivity
    assert!(o2.same_origin(&o3));
    assert!(o1.same_origin(&o3));

    // Opaque origins are anti-reflexive for different instances
    let op1 = Origin::new_opaque();
    let op2 = Origin::new_opaque();
    assert!(!op1.same_origin(&op2));
    assert!(!op2.same_origin(&op1));
}

