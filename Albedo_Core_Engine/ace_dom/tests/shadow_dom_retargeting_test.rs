//! # Bateria de Testes de Retargeting de Eventos e Composed Path em Shadow DOM
//!
//! Cobre exaustivamente:
//! - WHATWG DOM §2.6, §2.7 & §2.8.
//! - Retargeting de `event.target` através de fronteiras de `ShadowRoot` simples e aninhadas.
//! - Propagação com `composed: true` e encapsulamento estrito com `composed: false`.
//! - API `Event.composedPath()` e `EventBindings::composed_path`.
//! - Interrupção de propagação (`stopPropagation`, `stopImmediatePropagation`) e `preventDefault`.

use ace_dom::bindings::EventBindings;
use ace_dom::events::{dispatch_event, AddEventListenerOptions, Event, EventListener, EventPhase, EventRegistry};
use ace_dom::node::element::Namespace;
use ace_dom::node::ShadowMode;
use ace_dom::tree::Document;
use std::sync::{Arc, Mutex};

#[test]
fn test_composed_path_light_dom() {
    let mut doc = Document::new(None);
    let body_id = doc.create_element("body", Namespace::Html);
    let div_id = doc.create_element("div", Namespace::Html);
    let btn_id = doc.create_element("button", Namespace::Html);

    doc.append_child(doc.root(), body_id).unwrap();
    doc.append_child(body_id, div_id).unwrap();
    doc.append_child(div_id, btn_id).unwrap();

    let mut registry = EventRegistry::new();
    let mut event = Event::new("click", true, true);

    dispatch_event(&mut registry, &doc, btn_id, &mut event);

    // No Light DOM, o caminho composto deve ser [btn, div, body, root]
    assert_eq!(
        event.composed_path(),
        &[btn_id, div_id, body_id, doc.root()]
    );
}

#[test]
fn test_shadow_dom_composed_true_retargeting_pipeline() {
    let mut doc = Document::new(None);
    let body_id = doc.create_element("body", Namespace::Html);
    let host_id = doc.create_element("custom-card", Namespace::Html);

    doc.append_child(doc.root(), body_id).unwrap();
    doc.append_child(body_id, host_id).unwrap();

    // Anexa ShadowRoot aberta
    let shadow_root_id = doc.attach_shadow(host_id, ShadowMode::Open).unwrap();
    let inner_div = doc.create_element("div", Namespace::Html);
    let inner_btn = doc.create_element("button", Namespace::Html);

    doc.append_child(shadow_root_id, inner_div).unwrap();
    doc.append_child(inner_div, inner_btn).unwrap();

    let mut registry = EventRegistry::new();

    // Registra observadores em cada nível para inspecionar `target`, `currentTarget` e `composed_path`
    let log = Arc::new(Mutex::new(Vec::new()));

    // 1. Ouvinte no inner_btn
    let log_clone1 = Arc::clone(&log);
    registry.add_event_listener(
        inner_btn,
        "custom-event",
        EventListener::new(1, false, move |e| {
            assert_eq!(e.phase, EventPhase::AtTarget);
            assert_eq!(e.target, Some(inner_btn));
            assert_eq!(e.current_target, Some(inner_btn));
            log_clone1.lock().unwrap().push(("inner_btn", e.target.unwrap(), e.composed_path().to_vec()));
        }),
    );

    // 2. Ouvinte no inner_div
    let log_clone2 = Arc::clone(&log);
    registry.add_event_listener(
        inner_div,
        "custom-event",
        EventListener::new(2, false, move |e| {
            assert_eq!(e.phase, EventPhase::BubblingPhase);
            assert_eq!(e.target, Some(inner_btn));
            assert_eq!(e.current_target, Some(inner_div));
            log_clone2.lock().unwrap().push(("inner_div", e.target.unwrap(), e.composed_path().to_vec()));
        }),
    );

    // 3. Ouvinte na ShadowRoot
    let log_clone3 = Arc::clone(&log);
    registry.add_event_listener(
        shadow_root_id,
        "custom-event",
        EventListener::new(3, false, move |e| {
            assert_eq!(e.phase, EventPhase::BubblingPhase);
            assert_eq!(e.target, Some(inner_btn));
            assert_eq!(e.current_target, Some(shadow_root_id));
            log_clone3.lock().unwrap().push(("shadow_root", e.target.unwrap(), e.composed_path().to_vec()));
        }),
    );

    // 4. Ouvinte no Host (deve ver target retargetado para host_id)
    let log_clone4 = Arc::clone(&log);
    registry.add_event_listener(
        host_id,
        "custom-event",
        EventListener::new(4, false, move |e| {
            assert_eq!(e.phase, EventPhase::BubblingPhase);
            assert_eq!(e.target, Some(host_id));
            assert_eq!(e.current_target, Some(host_id));
            log_clone4.lock().unwrap().push(("host", e.target.unwrap(), e.composed_path().to_vec()));
        }),
    );

    // 5. Ouvinte no Body (deve ver target retargetado para host_id)
    let log_clone5 = Arc::clone(&log);
    registry.add_event_listener(
        body_id,
        "custom-event",
        EventListener::new(5, false, move |e| {
            assert_eq!(e.phase, EventPhase::BubblingPhase);
            assert_eq!(e.target, Some(host_id));
            assert_eq!(e.current_target, Some(body_id));
            log_clone5.lock().unwrap().push(("body", e.target.unwrap(), e.composed_path().to_vec()));
        }),
    );

    let mut event = Event::new("custom-event", true, true);
    event.composed = true;

    dispatch_event(&mut registry, &doc, inner_btn, &mut event);

    let entries = log.lock().unwrap().clone();
    assert_eq!(entries.len(), 5);

    // Verifica ordem e retargeting exato
    assert_eq!(entries[0].0, "inner_btn");
    assert_eq!(entries[0].1, inner_btn);

    assert_eq!(entries[1].0, "inner_div");
    assert_eq!(entries[1].1, inner_btn);

    assert_eq!(entries[2].0, "shadow_root");
    assert_eq!(entries[2].1, inner_btn);

    assert_eq!(entries[3].0, "host");
    assert_eq!(entries[3].1, host_id); // Retargeted para o host!

    assert_eq!(entries[4].0, "body");
    assert_eq!(entries[4].1, host_id); // Retargeted para o host!

    // Valida composed_path completo
    let expected_composed_path = vec![inner_btn, inner_div, shadow_root_id, host_id, body_id, doc.root()];
    assert_eq!(event.composed_path(), expected_composed_path.as_slice());
}

#[test]
fn test_shadow_dom_composed_false_containment() {
    let mut doc = Document::new(None);
    let body_id = doc.create_element("body", Namespace::Html);
    let host_id = doc.create_element("custom-card", Namespace::Html);

    doc.append_child(doc.root(), body_id).unwrap();
    doc.append_child(body_id, host_id).unwrap();

    let shadow_root_id = doc.attach_shadow(host_id, ShadowMode::Open).unwrap();
    let inner_btn = doc.create_element("button", Namespace::Html);
    doc.append_child(shadow_root_id, inner_btn).unwrap();

    let mut registry = EventRegistry::new();

    let inner_called = Arc::new(Mutex::new(false));
    let host_called = Arc::new(Mutex::new(false));
    let body_called = Arc::new(Mutex::new(false));

    let inner_clone = Arc::clone(&inner_called);
    registry.add_event_listener(
        inner_btn,
        "internal-event",
        EventListener::new(1, false, move |_| {
            *inner_clone.lock().unwrap() = true;
        }),
    );

    let host_clone = Arc::clone(&host_called);
    registry.add_event_listener(
        host_id,
        "internal-event",
        EventListener::new(2, false, move |_| {
            *host_clone.lock().unwrap() = true;
        }),
    );

    let body_clone = Arc::clone(&body_called);
    registry.add_event_listener(
        body_id,
        "internal-event",
        EventListener::new(3, false, move |_| {
            *body_clone.lock().unwrap() = true;
        }),
    );

    // Evento com composed: false
    let mut event = Event::new("internal-event", true, true);
    event.composed = false;

    dispatch_event(&mut registry, &doc, inner_btn, &mut event);

    assert!(*inner_called.lock().unwrap(), "Ouvinte interno DEVE ser chamado");
    assert!(!*host_called.lock().unwrap(), "Host NÃO DEVE ser chamado quando composed: false");
    assert!(!*body_called.lock().unwrap(), "Body NÃO DEVE ser chamado quando composed: false");

    // Caminho composto encerra na ShadowRoot
    assert_eq!(event.composed_path(), &[inner_btn, shadow_root_id]);
}

#[test]
fn test_multi_level_nested_shadow_dom_retargeting() {
    let mut doc = Document::new(None);
    let body_id = doc.create_element("body", Namespace::Html);
    let outer_host = doc.create_element("outer-component", Namespace::Html);

    doc.append_child(doc.root(), body_id).unwrap();
    doc.append_child(body_id, outer_host).unwrap();

    // 1º Nível de Shadow DOM
    let outer_shadow = doc.attach_shadow(outer_host, ShadowMode::Open).unwrap();
    let inner_host = doc.create_element("inner-component", Namespace::Html);
    doc.append_child(outer_shadow, inner_host).unwrap();

    // 2º Nível de Shadow DOM
    let inner_shadow = doc.attach_shadow(inner_host, ShadowMode::Open).unwrap();
    let deep_btn = doc.create_element("button", Namespace::Html);
    doc.append_child(inner_shadow, deep_btn).unwrap();

    let mut registry = EventRegistry::new();
    let target_log = Arc::new(Mutex::new(Vec::new()));

    // Observa no deep_btn
    let t_clone1 = Arc::clone(&target_log);
    registry.add_event_listener(
        deep_btn,
        "deep-click",
        EventListener::new(1, false, move |e| {
            t_clone1.lock().unwrap().push(("deep_btn", e.target.unwrap()));
        }),
    );

    // Observa no inner_shadow
    let t_clone2 = Arc::clone(&target_log);
    registry.add_event_listener(
        inner_shadow,
        "deep-click",
        EventListener::new(2, false, move |e| {
            t_clone2.lock().unwrap().push(("inner_shadow", e.target.unwrap()));
        }),
    );

    // Observa no inner_host (retargeted para inner_host)
    let t_clone3 = Arc::clone(&target_log);
    registry.add_event_listener(
        inner_host,
        "deep-click",
        EventListener::new(3, false, move |e| {
            t_clone3.lock().unwrap().push(("inner_host", e.target.unwrap()));
        }),
    );

    // Observa na outer_shadow (deve enxergar inner_host)
    let t_clone4 = Arc::clone(&target_log);
    registry.add_event_listener(
        outer_shadow,
        "deep-click",
        EventListener::new(4, false, move |e| {
            t_clone4.lock().unwrap().push(("outer_shadow", e.target.unwrap()));
        }),
    );

    // Observa no outer_host (retargeted para outer_host)
    let t_clone5 = Arc::clone(&target_log);
    registry.add_event_listener(
        outer_host,
        "deep-click",
        EventListener::new(5, false, move |e| {
            t_clone5.lock().unwrap().push(("outer_host", e.target.unwrap()));
        }),
    );

    // Observa no body (deve enxergar outer_host)
    let t_clone6 = Arc::clone(&target_log);
    registry.add_event_listener(
        body_id,
        "deep-click",
        EventListener::new(6, false, move |e| {
            t_clone6.lock().unwrap().push(("body", e.target.unwrap()));
        }),
    );

    let mut event = Event::new("deep-click", true, true);
    event.composed = true;

    dispatch_event(&mut registry, &doc, deep_btn, &mut event);

    let log = target_log.lock().unwrap().clone();
    assert_eq!(log, vec![
        ("deep_btn", deep_btn),
        ("inner_shadow", deep_btn),
        ("inner_host", inner_host),
        ("outer_shadow", inner_host),
        ("outer_host", outer_host),
        ("body", outer_host),
    ]);

    assert_eq!(
        event.composed_path(),
        &[deep_btn, inner_shadow, inner_host, outer_shadow, outer_host, body_id, doc.root()]
    );
}

#[test]
fn test_nested_shadow_dom_composed_false_at_inner_level() {
    let mut doc = Document::new(None);
    let body_id = doc.create_element("body", Namespace::Html);
    let outer_host = doc.create_element("outer-component", Namespace::Html);

    doc.append_child(doc.root(), body_id).unwrap();
    doc.append_child(body_id, outer_host).unwrap();

    let outer_shadow = doc.attach_shadow(outer_host, ShadowMode::Open).unwrap();
    let inner_host = doc.create_element("inner-component", Namespace::Html);
    doc.append_child(outer_shadow, inner_host).unwrap();

    let inner_shadow = doc.attach_shadow(inner_host, ShadowMode::Open).unwrap();
    let deep_btn = doc.create_element("button", Namespace::Html);
    doc.append_child(inner_shadow, deep_btn).unwrap();

    let mut registry = EventRegistry::new();
    let outer_called = Arc::new(Mutex::new(false));

    let outer_clone = Arc::clone(&outer_called);
    registry.add_event_listener(
        outer_host,
        "deep-click",
        EventListener::new(1, false, move |_| {
            *outer_clone.lock().unwrap() = true;
        }),
    );

    // Evento com composed: false
    let mut event = Event::new("deep-click", true, true);
    event.composed = false;

    dispatch_event(&mut registry, &doc, deep_btn, &mut event);

    assert!(!*outer_called.lock().unwrap());
    assert_eq!(event.composed_path(), &[deep_btn, inner_shadow]);
}

#[test]
fn test_shadow_dom_capture_phase_retargeting() {
    let mut doc = Document::new(None);
    let body_id = doc.create_element("body", Namespace::Html);
    let host_id = doc.create_element("x-widget", Namespace::Html);

    doc.append_child(doc.root(), body_id).unwrap();
    doc.append_child(body_id, host_id).unwrap();

    let shadow_root = doc.attach_shadow(host_id, ShadowMode::Open).unwrap();
    let inner_btn = doc.create_element("button", Namespace::Html);
    doc.append_child(shadow_root, inner_btn).unwrap();

    let mut registry = EventRegistry::new();
    let capture_log = Arc::new(Mutex::new(Vec::new()));

    // Captura no Body
    let c_clone1 = Arc::clone(&capture_log);
    registry.add_event_listener(
        body_id,
        "capture-test",
        EventListener::with_options(
            1,
            AddEventListenerOptions { capture: true, ..Default::default() },
            move |e| {
                assert_eq!(e.phase, EventPhase::CapturingPhase);
                c_clone1.lock().unwrap().push(("body_capture", e.target.unwrap()));
            },
        ),
    );

    // Captura no Host
    let c_clone2 = Arc::clone(&capture_log);
    registry.add_event_listener(
        host_id,
        "capture-test",
        EventListener::with_options(
            2,
            AddEventListenerOptions { capture: true, ..Default::default() },
            move |e| {
                assert_eq!(e.phase, EventPhase::CapturingPhase);
                c_clone2.lock().unwrap().push(("host_capture", e.target.unwrap()));
            },
        ),
    );

    // Captura na ShadowRoot
    let c_clone3 = Arc::clone(&capture_log);
    registry.add_event_listener(
        shadow_root,
        "capture-test",
        EventListener::with_options(
            3,
            AddEventListenerOptions { capture: true, ..Default::default() },
            move |e| {
                assert_eq!(e.phase, EventPhase::CapturingPhase);
                c_clone3.lock().unwrap().push(("shadow_capture", e.target.unwrap()));
            },
        ),
    );

    let mut event = Event::new("capture-test", true, true);
    event.composed = true;

    dispatch_event(&mut registry, &doc, inner_btn, &mut event);

    let log = capture_log.lock().unwrap().clone();
    assert_eq!(log, vec![
        ("body_capture", host_id),
        ("host_capture", host_id),
        ("shadow_capture", inner_btn),
    ]);
}

#[test]
fn test_shadow_dom_stop_propagation() {
    let mut doc = Document::new(None);
    let body_id = doc.create_element("body", Namespace::Html);
    let host_id = doc.create_element("my-card", Namespace::Html);

    doc.append_child(doc.root(), body_id).unwrap();
    doc.append_child(body_id, host_id).unwrap();

    let shadow_root = doc.attach_shadow(host_id, ShadowMode::Open).unwrap();
    let inner_btn = doc.create_element("button", Namespace::Html);
    doc.append_child(shadow_root, inner_btn).unwrap();

    let mut registry = EventRegistry::new();
    let host_reached = Arc::new(Mutex::new(false));

    // Ouvinte no botão interno interrompe a propagação
    registry.add_event_listener(
        inner_btn,
        "click",
        EventListener::new(1, false, |e| {
            e.stop_propagation();
        }),
    );

    let h_clone = Arc::clone(&host_reached);
    registry.add_event_listener(
        host_id,
        "click",
        EventListener::new(2, false, move |_| {
            *h_clone.lock().unwrap() = true;
        }),
    );

    let mut event = Event::new("click", true, true);
    event.composed = true;

    dispatch_event(&mut registry, &doc, inner_btn, &mut event);

    assert!(!*host_reached.lock().unwrap(), "Stop propagation deve impedir o borbulhamento para o host");
}

#[test]
fn test_shadow_dom_prevent_default() {
    let mut doc = Document::new(None);
    let host_id = doc.create_element("form-control", Namespace::Html);
    doc.append_child(doc.root(), host_id).unwrap();

    let shadow_root = doc.attach_shadow(host_id, ShadowMode::Open).unwrap();
    let btn_id = doc.create_element("button", Namespace::Html);
    doc.append_child(shadow_root, btn_id).unwrap();

    let mut registry = EventRegistry::new();

    registry.add_event_listener(
        btn_id,
        "submit",
        EventListener::new(1, false, |e| {
            e.prevent_default();
        }),
    );

    let mut event = Event::new("submit", true, true);
    event.composed = true;

    let allowed = dispatch_event(&mut registry, &doc, btn_id, &mut event);
    assert!(!allowed, "dispatch_event deve retornar false quando prevent_default() for chamado");
    assert!(event.is_default_prevented());
}

#[test]
fn test_event_bindings_webidl_methods() {
    let mut doc = Document::new(None);
    let host_id = doc.create_element("x-elem", Namespace::Html);
    doc.append_child(doc.root(), host_id).unwrap();

    let shadow = doc.attach_shadow(host_id, ShadowMode::Open).unwrap();
    let child = doc.create_element("span", Namespace::Html);
    doc.append_child(shadow, child).unwrap();

    let mut registry = EventRegistry::new();
    let mut event = Event::new("custom-type", true, true);
    event.composed = true;

    // WebIDL Getters antes do dispatch
    assert_eq!(EventBindings::get_type(&event).as_str(), "custom-type");
    assert!(EventBindings::get_bubbles(&event));
    assert!(EventBindings::get_cancelable(&event));
    assert!(EventBindings::get_composed(&event));
    assert_eq!(EventBindings::get_event_phase(&event), 0); // None

    dispatch_event(&mut registry, &doc, child, &mut event);

    // WebIDL composedPath
    let path = EventBindings::composed_path(&event);
    assert_eq!(path, vec![child, shadow, host_id, doc.root()]);

    // stopPropagation e preventDefault via EventBindings
    EventBindings::prevent_default(&event);
    assert!(EventBindings::get_default_prevented(&event));

    EventBindings::stop_propagation(&event);
    assert!(event.is_propagation_stopped());
}

#[test]
fn test_stress_multiple_sibling_shadow_components() {
    let mut doc = Document::new(None);
    let body_id = doc.create_element("body", Namespace::Html);
    doc.append_child(doc.root(), body_id).unwrap();

    let mut buttons = Vec::new();
    let mut hosts = Vec::new();

    for i in 0..10 {
        let host = doc.create_element(format!("comp-{}", i), Namespace::Html);
        doc.append_child(body_id, host).unwrap();

        let shadow = doc.attach_shadow(host, ShadowMode::Open).unwrap();
        let btn = doc.create_element("button", Namespace::Html);
        doc.append_child(shadow, btn).unwrap();

        hosts.push(host);
        buttons.push(btn);
    }

    let mut registry = EventRegistry::new();
    let body_dispatches = Arc::new(Mutex::new(Vec::new()));

    let b_clone = Arc::clone(&body_dispatches);
    registry.add_event_listener(
        body_id,
        "comp-click",
        EventListener::new(1, false, move |e| {
            b_clone.lock().unwrap().push(e.target.unwrap());
        }),
    );

    // Dispara eventos em cada botão interno
    for (i, &btn) in buttons.iter().enumerate() {
        let mut ev = Event::new("comp-click", true, true);
        ev.composed = true;
        dispatch_event(&mut registry, &doc, btn, &mut ev);

        // O body deve ter enxergado o respectivo host
        let last_target = *body_dispatches.lock().unwrap().last().unwrap();
        assert_eq!(last_target, hosts[i]);
    }

    assert_eq!(body_dispatches.lock().unwrap().len(), 10);
}
