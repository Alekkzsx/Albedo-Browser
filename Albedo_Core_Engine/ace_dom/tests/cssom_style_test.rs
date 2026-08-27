use ace_dom::cssom::{CSSStyleDeclaration, CSSStyleSheet, StyleResolver};
use ace_dom::parse_html;

#[test]
fn test_css_style_declaration_parsing_and_mutation() {
    let mut decl = CSSStyleDeclaration::parse("color: #ff0000; font-size: 14px; margin-top: 10px !important;");

    assert_eq!(decl.len(), 3);
    assert_eq!(decl.get_property_value("color"), Some("#ff0000"));
    assert_eq!(decl.get_property_value("font-size"), Some("14px"));
    assert_eq!(decl.get_property_value("margin-top"), Some("10px"));
    assert_eq!(decl.get_property_priority("margin-top"), "important");
    assert_eq!(decl.get_property_priority("color"), "");

    // Modifica propriedade
    decl.set_property("color", "blue", true);
    assert_eq!(decl.get_property_value("color"), Some("blue"));
    assert_eq!(decl.get_property_priority("color"), "important");

    // Remove propriedade
    let removed = decl.remove_property("font-size");
    assert_eq!(removed.as_deref(), Some("14px"));
    assert_eq!(decl.get_property_value("font-size"), None);
}

#[test]
fn test_element_inline_style_auto_sync() {
    let mut doc = parse_html(r#"<div id="box" style="display: flex; align-items: center;"></div>"#);
    let box_id = doc.get_element_by_id("box").unwrap();

    let node = doc.get_node(box_id).unwrap();
    let el = node.as_element().unwrap();

    assert!(el.style().is_some());
    let style = el.style().unwrap();
    assert_eq!(style.get_property_value("display"), Some("flex"));
    assert_eq!(style.get_property_value("align-items"), Some("center"));

    // Modifica via set_attribute
    if let Some(n) = doc.get_node_mut(box_id) {
        if let Some(e) = n.as_element_mut() {
            e.set_attribute("style", "display: block; opacity: 0.8;");
            assert_eq!(e.style().unwrap().get_property_value("display"), Some("block"));
            assert_eq!(e.style().unwrap().get_property_value("opacity"), Some("0.8"));
        }
    }
}

#[test]
fn test_css_cascading_and_computed_style_resolution() {
    let doc = parse_html(r#"<div id="target" class="btn primary" style="padding: 20px; color: red;">Hello</div>"#);
    let target_id = doc.get_element_by_id("target").unwrap();

    let css = r#"
        .btn {
            background-color: grey;
            padding: 10px;
            color: black;
        }
        .primary {
            background-color: blue !important;
        }
    "#;

    let sheet = CSSStyleSheet::parse(css);
    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[sheet]);

    // inline style sobrescreve stylesheet normal
    assert_eq!(computed.get_property_value("color"), Some("red"));
    assert_eq!(computed.get_property_value("padding"), Some("20px"));

    // !important da stylesheet sobrescreve background
    assert_eq!(computed.get_property_value("background-color"), Some("blue"));
}
