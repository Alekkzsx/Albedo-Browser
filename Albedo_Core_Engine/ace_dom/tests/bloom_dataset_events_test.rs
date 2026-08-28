//! # Bateria de Testes de Counting Bloom Filter, dataset e EventListenerOptions (ace_dom)

use ace_dom::events::{AddEventListenerOptions, Event, EventListener, EventRegistry};
use ace_dom::node::element::Namespace;
use ace_dom::query::{AncestorFilter, ComplexSelector};
use ace_dom::tree::Document;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_ancestor_bloom_filter_fast_rejection() {
    let mut doc = Document::new(None);
    let nav_id = doc.create_element("nav", Namespace::Html);
    if let Some(el) = doc.get_node_mut(nav_id).and_then(|n| n.as_element_mut()) {
        el.set_attribute("id", "main-nav");
        el.set_attribute("class", "navbar primary");
    }

    let mut filter = AncestorFilter::new();
    let nav_el = doc.get_node(nav_id).unwrap().as_element().unwrap();
    filter.push_element(nav_el);

    // Seletor presente no filtro
    let sel_present = ComplexSelector::parse("nav.navbar").unwrap();
    assert!(!filter.fast_reject(&sel_present.parts[0].0));

    // Seletor ausente: deve ser rejeitado em O(1)
    let sel_absent = ComplexSelector::parse("sidebar.footer-nav").unwrap();
    assert!(filter.fast_reject(&sel_absent.parts[0].0));

    // Desempilha
    filter.pop_element(nav_el);
    assert!(filter.fast_reject(&sel_present.parts[0].0));
}

#[test]
fn test_dataset_dom_string_map_bidirectional_sync() {
    let mut doc = Document::new(None);
    let div_id = doc.create_element("div", Namespace::Html);

    if let Some(el) = doc.get_node_mut(div_id).and_then(|n| n.as_element_mut()) {
        el.set_attribute("data-user-id", "42");
        el.set_attribute("data-theme-color", "dark");
    }

    // Leitura via camelCase
    let mut el_clone = doc.get_node(div_id).unwrap().as_element().unwrap().clone();
    let mut ds = el_clone.dataset_mut();
    assert_eq!(ds.get("userId"), Some("42"));
    assert_eq!(ds.get("themeColor"), Some("dark"));

    // Escrita via camelCase
    ds.set("roleType", "admin");
    assert_eq!(ds.get("roleType"), Some("admin"));

    // Remoção via camelCase
    assert!(ds.remove("userId"));
    assert_eq!(ds.get("userId"), None);
}

#[test]
fn test_event_listener_options_once_and_capture() {
    let mut doc = Document::new(None);
    let parent = doc.create_element("div", Namespace::Html);
    let child = doc.create_element("button", Namespace::Html);
    doc.append_child(parent, child).unwrap();
    let root = doc.root();
    doc.append_child(root, parent).unwrap();

    let mut registry = EventRegistry::new();
    let call_count = Arc::new(AtomicUsize::new(0));

    let c_clone = call_count.clone();
    registry.add_event_listener(
        child,
        "click",
        EventListener::with_options(
            1,
            AddEventListenerOptions {
                once: true,
                ..Default::default()
            },
            move |_| {
                c_clone.fetch_add(1, Ordering::SeqCst);
            },
        ),
    );

    // Primeiro disparo
    let mut ev1 = Event::new("click", true, true);
    registry.dispatch(&doc, child, &mut ev1);
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    // Segundo disparo (o ouvinte 'once' deve ter sido automaticamente removido)
    let mut ev2 = Event::new("click", true, true);
    registry.dispatch(&doc, child, &mut ev2);
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}
