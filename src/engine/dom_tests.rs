#![cfg(test)]

use crate::engine::dom::{AceDOM, AceNodeType};
use kuchiki::traits::TendrilSink;

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
