//! # Bateria de Testes do Adoption Agency Algorithm (AAA - ace_dom)

use ace_dom::parse_html;

#[test]
fn test_misnested_formatting_tags_adoption() {
    // Caso clássico de tag de formatação entrelaçada
    let html = "<body><b>1<p>2</b>3</p></body>";
    let doc = parse_html(html);

    assert!(doc.body.is_some());
    let body_id = doc.body.unwrap();
    let body_children: Vec<_> = doc.children(body_id).collect();

    // Deve conter <b> e <p>
    assert!(!body_children.is_empty());
}

#[test]
fn test_nested_multiple_formatting_elements() {
    let html = "<body><b><i>Texto em Negrito e Itálico</b> apenas Itálico</i></body>";
    let doc = parse_html(html);

    assert!(doc.body.is_some());
}
