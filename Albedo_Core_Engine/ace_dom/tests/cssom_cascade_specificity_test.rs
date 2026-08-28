use ace_dom::cssom::{CSSStyleSheet, StyleResolver};
use ace_dom::query::Specificity;
use ace_dom::{parse_html, ElementBindings};

#[test]
fn test_specificity_ordering_and_cascading() {
    // 1. Testa cálculo de especificidade
    let spec_id = Specificity::new(1, 0, 0); // #header
    let spec_class = Specificity::new(0, 1, 0); // .btn
    let spec_tag = Specificity::new(0, 0, 1); // div
    let spec_class_tag = Specificity::new(0, 1, 1); // div.btn

    assert!(spec_id > spec_class_tag);
    assert!(spec_class_tag > spec_class);
    assert!(spec_class > spec_tag);
    assert_eq!(spec_tag, Specificity::new(0, 0, 1));

    // 2. Cria documento de teste
    let html = "<div id=\"main\" class=\"container active\"><p id=\"target\" class=\"text highlight\" style=\"color: purple;\">Hello World</p></div>";
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    // 3. Monta folha de estilos com regras de diferentes especificidades usando CSSStyleSheet::parse
    let css = r#"
        p { color: black; font-size: 14px; }
        .text { color: blue; line-height: 1.5; }
        #target { color: red; }
        p.text { margin: 10px !important; color: green !important; }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // 4. Resolve o estilo computado
    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[sheet]);

    // !important ganha de estilo inline normal
    assert_eq!(computed.get_property_value("color"), Some("green"));
    assert_eq!(computed.get_property_value("margin"), Some("10px"));
    assert_eq!(computed.get_property_value("font-size"), Some("14px"));
    assert_eq!(computed.get_property_value("line-height"), Some("1.5"));
}

#[test]
fn test_element_matches_and_closest() {
    let html = "<div id=\"wrapper\" class=\"site-container\"><header class=\"header\"><nav class=\"nav-bar\"><a id=\"home-link\" class=\"nav-link active\" href=\"/home\">Home</a></nav></header></div>";
    let doc = parse_html(html);
    let link_id = doc.get_element_by_id("home-link").expect("home-link");
    let wrapper_id = doc.get_element_by_id("wrapper").expect("wrapper");

    // Testes de matches
    assert!(doc.element_matches(link_id, "a"));
    assert!(doc.element_matches(link_id, "a.nav-link"));
    assert!(doc.element_matches(link_id, "#home-link"));
    assert!(doc.element_matches(link_id, "nav > a.active"));
    assert!(!doc.element_matches(link_id, "div"));
    assert!(!doc.element_matches(link_id, ".header"));

    // Testes de closest
    assert_eq!(doc.element_closest(link_id, "a"), Some(link_id));
    assert_eq!(doc.element_closest(link_id, "nav"), doc.query_selector("nav"));
    assert_eq!(doc.element_closest(link_id, ".header"), doc.query_selector(".header"));
    assert_eq!(doc.element_closest(link_id, "#wrapper"), Some(wrapper_id));
    assert_eq!(doc.element_closest(link_id, "footer"), None);

    // Testes de WebIDL bindings
    assert!(ElementBindings::matches(&doc, link_id, "a.active"));
    assert_eq!(
        ElementBindings::closest(&doc, link_id, "#wrapper"),
        Some(wrapper_id)
    );
}
