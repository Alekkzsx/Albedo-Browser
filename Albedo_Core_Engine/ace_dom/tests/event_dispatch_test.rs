//! # Bateria de Testes de Despacho de Eventos DOM (ace_dom)

use ace_dom::events::{dispatch_event, Event, EventListener, EventPhase, EventRegistry};
use ace_dom::parse_html;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[test]
fn test_event_dispatch_bubbling_order() {
    let html = r#"
    <div id="parent">
      <button id="btn">Clique</button>
    </div>
    "#;

    let doc = parse_html(html);
    let parent_id = doc.get_element_by_id("parent").unwrap();
    let btn_id = doc.get_element_by_id("btn").unwrap();

    let mut registry = EventRegistry::new();
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));

    // Ouvinte de captura no pai
    let order_clone1 = Arc::clone(&order);
    registry.add_event_listener(
        parent_id,
        "click",
        EventListener {
            callback: Box::new(move |e| {
                assert_eq!(e.phase, EventPhase::CapturingPhase);
                order_clone1.lock().unwrap().push("parent_capture");
            }),
            capture: true,
            once: false,
            passive: false,
        },
    );

    // Ouvinte no alvo (button)
    let order_clone2 = Arc::clone(&order);
    registry.add_event_listener(
        btn_id,
        "click",
        EventListener {
            callback: Box::new(move |e| {
                assert_eq!(e.phase, EventPhase::AtTarget);
                order_clone2.lock().unwrap().push("btn_target");
            }),
            capture: false,
            once: false,
            passive: false,
        },
    );

    // Ouvinte de borbulhamento no pai
    let order_clone3 = Arc::clone(&order);
    registry.add_event_listener(
        parent_id,
        "click",
        EventListener {
            callback: Box::new(move |e| {
                assert_eq!(e.phase, EventPhase::BubblingPhase);
                order_clone3.lock().unwrap().push("parent_bubble");
            }),
            capture: false,
            once: false,
            passive: false,
        },
    );

    let mut event = Event::new("click", true, true);
    dispatch_event(&mut registry, &doc, btn_id, &mut event);

    let recorded = order.lock().unwrap().clone();
    assert_eq!(
        recorded,
        vec!["parent_capture", "btn_target", "parent_bubble"]
    );
}

#[test]
fn test_event_stop_propagation() {
    let html = "<div id='p'><button id='b'>Click</button></div>";
    let doc = parse_html(html);
    let parent_id = doc.get_element_by_id("p").unwrap();
    let btn_id = doc.get_element_by_id("b").unwrap();

    let mut registry = EventRegistry::new();
    let parent_called = Arc::new(AtomicU32::new(0));

    // Ouvinte no botão que interrompe a propagação
    registry.add_event_listener(
        btn_id,
        "click",
        EventListener {
            callback: Box::new(|e| {
                e.stop_propagation();
            }),
            capture: false,
            once: false,
            passive: false,
        },
    );

    // Ouvinte no pai (que NÃO deve ser chamado por causa do stop_propagation)
    let p_clone = Arc::clone(&parent_called);
    registry.add_event_listener(
        parent_id,
        "click",
        EventListener {
            callback: Box::new(move |_| {
                p_clone.fetch_add(1, Ordering::SeqCst);
            }),
            capture: false,
            once: false,
            passive: false,
        },
    );

    let mut event = Event::new("click", true, true);
    dispatch_event(&mut registry, &doc, btn_id, &mut event);

    assert_eq!(parent_called.load(Ordering::SeqCst), 0);
}
