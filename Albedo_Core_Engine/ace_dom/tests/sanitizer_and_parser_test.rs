//! # Bateria de Testes do Sanitizador HTML e DOMParser Multimídia (ace_dom)

use ace_dom::dom_parser::{DOMParser, SupportedType};
use ace_dom::sanitizer::HTMLSanitizer;

#[test]
fn test_html_sanitizer_removes_xss_vectors() {
    let dirty_html = r#"
    <div>
      <h1>Título Seguro</h1>
      <script>alert('XSS Script');</script>
      <a href="javascript:alert('XSS Link')" onclick="badFunc()">Link Malicioso</a>
      <img src="valid.png" onerror="stealCookies()">
      <iframe src="http://evil.com"></iframe>
    </div>
    "#;

    let clean_doc = HTMLSanitizer::parse_html_safe(dirty_html, None);

    // 1. Tags <script> e <iframe> devem ter sido eliminadas
    assert!(clean_doc.query_selector("script").is_none());
    assert!(clean_doc.query_selector("iframe").is_none());

    // 2. O <a> seguro deve existir, mas sem href="javascript:" e sem onclick
    let a_id = clean_doc.query_selector("a").expect("link preservado");
    let a_el = clean_doc.get_node(a_id).unwrap().as_element().unwrap();
    assert_eq!(a_el.get_attribute("href"), None);
    assert_eq!(a_el.get_attribute("onclick"), None);

    // 3. A <img> deve existir, mas sem onerror
    let img_id = clean_doc.query_selector("img").expect("imagem preservada");
    let img_el = clean_doc.get_node(img_id).unwrap().as_element().unwrap();
    assert_eq!(img_el.get_attribute("src"), Some("valid.png"));
    assert_eq!(img_el.get_attribute("onerror"), None);
}

#[test]
fn test_dom_parser_multimedia_dispatch() {
    let parser = DOMParser::new();

    // 1. Parsing HTML
    let html_doc = parser
        .parse_from_string("<div><p>Albedo</p></div>", SupportedType::TextHtml)
        .expect("HTML parsed");
    assert!(html_doc.query_selector("p").is_some());

    // 2. Parsing SVG XML
    let svg_xml = r#"<circle cx="50" cy="50" r="40" stroke="green" fill="yellow" />"#;
    let svg_doc = parser
        .parse_from_string(svg_xml, SupportedType::ImageSvgXml)
        .expect("SVG parsed");
    let svg_root = svg_doc.document_element.expect("svg root");
    assert_eq!(svg_doc.get_node(svg_root).unwrap().tag_name().unwrap().as_str(), "svg");
    assert!(svg_doc.query_selector("circle").is_some());

    // 3. Parsing via string MIME
    let xml_doc = parser
        .parse_from_str("<catalog><book id=\"bk101\"/></catalog>", "application/xml")
        .expect("XML parsed");
    assert!(xml_doc.query_selector("catalog").is_some());
}
