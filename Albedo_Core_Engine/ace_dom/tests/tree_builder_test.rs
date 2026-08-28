//! # Bateria de Testes do Tree Builder HTML5 (ace_dom)

use ace_dom::parse_html;

#[test]
fn test_parse_complete_html_document() {
    let html = r#"<!DOCTYPE html>
<html>
  <head>
    <title>Título da Página</title>
  </head>
  <body>
    <div id="root" class="container dark">
      <h1>Olá Mundo</h1>
      <p>Este é um parágrafo no <b>Albedo</b>.</p>
    </div>
  </body>
</html>"#;

    let doc = parse_html(html);

    assert!(doc.doctype.is_some());
    assert!(doc.document_element.is_some());
    assert!(doc.head.is_some());
    assert!(doc.body.is_some());

    let root_div = doc.get_element_by_id("root").expect("id=root encontrado");
    let node = doc.get_node(root_div).expect("nó root existe");
    let el = node.as_element().expect("é elemento");

    assert_eq!(el.tag_name.as_str(), "div");
    assert!(el.has_class("container"));
    assert!(el.has_class("dark"));
    assert_eq!(el.get_attribute("id"), Some("root"));
}

#[test]
fn test_parse_implicit_structure() {
    let html = "<div>Parágrafo direto</div>";
    let doc = parse_html(html);

    // Deve ter gerado <html>, <head> e <body> automaticamente
    assert!(doc.document_element.is_some());
    assert!(doc.body.is_some());

    let body_id = doc.body.unwrap();
    let body_children: Vec<_> = doc.children(body_id).collect();
    assert!(!body_children.is_empty());

    let first_el = body_children[0].1.as_element().expect("filho é div");
    assert_eq!(first_el.tag_name.as_str(), "div");
}

#[test]
fn test_void_elements_do_not_nest() {
    let html = "<body><img src='logo.png'><input type='text'><p>Texto</p></body>";
    let doc = parse_html(html);

    let body_id = doc.body.unwrap();
    let body_children: Vec<_> = doc.children(body_id).collect();

    // img, input e p devem ser irmãos no mesmo nível dentro de body
    assert_eq!(body_children.len(), 3);
    assert_eq!(body_children[0].1.as_element().unwrap().tag_name.as_str(), "img");
    assert_eq!(body_children[1].1.as_element().unwrap().tag_name.as_str(), "input");
    assert_eq!(body_children[2].1.as_element().unwrap().tag_name.as_str(), "p");
}

#[test]
fn test_text_node_merging() {
    let html = "<div>Parte 1 &amp; Parte 2</div>";
    let doc = parse_html(html);

    let div_id = doc.query_selector("div").expect("div encontrada");
    let children: Vec<_> = doc.children(div_id).collect();

    // O texto deve ser fundido em um único nó de texto
    assert_eq!(children.len(), 1);
    let text = children[0].1.text_content().expect("tem texto");
    assert_eq!(text, "Parte 1 & Parte 2");
}
