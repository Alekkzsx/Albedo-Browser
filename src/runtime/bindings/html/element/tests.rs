use super::*;
use crate::engine::dom::AceDOM;
use crate::engine::style;
use crate::engine::ElementGeometry;
use crate::runtime::core::runtime::JsRuntime;
use std::sync::{Arc, Mutex};

// Helper to create test environment
fn create_test_env(
    html: &str,
) -> (
    JsRuntime,
    Arc<Mutex<AceDOM>>,
    Arc<Mutex<std::collections::HashMap<usize, ElementGeometry>>>,
    Arc<Mutex<std::collections::HashMap<usize, (f32, f32)>>>,
) {
    // Usando ACE-HTML parser proprietário ao invés de kuchiki
    let dom = Arc::new(Mutex::new(AceDOM::from_html(html)));
    let stylesheet = Arc::new(Mutex::new(style::parse("")));
    let primitives = Arc::new(Mutex::new(Vec::new()));
    let canvas_contexts = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let geometry = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let scroll = Arc::new(Mutex::new(std::collections::HashMap::new()));

    let mut rt = JsRuntime::new().unwrap();

    // Manually link geometry maps to runtime
    rt.element_geometry = geometry.clone();
    rt.element_scroll = scroll.clone();

    // We assume register is available via crate::runtime::bindings::html::document::register
    let _ = crate::runtime::bindings::html::document::register(
        &rt,
        dom.clone(),
        stylesheet,
        primitives,
        canvas_contexts,
        "http://test.com".to_string(),
        "".to_string(),
        None, // resource_manager
    );

    (rt, dom, geometry, scroll)
}

#[test]
fn test_client_dimensions() {
    let html = r#"<div id="test"></div>"#;
    let (rt, dom, geometry, _) = create_test_env(html);

    // Find node_idx for #test
    let mut node_idx = 0;
    {
        let dom = dom.lock().unwrap();
        for (idx, node) in dom.nodes.iter().enumerate() {
            if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                if el.attributes.get("id").map(|s| s.as_str()) == Some("test") {
                    node_idx = idx;
                    break;
                }
            }
        }
    }

    // Populate element geometry manually (simulating engine layout)
    {
        let mut geom = ElementGeometry::new();
        geom.width = 100.0;
        geom.height = 100.0;
        geom.border_top = 5.0;
        geom.border_right = 5.0;
        geom.border_bottom = 5.0;
        geom.border_left = 5.0;
        geom.padding_top = 10.0; // padding doesn't subtract from clientWidth/Height in standard box-model IF width is content-box, but here we store layout width.
                                 // Wait, Taffy width is total width (border-box)? Or content-box?
                                 // Taffy default is border-box if I recall correctly or depends on configuration.
                                 // But ElementGeometry `width` comes from `layout.size.width`.
                                 // clientWidth = width - border.

        geometry.lock().unwrap().insert(node_idx, geom);
    }

    let result = rt
        .execute_script(
            "
        var el = document.getElementById('test');
        el.clientWidth + '|' + el.clientHeight
    ",
        )
        .unwrap();

    // 100 - 5 - 5 = 90
    assert_eq!(result, "90|90");
}

#[test]
fn test_scroll_dimensions() {
    let html = r#"<div id="container"></div>"#;
    let (rt, dom, geometry, _) = create_test_env(html);

    let mut node_idx = 0;
    {
        let dom = dom.lock().unwrap();
        for (idx, node) in dom.nodes.iter().enumerate() {
            if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                if el.attributes.get("id").map(|s| s.as_str()) == Some("container") {
                    node_idx = idx;
                    break;
                }
            }
        }
    }

    {
        let mut geom = ElementGeometry::new();
        geom.width = 100.0; // Client width ~100
        geom.height = 100.0;
        geom.content_width = 200.0; // Inner content is 200
        geom.content_height = 300.0;

        geometry.lock().unwrap().insert(node_idx, geom);
    }

    let result = rt
        .execute_script(
            "
        var el = document.getElementById('container');
        el.scrollWidth + '|' + el.scrollHeight
    ",
        )
        .unwrap();

    assert_eq!(result, "200|300");
}

#[test]
fn test_scroll_position() {
    let html = r#"<div id="scrollable"></div>"#;
    let (rt, dom, _, scroll) = create_test_env(html);

    let mut node_idx = 0;
    {
        let dom = dom.lock().unwrap();
        for (idx, node) in dom.nodes.iter().enumerate() {
            if let crate::engine::dom::AceNodeType::Element(el) = &node.node_type {
                if el.attributes.get("id").map(|s| s.as_str()) == Some("scrollable") {
                    node_idx = idx;
                    break;
                }
            }
        }
    }

    // JS Setters
    let result = rt
        .execute_script(
            "
        var el = document.getElementById('scrollable');
        el.scrollTop = 50;
        el.scrollLeft = 25;
        el.scrollTop + '|' + el.scrollLeft
    ",
        )
        .unwrap();

    assert_eq!(result, "50|25");

    // Verify persistence
    {
        let scroll_map = scroll.lock().unwrap();
        let (x, y) = scroll_map.get(&node_idx).unwrap();
        assert_eq!(*x, 25.0);
        assert_eq!(*y, 50.0);
    }
}
