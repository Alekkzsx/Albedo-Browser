use ace_dom::cssom::{CSSStyleSheet, StyleResolver};
use ace_dom::parse_html;

#[test]
fn test_simple_var_substitution() {
    let html = r#"<div id="target" style="--main-color: #ff0000; --top: 10px; --right: 20px; color: var(--main-color); margin: var(--top) var(--right) 0 0;"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("#ff0000"));
    assert_eq!(computed.get_property_value("margin"), Some("10px 20px 0 0"));
}

#[test]
fn test_var_with_fallbacks() {
    let html = r#"
        <div id="target" style="
            color: var(--missing-color, #00ff00);
            font-family: var(--missing-font, Arial, Helvetica, sans-serif);
            content: var(--missing-content, );
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("#00ff00"));
    assert_eq!(
        computed.get_property_value("font-family"),
        Some("Arial, Helvetica, sans-serif")
    );
    assert_eq!(computed.get_property_value("content"), Some(""));
}

#[test]
fn test_nested_var_fallbacks() {
    let html = r#"
        <div id="target" style="
            --accent: orange;
            color: var(--u1, var(--u2, var(--u3, purple)));
            background-color: var(--u1, var(--accent, red));
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("purple"));
    assert_eq!(computed.get_property_value("background-color"), Some("orange"));
}

#[test]
fn test_direct_cycle_detection() {
    // --a: var(--a); direct self-reference cycle
    let html = r#"<div id="target" style="--a: var(--a); color: var(--a);"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    // Cyclic var() causes IACVT -> acts as 'unset' -> resolves to initial value 'black'
    assert_eq!(computed.get_property_value("color"), Some("black"));
}

#[test]
fn test_mutual_cycle_detection() {
    // 2-node cycle: --a -> --b -> --a
    let html = r#"<div id="target" style="--a: var(--b); --b: var(--a); color: var(--a); display: var(--b);"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("black"));
    assert_eq!(computed.get_property_value("display"), Some("inline"));
}

#[test]
fn test_transitive_cycle_detection() {
    // 3-node cycle: --a -> --b -> --c -> --a
    let html = r#"<div id="target" style="--a: var(--b); --b: var(--c); --c: var(--a); color: var(--a);"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("black"));
}

#[test]
fn test_cycle_with_fallback_still_invalid() {
    // W3C CSS Variables §3.2: Fallback does NOT rescue a variable that participates in a cycle
    let html = r#"<div id="target" style="--a: var(--b, red); --b: var(--a, blue); color: var(--a);"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("black"));
}

#[test]
fn test_iacvt_inheritable_falls_back_to_parent() {
    // Parent defines color: blue. Child has IACVT on color. Child must inherit parent color: blue.
    let html = r#"
        <div id="parent" style="color: blue;">
            <div id="child" style="--cycle: var(--cycle); color: var(--cycle);"></div>
        </div>
    "#;
    let doc = parse_html(html);
    let child_id = doc.get_element_by_id("child").expect("child element");

    let computed = StyleResolver::resolve_element_style(&doc, child_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("blue"));
}

#[test]
fn test_iacvt_non_inheritable_resets_to_initial() {
    // Parent defines margin: 50px, display: flex. Child has IACVT on margin and display.
    // display and margin are non-inheritable -> reset to initial values ('inline', '0px').
    let html = r#"
        <div id="parent" style="margin: 50px; display: flex;">
            <div id="child" style="--cycle: var(--cycle); display: var(--cycle); margin: var(--cycle);"></div>
        </div>
    "#;
    let doc = parse_html(html);
    let child_id = doc.get_element_by_id("child").expect("child element");

    let computed = StyleResolver::resolve_element_style(&doc, child_id, &[]);
    assert_eq!(computed.get_property_value("display"), Some("inline"));
    assert_eq!(computed.get_property_value("margin"), Some("0px"));
}

#[test]
fn test_custom_properties_case_sensitivity() {
    // W3C CSS Variables §2: Custom properties are case-sensitive
    let html = r#"
        <div id="target" style="
            --mainColor: #ff0000;
            --maincolor: #00ff00;
            --MAINCOLOR: #0000ff;
            color: var(--mainColor);
            background-color: var(--maincolor);
            border-color: var(--MAINCOLOR);
        "></div>
    "#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("#ff0000"));
    assert_eq!(computed.get_property_value("background-color"), Some("#00ff00"));
    assert_eq!(computed.get_property_value("border-color"), Some("#0000ff"));
    assert_eq!(computed.get_custom_property("--mainColor"), Some("#ff0000"));
    assert_eq!(computed.get_custom_property("--maincolor"), Some("#00ff00"));
    assert_eq!(computed.get_custom_property("--MAINCOLOR"), Some("#0000ff"));
    assert_eq!(computed.get_custom_property("--MainColor"), None);
}

#[test]
fn test_custom_properties_tree_inheritance_and_shadowing() {
    let html = r#"
        <div id="root" style="--accent: #ff9900; --padding: 10px;">
            <div id="parent" style="--padding: 24px;">
                <div id="child" style="color: var(--accent); padding: var(--padding);"></div>
            </div>
        </div>
    "#;
    let doc = parse_html(html);
    let child_id = doc.get_element_by_id("child").expect("child element");

    let computed = StyleResolver::resolve_element_style(&doc, child_id, &[]);
    assert_eq!(computed.get_property_value("color"), Some("#ff9900"));
    assert_eq!(computed.get_property_value("padding"), Some("24px"));
}

#[test]
fn test_inline_custom_properties_override_stylesheet() {
    let html = r#"<div id="target" class="card" style="--theme-size: 40px; width: var(--theme-size);"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let css = r#"
        .card {
            --theme-size: 100px;
            width: var(--theme-size);
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);
    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[sheet]);

    // Inline style sobrescreve custom property da folha de estilo
    assert_eq!(computed.get_property_value("width"), Some("40px"));
    assert_eq!(computed.get_custom_property("--theme-size"), Some("40px"));
}
