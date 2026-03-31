#![cfg(test)]

use crate::engine::dom::{AceDOM, AceNodeType};

fn find_first_element_by_tag(dom: &AceDOM, tag: &str) -> usize {
    dom.nodes
        .iter()
        .enumerate()
        .find_map(|(idx, node)| match &node.node_type {
            AceNodeType::Element(el) if el.tag == tag => Some(idx),
            _ => None,
        })
        .unwrap_or_else(|| panic!("element <{}> not found", tag))
}

#[test]
fn test_acedom_parsing() {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Test Page</title>
            <meta charset="utf-8">
            <link rel="stylesheet" href="style.css">
        </head>
        <body class="main" id="body-id">
            <div data-custom="value">Hello</div>
            <img src="image.png" />
        </body>
        </html>
    "#;

    let dom = AceDOM::from_html(html);

    // Verify Title (Metadata extraction happens in AceDOM::from_html_document)
    // Check nodes directly if metadata failing
    
    // Verify Body Attributes
    if let Some(body_idx) = dom.body {
        let body_node = dom.get_node(body_idx).unwrap();
        if let AceNodeType::Element(el) = &body_node.node_type {
            assert_eq!(el.attributes.get("class"), Some(&"main".to_string()));
            assert_eq!(el.attributes.get("id"), Some(&"body-id".to_string()));
            assert_eq!(el.tag, "body");
        } else {
            panic!("Body is not an element");
        }

        // Verify Child Div
        let mut found_div = false;
        for &child_idx in &body_node.children {
            let child = dom.get_node(child_idx).unwrap();
            if let AceNodeType::Element(el) = &child.node_type {
                if el.tag == "div" {
                    assert_eq!(el.attributes.get("data-custom"), Some(&"value".to_string()));
                    found_div = true;
                }
            }
        }
        assert!(found_div, "Did not find child div");
    } else {
        panic!("Body not found");
    }
}

#[test]
fn test_css_parsing() {
    let css = "body { color: red; border: 1px solid black; }\n.box { margin: 10px; }";
    let stylesheet = crate::engine::style::parse(css);
    assert_eq!(
        stylesheet.rules.len(),
        2,
        "Deveria ter extraido 2 regras do CSS"
    );
}

#[test]
fn test_inner_html_respects_textarea_context() {
    let mut dom = AceDOM::from_html("<textarea></textarea>");
    let textarea_idx = find_first_element_by_tag(&dom, "textarea");

    dom.set_inner_html_from_html(textarea_idx, "A &lt; B");
    assert_eq!(dom.serialize_subtree_text(textarea_idx), "A < B");
}

#[test]
fn test_inner_html_respects_table_context() {
    let mut dom = AceDOM::from_html("<table></table>");
    let table_idx = find_first_element_by_tag(&dom, "table");

    dom.set_inner_html_from_html(table_idx, "<tr><td>cell</td></tr>");
    let serialized = dom.serialize_subtree_html(table_idx);

    assert!(serialized.contains("<tr>"), "expected table row in: {}", serialized);
    assert!(serialized.contains("cell"), "expected cell text in: {}", serialized);
}

#[test]
fn test_import_html_fragment_uses_parent_context() {
    let mut dom = AceDOM::from_html("<select></select>");
    let select_idx = find_first_element_by_tag(&dom, "select");

    let imported = dom.import_html_fragment("<option>one</option>", Some(select_idx));
    assert_eq!(imported.len(), 1);

    let first = imported[0];
    let node = dom.get_node(first).expect("imported node must exist");
    match &node.node_type {
        AceNodeType::Element(el) => assert_eq!(el.tag, "option"),
        _ => panic!("expected imported fragment root to be <option>"),
    }
}
