//! # Bateria de Testes de Formulários, ValidityState e Custom Elements (ace_dom)

use ace_dom::custom_elements::{is_valid_custom_element_name, CustomElementRegistry};
use ace_dom::form::{check_control_validity, FormData, FormDataValue};
use ace_dom::node::element::Namespace;
use ace_dom::tree::Document;

#[test]
fn test_form_control_validity_state() {
    let mut doc = Document::new(None);
    let input_id = doc.create_element("input", Namespace::Html);

    if let Some(el) = doc.get_node_mut(input_id).and_then(|n| n.as_element_mut()) {
        el.set_attribute("type", "email");
        el.set_attribute("required", "");
        el.set_attribute("value", "email-invalido-sem-arroba");
    }

    let validity = check_control_validity(&doc, input_id);
    assert!(!validity.valid());
    assert!(validity.type_mismatch);
    assert!(!validity.value_missing);

    // Corrige para e-mail válido
    if let Some(el) = doc.get_node_mut(input_id).and_then(|n| n.as_element_mut()) {
        el.set_attribute("value", "usuario@albedo.dev");
    }

    let validity_ok = check_control_validity(&doc, input_id);
    assert!(validity_ok.valid());
}

#[test]
fn test_form_data_serialization() {
    let mut fd = FormData::new();
    fd.append("username", "albedo_user");
    fd.append("search query", "rust browser engine");
    fd.append_file("avatar", "profile.png", "image/png", vec![0x89, 0x50, 0x4E, 0x47]);

    assert_eq!(fd.len(), 3);
    assert_eq!(fd.get("username").and_then(|v| v.as_text()), Some("albedo_user"));

    // Valida codificação url-encoded
    let encoded = fd.to_url_encoded();
    assert_eq!(encoded, "username=albedo_user&search+query=rust+browser+engine");

    // Valida montagem multipart
    let multipart = fd.to_multipart("---BOUND123");
    let multipart_str = String::from_utf8_lossy(&multipart);
    assert!(multipart_str.contains("Content-Disposition: form-data; name=\"username\""));
    assert!(multipart_str.contains("filename=\"profile.png\""));
}

#[test]
fn test_custom_element_name_validation_and_registry() {
    // Nomes válidos com hífen
    assert!(is_valid_custom_element_name("user-profile"));
    assert!(is_valid_custom_element_name("custom-tab-panel"));

    // Nomes inválidos (sem hífen ou reservados)
    assert!(!is_valid_custom_element_name("div"));
    assert!(!is_valid_custom_element_name("user"));
    assert!(!is_valid_custom_element_name("font-face"));

    let mut registry = CustomElementRegistry::new();
    let res = registry.define("user-card", &["data-user", "theme"]);
    assert!(res.is_ok());

    let def = registry.get("user-card").expect("user-card registrado");
    assert_eq!(def.name.as_str(), "user-card");
    assert_eq!(def.observed_attributes.len(), 2);
}
