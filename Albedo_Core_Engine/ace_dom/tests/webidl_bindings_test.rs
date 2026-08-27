use ace_dom::bindings::{
    DOMDataStore, DocumentBindings, ElementBindings, JSObjectId, NodeBindings,
};
use ace_dom::gc::{GcTracer, MarkTracer, Traceable};
use ace_dom::parse_html;

#[test]
fn test_webidl_node_and_element_bindings() {
    let mut doc = parse_html(r#"<div id="card" class="box active">Hello <b>World</b></div>"#);
    let card_id = doc.get_element_by_id("card").expect("card deve existir");

    // 1. NodeBindings
    assert_eq!(NodeBindings::get_node_type(&doc, card_id), Some(1));
    assert_eq!(NodeBindings::get_node_name(&doc, card_id).as_deref(), Some("DIV"));
    assert_eq!(
        NodeBindings::get_text_content(&doc, card_id).as_deref(),
        Some("Hello World")
    );

    // Modifica textContent via WebIDL
    NodeBindings::set_text_content(&mut doc, card_id, "Texto Atualizado").unwrap();
    assert_eq!(
        NodeBindings::get_text_content(&doc, card_id).as_deref(),
        Some("Texto Atualizado")
    );

    // 2. ElementBindings
    assert_eq!(ElementBindings::get_tag_name(&doc, card_id).as_deref(), Some("DIV"));
    assert_eq!(ElementBindings::get_id(&doc, card_id).as_deref(), Some("card"));
    assert!(ElementBindings::has_attribute(&doc, card_id, "class"));
    assert_eq!(
        ElementBindings::get_attribute(&doc, card_id, "class").as_deref(),
        Some("box active")
    );

    // Modifica innerHTML via WebIDL
    ElementBindings::set_inner_html(&mut doc, card_id, "<p>Novo Parágrafo</p>").unwrap();
    assert_eq!(
        ElementBindings::get_inner_html(&doc, card_id).as_str(),
        "<p>Novo Parágrafo</p>"
    );

    // querySelector via WebIDL
    let p_id = ElementBindings::query_selector(&doc, card_id, "p");
    assert!(p_id.is_some());
}

#[test]
fn test_dom_data_store_wrapper_uniqueness_and_gc_tracing() {
    let mut doc = parse_html(r#"<div id="test"></div>"#);
    let test_id = doc.get_element_by_id("test").unwrap();

    let mut data_store = DOMDataStore::new();

    // Cria wrapper
    let wrapper1 = data_store
        .get_or_create_wrapper(test_id, JSObjectId::new)
        .clone();
    // Segundo acesso deve retornar exatamente o mesmo objeto (Uniqueness)
    let wrapper2 = data_store
        .get_or_create_wrapper(test_id, JSObjectId::new)
        .clone();

    assert_eq!(wrapper1.js_object_id, wrapper2.js_object_id);
    assert_eq!(data_store.get_node_by_js_id(wrapper1.js_object_id), Some(test_id));

    // Valida GC Tracing através do DataStore
    let mut tracer = MarkTracer::new();
    data_store.trace(&mut tracer);
    assert!(tracer.is_marked(test_id));
}
