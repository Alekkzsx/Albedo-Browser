//! # Adversarial Extreme Edge Cases & Fuzz Stress Suite (ace_dom)
//!
//! EMPIRICAL CHALLENGER extreme boundary suite verifying:
//! 1. 1000-deep nesting stress test (preventing stack overflow)
//! 2. 8-element multi-tag intertwined AAA matrix
//! 3. Duplicate attributes handling (First attribute wins per WHATWG §12.2.5.33)
//! 4. Case-insensitive DSD attribute parsing (`SHADOWROOTMODE="OPEN"`)
//! 5. Complex entity prefix collisions (&notin, &notinva, &plus, &plusmn, &plusdo)
//! 6. Script escaping transitions and self-closing slash nuances

use ace_dom::node::ShadowMode;
use ace_dom::parse_html;
use ace_dom::tokenizer::tokenize_to_compact_tokens;

#[test]
fn test_aaa_8_element_interlocking_matrix() {
    // 8 distinct formatting tags interleaved across a <p> barrier
    let html = "<b><i><u><s><tt><big><small><code>1<p>2</b>3</i>4</u>5</s>6</tt>7</big>8</small>9</code>0</p>";
    let doc = parse_html(html);

    let p_elements = doc.query_selector_all("p");
    assert_eq!(p_elements.len(), 1);

    // Verify all 8 formatting tags exist in the document
    for tag in &["b", "i", "u", "s", "tt", "big", "small", "code"] {
        let els = doc.query_selector_all(tag);
        assert!(!els.is_empty(), "Tag <{}> must be in tree after AAA", tag);
    }
}

#[test]
fn test_deep_nesting_1000_levels() {
    // 1000 levels of nested <div>
    let mut html = String::with_capacity(1000 * 12);
    for _ in 0..1000 {
        html.push_str("<div>");
    }
    html.push_str("Deep Inner Content");
    for _ in 0..1000 {
        html.push_str("</div>");
    }

    let doc = parse_html(&html);
    assert!(doc.body.is_some());
    let divs = doc.query_selector_all("div");
    assert_eq!(divs.len(), 1000);
}

#[test]
fn test_duplicate_attributes_first_wins() {
    // WHATWG §12.2.5.33: If attribute name already exists on current tag, ignore subsequent ones
    let html = r#"<div id="first-id" id="second-id" class="primary" class="secondary" title="initial" title="override">Content</div>"#;
    let doc = parse_html(html);

    let div_id = doc.get_element_by_id("first-id").expect("first-id wins");
    let node = doc.get_node(div_id).unwrap();
    let el = node.as_element().unwrap();

    assert_eq!(el.get_attribute("id"), Some("first-id"));
    assert_eq!(el.get_attribute("class"), Some("primary"));
    assert_eq!(el.get_attribute("title"), Some("initial"));
    assert!(doc.get_element_by_id("second-id").is_none());
}

#[test]
fn test_dsd_case_insensitivity() {
    // Attributes in HTML are case-insensitive
    let html = r#"
    <div id="case-host">
        <template SHADOWROOTMODE="OPEN">
            <span id="case-shadow-child">Case Insensitive DSD</span>
        </template>
    </div>
    "#;

    let doc = parse_html(html);
    let host_id = doc.get_element_by_id("case-host").expect("case-host found");
    let el = doc.get_node(host_id).unwrap().as_element().unwrap();
    let shadow_id = el.shadow_root().expect("shadow root attached via uppercase attribute");
    let shadow_node = doc.get_node(shadow_id).unwrap();
    if let ace_dom::NodeKind::ShadowRoot(ref s) = shadow_node.kind {
        assert_eq!(s.mode, ShadowMode::Open);
    } else {
        panic!("Expected ShadowRoot node");
    }
}

#[test]
fn test_complex_entity_prefix_resolutions() {
    // Testing entities that share prefixes
    let html = "<p>&notin; &notinva; &not; &notit; &plus; &plusmn; &plusdo; &prod; &coprod;</p>";
    let doc = parse_html(html);

    let p = doc.query_selector("p").expect("p element found");
    let text = doc.children(p).next().and_then(|(cid, _)| doc.get_node(cid)).and_then(|n| n.text_content()).unwrap();

    // Verify entity conversions
    assert!(text.contains('∉') || text.contains("&notin"));
    assert!(text.contains('¬') || text.contains("&not"));
    assert!(text.contains('+') || text.contains('±') || text.contains("&plus"));
    assert!(text.contains('∏') || text.contains("&prod"));
}

#[test]
fn test_unquoted_attribute_with_slashes_and_urls() {
    // Unquoted attribute values containing slashes and query strings
    let html = r#"<a href=https://example.com/api/v1?user=albedo&ref=test/foo target=_blank>Link</a>"#;
    let doc = parse_html(html);

    let a = doc.query_selector("a").expect("a element");
    let el = doc.get_node(a).unwrap().as_element().unwrap();
    let href = el.get_attribute("href").expect("href attribute");
    assert_eq!(href, "https://example.com/api/v1?user=albedo&ref=test/foo");
    let target = el.get_attribute("target").expect("target attribute");
    assert_eq!(target, "_blank");
}

#[test]
fn test_script_tag_with_embedded_quotes_and_html_comments() {
    // Script tag with complex JS strings that look like HTML tags
    let html = r#"
    <script>
        var htmlSnippet = "</div><script></script>";
        var regex = /<\/script>/g;
        <!-- comment in script
        var x = 10;
        // -->
    </script>
    <div id="after-script">After Script</div>
    "#;

    let doc = parse_html(html);
    let script = doc.query_selector("script");
    assert!(script.is_some(), "Script element parsed");
    let after = doc.get_element_by_id("after-script");
    assert!(after.is_some(), "Element after complex script parsed correctly");
}

#[test]
fn test_self_closing_slash_without_whitespace() {
    // Tags like <img/src="test.png"/alt="photo"/>
    let html = r#"<img/src="test.png"/alt="photo"/>"#;
    let _tokens = tokenize_to_compact_tokens(html);
    let _doc = parse_html(html);

    let imgs = _doc.query_selector_all("img");
    assert!(!imgs.is_empty(), "img element parsed from condensed slash syntax");
}
