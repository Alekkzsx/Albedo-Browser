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

#[test]
fn test_css_declaration_comments_and_quote_aware_important() {
    let css = r#"
        /* Comentário inicial */
        color: /* c1 */ #123456 /* c2 */;
        /* font-size: 999px; */
        font-size: 16px /* c3 */ ! /* c4 */ important /* c5 */;
        content: "hello !important string";
        background-image: url("https://example.com/asset!important.png");
        z-index: 10 ! IMPORTANT;
    "#;

    let decl = CSSStyleDeclaration::parse(css);
    assert_eq!(decl.get_property_value("color"), Some("#123456"));
    assert_eq!(decl.get_property_priority("color"), "");

    assert_eq!(decl.get_property_value("font-size"), Some("16px"));
    assert_eq!(decl.get_property_priority("font-size"), "important");

    assert_eq!(decl.get_property_value("content"), Some("\"hello !important string\""));
    assert_eq!(decl.get_property_priority("content"), "");

    assert_eq!(
        decl.get_property_value("background-image"),
        Some("url(\"https://example.com/asset!important.png\")")
    );
    assert_eq!(decl.get_property_priority("background-image"), "");

    assert_eq!(decl.get_property_value("z-index"), Some("10"));
    assert_eq!(decl.get_property_priority("z-index"), "important");
}

#[test]
fn test_stylesheet_case_insensitive_at_rules() {
    let css = r#"
        @IMPORT url("imported.css");
        @MeDiA screen and (min-width: 300px) {
            .box { color: red; }
        }
        @KEYFRAMES pulse {
            0% { opacity: 0; }
            100% { opacity: 1; }
        }
        @-WEBKIT-KEYFRAMES fade {
            from { opacity: 0; }
            to { opacity: 1; }
        }
    "#;

    let sheet = CSSStyleSheet::parse(css);
    assert_eq!(sheet.rules.len(), 4);

    match &sheet.rules[0] {
        ace_dom::cssom::CSSRule::Import { href } => assert_eq!(href.as_str(), "imported.css"),
        _ => panic!("Expected Import rule"),
    }

    match &sheet.rules[1] {
        ace_dom::cssom::CSSRule::Media { condition, rules } => {
            assert_eq!(condition.as_str(), "screen and (min-width: 300px)");
            assert_eq!(rules.len(), 1);
        }
        _ => panic!("Expected Media rule"),
    }

    match &sheet.rules[2] {
        ace_dom::cssom::CSSRule::Keyframes { name, .. } => {
            assert_eq!(name.as_str(), "pulse");
        }
        _ => panic!("Expected Keyframes rule"),
    }

    match &sheet.rules[3] {
        ace_dom::cssom::CSSRule::Keyframes { name, .. } => {
            assert_eq!(name.as_str(), "fade");
        }
        _ => panic!("Expected Keyframes rule"),
    }
}

#[test]
fn test_compound_selector_matching_and_cascade() {
    let doc = parse_html(r#"
        <div id="main" class="highlight header">Match 1</div>
        <div id="sidebar" class="highlight">No match</div>
        <div id="footer" class="header">No match</div>
    "#);

    let main_id = doc.get_element_by_id("main").unwrap();
    let sidebar_id = doc.get_element_by_id("sidebar").unwrap();
    let footer_id = doc.get_element_by_id("footer").unwrap();

    let css = r#"
        .highlight.header {
            color: purple;
            font-weight: bold;
        }
        #main.highlight {
            border: 1px solid black;
        }
    "#;

    let sheet = CSSStyleSheet::parse(css);
    let computed_main = StyleResolver::resolve_element_style(&doc, main_id, std::slice::from_ref(&sheet));
    let computed_sidebar = StyleResolver::resolve_element_style(&doc, sidebar_id, std::slice::from_ref(&sheet));
    let computed_footer = StyleResolver::resolve_element_style(&doc, footer_id, std::slice::from_ref(&sheet));

    // main matches both compound selectors
    assert_eq!(computed_main.get_property_value("color"), Some("purple"));
    assert_eq!(computed_main.get_property_value("font-weight"), Some("bold"));
    assert_eq!(computed_main.get_property_value("border"), Some("1px solid black"));

    // sidebar and footer do not match .highlight.header
    assert_eq!(computed_sidebar.get_property_value("color"), None);
    assert_eq!(computed_footer.get_property_value("color"), None);
}
