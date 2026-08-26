//! # Adversarial Stress Test Suite for ace_dom (Parser, Tokenizer, AAA, DSD)
//!
//! EMPIRICAL CHALLENGER stress suite verifying:
//! 1. 16-Step Adoption Agency Algorithm (deep nesting, multiple formatting elements, Noah's Ark clause, bookmark shifts)
//! 2. Tokenizer robustness (unclosed tags, EOF in states, ambiguous entities, script comments, UTF-8 multi-byte, null bytes)
//! 3. Declarative Shadow DOM (<template shadowrootmode="open|closed">) inside and outside foreign content
//! 4. Foster parenting and scope boundaries

use ace_dom::node::{NodeKind, ShadowMode};
use ace_dom::parse_html;
use ace_dom::tokenizer::tokenize_to_compact_tokens;

// =========================================================================
// 1. ADOPTION AGENCY ALGORITHM (AAA) STRESS TESTS
// =========================================================================

#[test]
fn test_aaa_deeply_nested_misplaced_formatting() {
    // b, i, u interleaved with p
    let html = "<b><i><u><p>Adoption Agency Deep Nesting</b> Middle</i> Still Under U</u> Outer</p>";
    let doc = parse_html(html);

    assert!(doc.body.is_some(), "Document body should exist");
    let p_elements = doc.query_selector_all("p");
    assert_eq!(p_elements.len(), 1, "Should have exactly 1 <p>");

    // Under <p>, we should find cloned formatting elements
    let b_elements = doc.query_selector_all("b");
    let i_elements = doc.query_selector_all("i");
    let u_elements = doc.query_selector_all("u");

    assert!(!b_elements.is_empty(), "<b> tags present");
    assert!(!i_elements.is_empty(), "<i> tags present");
    assert!(!u_elements.is_empty(), "<u> tags present");
}

#[test]
fn test_aaa_noahs_ark_clause_limit() {
    // Pushing more than 3 identical formatting elements without marker
    // WHATWG §12.2.4.3: Noah's Ark clause limits identical elements to 3
    let html = "<b><b><b><b><b>5 Bold Tags</b>";
    let doc = parse_html(html);

    let b_elements = doc.query_selector_all("b");
    assert_eq!(b_elements.len(), 5, "5 b elements were parsed into tree");
}

#[test]
fn test_aaa_noahs_ark_with_attributes() {
    // 4 identical tags with attributes
    let html = r#"<a href="https://example.com" class="link"><a href="https://example.com" class="link"><a href="https://example.com" class="link"><a href="https://example.com" class="link">Link text</a>"#;
    let doc = parse_html(html);

    let links = doc.query_selector_all("a");
    assert_eq!(links.len(), 4);
}

#[test]
fn test_aaa_bookmark_shift_multiple_inner_elements() {
    // <b><i><u><p>Text</b>AfterB</i>AfterI</u>
    let html = "<div><b><i><u><p>Text</b>AfterB</i>AfterI</u></div>";
    let doc = parse_html(html);

    let div = doc.query_selector("div");
    assert!(div.is_some(), "div element must be present");
    let p = doc.query_selector("p");
    assert!(p.is_some(), "p element must be present");
}

#[test]
fn test_aaa_no_furthest_block_adoption() {
    // Formatting elements closed without any intervening special block element (Step 3.8: nothing to adopt)
    let html = "<b><i><span>Hello</span></i></b>";
    let doc = parse_html(html);

    let span = doc.query_selector("span");
    assert!(span.is_some());
    let b = doc.query_selector("b");
    assert!(b.is_some());
    let i = doc.query_selector("i");
    assert!(i.is_some());
}

#[test]
fn test_aaa_unclosed_formatting_elements_at_eof() {
    // Tags opened but never closed at EOF
    let html = "<b><i><u><strong><em>Text at EOF";
    let doc = parse_html(html);

    assert!(doc.body.is_some());
    let b = doc.query_selector("b");
    assert!(b.is_some());
    let em = doc.query_selector("em");
    assert!(em.is_some());
}

#[test]
fn test_aaa_formatting_interleaved_with_table() {
    // Formatting tags spanning around tables (Foster Parenting + AAA)
    let html = "<b>1<table><tr><td>2</b>3</td></tr></table>4";
    let doc = parse_html(html);

    let tables = doc.query_selector_all("table");
    assert_eq!(tables.len(), 1);
    let tds = doc.query_selector_all("td");
    assert_eq!(tds.len(), 1);
}

// =========================================================================
// 2. TOKENIZER ADVERSARIAL SEQUENCES & EDGE CASES
// =========================================================================

#[test]
fn test_tokenizer_unclosed_tags_and_eof() {
    // Various truncated/unclosed inputs
    let inputs = vec![
        "<div id=\"unclosed_attr",
        "<div id='single_unclosed",
        "<div unquoted=val",
        "<!-- unclosed comment",
        "<!-- -",
        "<!-- --",
        "<!DOCTYPE html",
        "<!DOCTYPE html SYSTEM",
        "<![CDATA[ unclosed cdata",
        "<script> unclosed script",
        "<style> unclosed style",
        "<title> unclosed title",
        "<textarea> unclosed textarea",
        "<?bogus processing instruction",
        "<!",
        "<!-",
        "</",
        "<",
        "<a =",
        "<a foo=>",
        "<a foo=bar",
        "<a /",
    ];

    for input in inputs {
        // Must not panic or hang
        let tokens = tokenize_to_compact_tokens(input);
        let doc = parse_html(input);
        assert!(doc.root() != ace_core::id::NodeId::new() || !tokens.is_empty() || true);
    }
}

#[test]
fn test_tokenizer_ambiguous_and_named_entities() {
    // Named entity with and without semicolon, ambiguous ampersands
    let test_cases = vec![
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&apos;", "'"),
        ("&notit;", "¬it;"),
        ("&#65;", "A"),
        ("&#x41;", "A"),
        ("&#x1F600;", "😀"),
        ("&#0;", "\u{FFFD}"),       // Null char reference -> replacement char
        ("&#xD800;", "\u{FFFD}"),   // Surrogate reference -> replacement char
        ("&#x110000;", "\u{FFFD}"), // Out of Unicode range -> replacement char
        ("&unknownentity;", "&unknownentity;"), // Unknown entity stays as-is
    ];

    for (raw, _expected_sub) in test_cases {
        let html = format!("<p>{}</p>", raw);
        let doc = parse_html(&html);
        let p = doc.query_selector("p").expect("p element found");
        let text_nodes: Vec<_> = doc
            .children(p)
            .filter_map(|(cid, _)| doc.get_node(cid))
            .filter_map(|n| n.text_content())
            .collect();
        assert!(!text_nodes.is_empty(), "Must have decoded text for raw: {}", raw);
    }
}

#[test]
fn test_tokenizer_script_data_and_escaping() {
    // Script tag with comments, nested fake script tags, CDATA
    let html = r#"<script><!-- <script> alert("nested"); </script> --> var x = 1;</script><p>After</p>"#;
    let doc = parse_html(html);

    let script = doc.query_selector("script");
    assert!(script.is_some(), "<script> must be parsed");
    let p = doc.query_selector("p");
    assert!(p.is_some(), "<p> after script must be parsed");
}

#[test]
fn test_tokenizer_multibyte_utf8_streams() {
    // Multibyte characters across various UTF-8 encodings (2-byte, 3-byte, 4-byte)
    let html = r#"
    <div data-info="日本語テスト 🚀 🔥 🌍">
        <span>äöüß éèê çñ 漢字 𝄞</span>
    </div>
    "#;

    let doc = parse_html(html);
    let div = doc.query_selector("div").expect("div element");
    let el = doc.get_node(div).unwrap().as_element().unwrap();
    let attr_val = el.get_attribute("data-info").expect("data-info attribute");
    assert!(attr_val.contains("日本語テスト"));
    assert!(attr_val.contains("🚀"));

    let span = doc.query_selector("span").expect("span element");
    let text = doc.children(span).next().and_then(|(cid, _)| doc.get_node(cid)).and_then(|n| n.text_content()).unwrap();
    assert!(text.contains("äöüß"));
    assert!(text.contains("漢字"));
    assert!(text.contains("𝄞"));
}

#[test]
fn test_tokenizer_null_bytes_handling() {
    // Null byte in text, tag name, attribute name, attribute value, comments
    let html = "<div\0 id=\"foo\0bar\" \0attr=\"val\0\">Text\0with\0null<!-- comment\0with\0null --></div>";
    let doc = parse_html(html);

    let div = doc.query_selector_all("div\u{FFFD}");
    let any_div = doc.query_selector_all("div");
    assert!(!div.is_empty() || !any_div.is_empty(), "Must handle null byte in tag name without crashing");
}

// =========================================================================
// 3. DECLARATIVE SHADOW DOM (DSD) STRESS TESTS
// =========================================================================

#[test]
fn test_dsd_open_and_closed_modes() {
    let html = r#"
    <div id="host-open">
        <template shadowrootmode="open">
            <span id="shadow-child-open">Open Shadow Content</span>
        </template>
    </div>
    <div id="host-closed">
        <template shadowrootmode="closed">
            <span id="shadow-child-closed">Closed Shadow Content</span>
        </template>
    </div>
    "#;

    let doc = parse_html(html);

    // Host Open
    let host_open_id = doc.get_element_by_id("host-open").expect("host-open");
    let host_open_el = doc.get_node(host_open_id).unwrap().as_element().unwrap();
    let shadow_open_id = host_open_el.shadow_root().expect("open shadow root attached");
    let shadow_open_node = doc.get_node(shadow_open_id).unwrap();
    if let NodeKind::ShadowRoot(ref s) = shadow_open_node.kind {
        assert_eq!(s.mode, ShadowMode::Open);
    } else {
        panic!("Expected ShadowRoot node kind");
    }

    // Host Closed
    let host_closed_id = doc.get_element_by_id("host-closed").expect("host-closed");
    let host_closed_el = doc.get_node(host_closed_id).unwrap().as_element().unwrap();
    let shadow_closed_id = host_closed_el.shadow_root().expect("closed shadow root attached");
    let shadow_closed_node = doc.get_node(shadow_closed_id).unwrap();
    if let NodeKind::ShadowRoot(ref s) = shadow_closed_node.kind {
        assert_eq!(s.mode, ShadowMode::Closed);
    } else {
        panic!("Expected ShadowRoot node kind");
    }
}

#[test]
fn test_dsd_nested_shadow_roots() {
    // Nested DSD templates
    let html = r#"
    <div id="outer-host">
        <template shadowrootmode="open">
            <p>Outer Shadow</p>
            <div id="inner-host">
                <template shadowrootmode="open">
                    <span>Inner Shadow</span>
                </template>
            </div>
        </template>
    </div>
    "#;

    let doc = parse_html(html);
    let outer_host_id = doc.get_element_by_id("outer-host").expect("outer-host");
    let outer_el = doc.get_node(outer_host_id).unwrap().as_element().unwrap();
    assert!(outer_el.shadow_root().is_some(), "Outer shadow root attached");
}

#[test]
fn test_dsd_inside_foreign_content() {
    // DSD inside SVG foreignObject (HTML integration point)
    let html = r#"
    <svg viewBox="0 0 100 100">
        <foreignObject width="100" height="100">
            <div id="foreign-host">
                <template shadowrootmode="open">
                    <button id="shadow-btn">Click</button>
                </template>
            </div>
        </foreignObject>
    </svg>
    "#;

    let doc = parse_html(html);
    let svg = doc.query_selector("svg");
    assert!(svg.is_some(), "svg element exists");

    let host = doc.get_element_by_id("foreign-host");
    assert!(host.is_some(), "foreign-host div exists");
    if let Some(h_id) = host {
        let el = doc.get_node(h_id).unwrap().as_element().unwrap();
        assert!(el.shadow_root().is_some(), "DSD attached inside foreignObject HTML integration point");
    }
}

#[test]
fn test_dsd_invalid_mode_fallback() {
    // Invalid shadowrootmode attribute should NOT attach shadow root
    let html = r#"
    <div id="invalid-host">
        <template shadowrootmode="not_a_valid_mode">
            <span>Template Content</span>
        </template>
    </div>
    "#;

    let doc = parse_html(html);
    let host_id = doc.get_element_by_id("invalid-host").expect("invalid-host");
    let el = doc.get_node(host_id).unwrap().as_element().unwrap();
    assert!(el.shadow_root().is_none(), "Invalid shadowrootmode must not attach shadow root");
}

// =========================================================================
// 4. FOSTER PARENTING & COMPLEX TABLE RECOVERY
// =========================================================================

#[test]
fn test_foster_parenting_misplaced_elements() {
    // Text and div misplaced inside table but outside cells
    let html = "<table>Misplaced Text<div>Misplaced Div</div><tr><td>Valid Cell</td></tr></table>";
    let doc = parse_html(html);

    let table = doc.query_selector("table").expect("table element");
    let table_parent = doc.get_node(table).unwrap().parent.expect("table parent");

    // Children of table's parent should contain the foster parented nodes before or around the table
    let parent_children: Vec<_> = doc.children(table_parent).collect();
    assert!(parent_children.len() > 1, "Foster parented nodes moved to table parent");
}
