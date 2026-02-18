
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

    let _document = kuchiki::parse_html().one(html);
    let dom = AceDOM::new();

    // Verify Metadata
    // assert_eq!(dom.metadata.title, "Test Page".to_string());
    // assert_eq!(dom.metadata.charset, "utf-8".to_string());
    
    // Check resources
    // assert!(dom.resources.contains(&"style.css".to_string()));
    // assert!(dom.resources.contains(&"image.png".to_string()));

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
        // Search recursively or just children? The div is direct child of body in this HTML
        for &child_idx in &body_node.children {
            let child = dom.get_node(child_idx).unwrap();
             // Skip text nodes (newline/whitespace)
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
