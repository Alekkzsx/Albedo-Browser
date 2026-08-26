//! # Suíte Master de Verificação do Hardening do ace_dom
//!
//! Valida o Adoption Agency Algorithm canônico (16 passos), Event Retargeting com Shadow DOM,
//! Form Association remoto (form="id") e Validação Avançada de Restrições (min, max, step, pattern).

use ace_dom::events::{Event, EventListener, EventRegistry};
use ace_dom::form::{check_control_validity, FormData};
use ace_dom::node::element::Namespace;
use ace_dom::node::ShadowMode;
use ace_dom::parse_html;
use ace_dom::tree::Document;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_canonical_adoption_agency_algorithm_16_steps() {
    // Caso clássico do WHATWG: <b>1<p>2</b>3</p>
    let html = "<b>1<p>2</b>3</p>";
    let doc = parse_html(html);

    // O parágrafo <p> deve ter sido criado como irmão ou após o primeiro <b>
    let p_elements = doc.query_selector_all("p");
    assert_eq!(p_elements.len(), 1);

    // Dentro do <p>, deve haver um clone do <b> envolvendo o "2"
    let bold_elements = doc.query_selector_all("b");
    assert_eq!(bold_elements.len(), 2);
}

#[test]
fn test_shadow_dom_event_retargeting_and_composed_path() {
    let mut doc = Document::new(None);
    let body_id = doc.create_element("body", Namespace::Html);
    let host_id = doc.create_element("my-element", Namespace::Html);

    doc.append_child(doc.root(), body_id).unwrap();
    doc.append_child(body_id, host_id).unwrap();

    // Cria e anexa ShadowRoot
    let shadow_root_id = doc.attach_shadow(host_id, ShadowMode::Open).unwrap();
    let inner_btn_id = doc.create_element("button", Namespace::Html);
    doc.append_child(shadow_root_id, inner_btn_id).unwrap();

    let mut registry = EventRegistry::new();

    let body_target_seen = Arc::new(std::sync::Mutex::new(None));
    let btn_target_seen = Arc::new(std::sync::Mutex::new(None));

    let body_target_seen_clone = Arc::clone(&body_target_seen);
    registry.add_event_listener(
        body_id,
        "custom-click",
        EventListener::new(1, false, move |ev| {
            if let Some(target) = ev.target {
                *body_target_seen_clone.lock().unwrap() = Some(target);
            }
        }),
    );

    let btn_target_seen_clone = Arc::clone(&btn_target_seen);
    registry.add_event_listener(
        inner_btn_id,
        "custom-click",
        EventListener::new(2, false, move |ev| {
            if let Some(target) = ev.target {
                *btn_target_seen_clone.lock().unwrap() = Some(target);
            }
        }),
    );

    // Dispara evento com composed: true a partir do botão interno
    let mut event = Event::new("custom-click", true, true);
    event.composed = true;

    registry.dispatch(&doc, inner_btn_id, &mut event);

    // O botão interno vê target = inner_btn_id
    assert_eq!(*btn_target_seen.lock().unwrap(), Some(inner_btn_id));

    // O ouvinte no body (fora do Web Component) deve enxergar o target retargetado para host_id
    assert_eq!(*body_target_seen.lock().unwrap(), Some(host_id));
}

#[test]
fn test_form_data_remote_association_with_form_attr() {
    let html = r#"
    <body>
      <form id="contact-form">
        <input type="text" name="username" value="albedo_dev" />
      </form>
      <!-- Input colocado fora da tag <form>, mas referenciando form="contact-form" -->
      <input type="text" name="email" value="dev@albedo.org" form="contact-form" />
      <textarea name="bio" form="contact-form">Engenharia de navegadores de alta performance.</textarea>
      <!-- Input desabilitado (não deve ser submetido) -->
      <input type="text" name="ignored" value="skip" disabled form="contact-form" />
    </body>
    "#;

    let doc = parse_html(html);
    let form_id = doc.get_element_by_id("contact-form").expect("form encontrado");

    let form_data = FormData::from_form_element(&doc, form_id);

    assert_eq!(form_data.get("username").unwrap().as_text(), Some("albedo_dev"));
    assert_eq!(form_data.get("email").unwrap().as_text(), Some("dev@albedo.org"));
    assert_eq!(
        form_data.get("bio").unwrap().as_text(),
        Some("Engenharia de navegadores de alta performance.")
    );
    assert!(form_data.get("ignored").is_none());
}

#[test]
fn test_advanced_constraint_validation_rules() {
    let mut doc = Document::new(None);
    let input_id = doc.create_element("input", Namespace::Html);
    doc.append_child(doc.root(), input_id).unwrap();

    // 1. Validação de pattern
    {
        let node = doc.get_node_mut(input_id).unwrap();
        let el = node.as_element_mut().unwrap();
        el.set_attribute("type", "text");
        el.set_attribute("pattern", "[0-9]+");
        el.set_attribute("value", "abc");
    }
    let state = check_control_validity(&doc, input_id);
    assert!(state.pattern_mismatch);
    assert!(!state.valid());

    // 2. Correção do pattern
    {
        let node = doc.get_node_mut(input_id).unwrap();
        let el = node.as_element_mut().unwrap();
        el.set_attribute("value", "12345");
    }
    let state = check_control_validity(&doc, input_id);
    assert!(!state.pattern_mismatch);
    assert!(state.valid());

    // 3. Validação de range (min / max / step)
    {
        let node = doc.get_node_mut(input_id).unwrap();
        let el = node.as_element_mut().unwrap();
        el.remove_attribute("pattern");
        el.set_attribute("type", "number");
        el.set_attribute("min", "10");
        el.set_attribute("max", "100");
        el.set_attribute("step", "5");
        el.set_attribute("value", "7"); // Menor que o min (10)
    }
    let state = check_control_validity(&doc, input_id);
    assert!(state.range_underflow);
    assert!(!state.valid());

    // 4. Step mismatch
    {
        let node = doc.get_node_mut(input_id).unwrap();
        let el = node.as_element_mut().unwrap();
        el.set_attribute("value", "12"); // Não é múltiplo de 5 a partir de 10
    }
    let state = check_control_validity(&doc, input_id);
    assert!(state.step_mismatch);
    assert!(!state.valid());

    // 5. Valor perfeitamente válido
    {
        let node = doc.get_node_mut(input_id).unwrap();
        let el = node.as_element_mut().unwrap();
        el.set_attribute("value", "25");
    }
    let state = check_control_validity(&doc, input_id);
    assert!(state.valid());
}
