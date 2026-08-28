use ace_dom::cssom::{CSSStyleSheet, StyleResolver};
use ace_dom::parse_html;

#[test]
fn test_automatic_inheritance_deep_tree() {
    let html = r#"
        <div id="l1" style="color: #123456; font-size: 24px; visibility: hidden; line-height: 1.8; letter-spacing: 2px;">
            <div id="l2">
                <div id="l3">
                    <span id="leaf">Deep Text</span>
                </div>
            </div>
        </div>
    "#;
    let doc = parse_html(html);
    let leaf_id = doc.get_element_by_id("leaf").expect("leaf element");

    let computed = StyleResolver::resolve_element_style(&doc, leaf_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("#123456"));
    assert_eq!(computed.get_property_value("font-size"), Some("24px"));
    assert_eq!(computed.get_property_value("visibility"), Some("hidden"));
    assert_eq!(computed.get_property_value("line-height"), Some("1.8"));
    assert_eq!(computed.get_property_value("letter-spacing"), Some("2px"));
}

#[test]
fn test_non_inheritable_properties_do_not_leak() {
    let html = r#"
        <div id="parent" style="margin: 40px; padding: 20px; background-color: yellow; display: flex; opacity: 0.5;">
            <div id="child">Child Element</div>
        </div>
    "#;
    let doc = parse_html(html);
    let child_id = doc.get_element_by_id("child").expect("child element");

    let computed = StyleResolver::resolve_element_style(&doc, child_id, &[]);
    assert_eq!(computed.get_property_value("margin"), None);
    assert_eq!(computed.get_property_value("padding"), None);
    assert_eq!(computed.get_property_value("background-color"), None);
    assert_eq!(computed.get_property_value("display"), None);
    assert_eq!(computed.get_property_value("opacity"), None);
}

#[test]
fn test_keyword_inherit_explicit() {
    let html = r#"
        <div id="parent" style="border: 2px solid red; background-color: blue;">
            <div id="child" style="border: inherit; background-color: inherit;">Child</div>
        </div>
    "#;
    let doc = parse_html(html);
    let child_id = doc.get_element_by_id("child").expect("child element");

    let computed = StyleResolver::resolve_element_style(&doc, child_id, &[]);
    assert_eq!(computed.get_property_value("border"), Some("2px solid red"));
    assert_eq!(computed.get_property_value("background-color"), Some("blue"));
}

#[test]
fn test_keyword_initial_resets_inheritance() {
    let html = r#"
        <div id="parent" style="color: red; font-size: 32px; visibility: hidden;">
            <div id="child" style="color: initial; font-size: initial; visibility: initial;">Child</div>
        </div>
    "#;
    let doc = parse_html(html);
    let child_id = doc.get_element_by_id("child").expect("child element");

    let computed = StyleResolver::resolve_element_style(&doc, child_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("black"));
    assert_eq!(computed.get_property_value("font-size"), Some("16px"));
    assert_eq!(computed.get_property_value("visibility"), Some("visible"));
}

#[test]
fn test_keyword_unset_dispatch() {
    // unset em propriedade herdável (color) -> age como inherit (pega do pai)
    // unset em propriedade não-herdável (display, margin) -> age como initial ('inline', '0px')
    let html = r#"
        <div id="parent" style="color: purple; display: flex; margin: 30px;">
            <div id="child" style="color: unset; display: unset; margin: unset;">Child</div>
        </div>
    "#;
    let doc = parse_html(html);
    let child_id = doc.get_element_by_id("child").expect("child element");

    let computed = StyleResolver::resolve_element_style(&doc, child_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("purple"));
    assert_eq!(computed.get_property_value("display"), Some("inline"));
    assert_eq!(computed.get_property_value("margin"), Some("0px"));
}

#[test]
fn test_root_element_keyword_defaults() {
    let html = r#"<html id="root" style="color: inherit; display: inherit; margin: inherit;"></html>"#;
    let doc = parse_html(html);
    let root_id = doc.get_element_by_id("root").expect("root element");

    let computed = StyleResolver::resolve_element_style(&doc, root_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("black"));
    assert_eq!(computed.get_property_value("display"), Some("inline"));
    assert_eq!(computed.get_property_value("margin"), Some("0px"));
}

#[test]
fn test_visibility_override_in_child() {
    let html = r#"
        <div id="parent" style="visibility: hidden;">
            <div id="child" style="visibility: visible;">Child</div>
        </div>
    "#;
    let doc = parse_html(html);
    let child_id = doc.get_element_by_id("child").expect("child element");

    let computed = StyleResolver::resolve_element_style(&doc, child_id, &[]);
    assert_eq!(computed.get_property_value("visibility"), Some("visible"));
}

#[test]
fn test_stylesheet_cascading_with_inheritance() {
    let html = r#"
        <div id="container" class="theme-dark">
            <p id="para">Paragraph text</p>
        </div>
    "#;
    let doc = parse_html(html);
    let para_id = doc.get_element_by_id("para").expect("para element");

    let css = r#"
        .theme-dark {
            color: #eeeeee;
            cursor: pointer;
            direction: rtl;
        }
        p {
            font-size: 18px;
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);
    let computed = StyleResolver::resolve_element_style(&doc, para_id, &[sheet]);

    assert_eq!(computed.get_property_value("color"), Some("#eeeeee"));
    assert_eq!(computed.get_property_value("cursor"), Some("pointer"));
    assert_eq!(computed.get_property_value("direction"), Some("rtl"));
    assert_eq!(computed.get_property_value("font-size"), Some("18px"));
}
