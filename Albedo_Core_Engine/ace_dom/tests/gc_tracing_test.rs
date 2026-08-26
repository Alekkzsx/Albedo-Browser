use ace_dom::{parse_html, MarkTracer, ShadowMode};

#[test]
fn test_unified_heap_gc_tracing_reachability() {
    let html = r#"
        <!DOCTYPE html>
        <html>
            <head><title>Tracing Test</title></head>
            <body>
                <div id="container">
                    <p class="para">Paragraph 1</p>
                    <p class="para">Paragraph 2</p>
                </div>
            </body>
        </html>
    "#;

    let doc = parse_html(html);
    let mut tracer = MarkTracer::new();
    tracer.trace_document(&doc);

    // Todos os nós da árvore conectada devem ser alcançáveis
    let container_id = doc.get_element_by_id("container").unwrap();
    assert!(tracer.visited_nodes.contains(&container_id));
    assert!(tracer.visited_nodes.contains(&doc.root()));
    assert!(tracer.visited_nodes.contains(&doc.document_element.unwrap()));
    assert!(tracer.visited_nodes.contains(&doc.body.unwrap()));
}

#[test]
fn test_gc_tracing_with_shadow_root_and_rare_data() {
    let html = r#"<div id="host"><p>Slot candidate</p></div>"#;
    let mut doc = parse_html(html);

    let host_id = doc.get_element_by_id("host").unwrap();
    let shadow_root_id = doc.attach_shadow(host_id, ShadowMode::Open).unwrap();
    let inner_span = doc.create_element("span", ace_dom::Namespace::Html);
    doc.append_child(shadow_root_id, inner_span).unwrap();

    let mut tracer = MarkTracer::new();
    tracer.trace_document(&doc);

    assert!(tracer.visited_nodes.contains(&host_id));
    assert!(tracer.visited_nodes.contains(&shadow_root_id));
    assert!(tracer.visited_nodes.contains(&inner_span));
}
