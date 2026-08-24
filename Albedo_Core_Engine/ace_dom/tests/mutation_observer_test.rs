//! # Bateria de Testes do MutationObserver (ace_dom)

use ace_dom::node::element::Namespace;
use ace_dom::observer::{MutationObserver, MutationObserverInit, MutationRecord, MutationType};
use ace_dom::tree::Document;
use ace_core::intern::Atom;
use smol_str::SmolStr;

#[test]
fn test_mutation_observer_child_list_recording() {
    let mut doc = Document::new(None);
    let parent_id = doc.create_element("div", Namespace::Html);
    let child1 = doc.create_element("span", Namespace::Html);

    let mut observer = MutationObserver::new();
    observer.observe(
        parent_id,
        MutationObserverInit {
            child_list: true,
            ..Default::default()
        },
    );

    // Simula a adição do filho
    let record = MutationRecord::child_list(
        parent_id,
        vec![child1],
        Vec::new(),
        None,
        None,
    );
    observer.notify_mutation(record);

    let records = observer.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record_type, MutationType::ChildList);
    assert_eq!(records[0].added_nodes, vec![child1]);

    // Fila esvaziada
    assert!(observer.take_records().is_empty());
}

#[test]
fn test_mutation_observer_attribute_filter() {
    let mut doc = Document::new(None);
    let el_id = doc.create_element("button", Namespace::Html);

    let mut observer = MutationObserver::new();
    observer.observe(
        el_id,
        MutationObserverInit {
            attributes: true,
            attribute_filter: Some(vec![Atom::new("disabled"), Atom::new("aria-hidden")]),
            ..Default::default()
        },
    );

    // Notificação com atributo permitido
    observer.notify_mutation(MutationRecord::attribute(
        el_id,
        Atom::new("disabled"),
        None,
    ));

    // Notificação com atributo ignorado pelo filtro
    observer.notify_mutation(MutationRecord::attribute(
        el_id,
        Atom::new("class"),
        Some(SmolStr::new("old-btn")),
    ));

    let records = observer.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].attribute_name, Some(Atom::new("disabled")));
}
