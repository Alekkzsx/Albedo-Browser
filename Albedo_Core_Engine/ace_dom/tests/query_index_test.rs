//! # Bateria de Testes de Índices e Consultas DOM (ace_dom)

use ace_dom::parse_html;
use ace_dom::query::ElementIndex;

#[test]
fn test_query_by_id_and_class_name() {
    let html = r#"
    <div id="header" class="nav top-bar">
      <a href="/" class="link active">Home</a>
      <a href="/about" class="link">About</a>
    </div>
    <div id="content" class="main-body">
      <p class="text">Primeiro texto</p>
      <p class="text highlight">Segundo texto</p>
    </div>
    "#;

    let doc = parse_html(html);

    // 1. Teste get_element_by_id
    let header_id = doc.get_element_by_id("header").expect("header encontrado");
    let header_el = doc.get_node(header_id).unwrap().as_element().unwrap();
    assert_eq!(header_el.tag_name.as_str(), "div");

    // 2. Teste get_elements_by_class_name
    let links = doc.get_elements_by_class_name("link");
    assert_eq!(links.len(), 2);

    let texts = doc.get_elements_by_class_name("text");
    assert_eq!(texts.len(), 2);

    let highlight = doc.get_elements_by_class_name("highlight");
    assert_eq!(highlight.len(), 1);
}

#[test]
fn test_element_index_fast_lookup() {
    let html = r#"
    <div id="card-1" class="card primary">Card 1</div>
    <div id="card-2" class="card secondary">Card 2</div>
    "#;

    let doc = parse_html(html);
    let mut index = ElementIndex::new();
    index.rebuild(&doc);

    assert!(index.get_by_id("card-1").is_some());
    assert!(index.get_by_id("card-2").is_some());
    assert!(index.get_by_id("card-3").is_none());

    let cards = index.get_by_class("card");
    assert_eq!(cards.len(), 2);
}

#[test]
fn test_query_selector_and_query_selector_all() {
    let html = r#"
    <form action="/submit">
      <input type="text" name="username" class="form-control" />
      <input type="password" name="password" class="form-control" />
      <button type="submit" id="btn-submit">Enviar</button>
    </form>
    "#;

    let doc = parse_html(html);

    // Seletor de Tag
    let form = doc.query_selector("form").expect("form encontrado");
    assert_eq!(doc.get_node(form).unwrap().tag_name().unwrap().as_str(), "form");

    // Seletor de ID
    let btn = doc.query_selector("#btn-submit").expect("btn encontrado");
    assert_eq!(doc.get_node(btn).unwrap().tag_name().unwrap().as_str(), "button");

    // Seletor de Classe (All)
    let inputs = doc.query_selector_all(".form-control");
    assert_eq!(inputs.len(), 2);

    // Seletor de Atributo
    let user_input = doc.query_selector("[name=\"username\"]").expect("input username");
    assert_eq!(doc.get_node(user_input).unwrap().tag_name().unwrap().as_str(), "input");
}
