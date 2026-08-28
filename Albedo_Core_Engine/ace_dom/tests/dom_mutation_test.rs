//! # Bateria de Testes de Mutações e Navegação na Árvore DOM (ace_dom)

use ace_dom::node::element::Namespace;
use ace_dom::tree::Document;

#[test]
fn test_manual_dom_construction_and_traversal() {
    let mut doc = Document::new(Some("https://albedo.org"));

    let html_id = doc.create_element("html", Namespace::Html);
    let body_id = doc.create_element("body", Namespace::Html);
    let div_id = doc.create_element("div", Namespace::Html);
    let span1_id = doc.create_element("span", Namespace::Html);
    let span2_id = doc.create_element("span", Namespace::Html);

    doc.append_child(doc.root(), html_id).unwrap();
    doc.append_child(html_id, body_id).unwrap();
    doc.append_child(body_id, div_id).unwrap();
    doc.append_child(div_id, span1_id).unwrap();
    doc.append_child(div_id, span2_id).unwrap();

    // Testa iterador de filhos de <div>
    let div_children: Vec<_> = doc.children(div_id).collect();
    assert_eq!(div_children.len(), 2);
    assert_eq!(div_children[0].0, span1_id);
    assert_eq!(div_children[1].0, span2_id);

    // Testa iterador de ancestrais de <span>
    let span_ancestors: Vec<_> = doc.ancestors(span1_id).collect();
    assert_eq!(span_ancestors.len(), 4); // div, body, html, root
    assert_eq!(span_ancestors[0].0, div_id);
    assert_eq!(span_ancestors[1].0, body_id);
    assert_eq!(span_ancestors[2].0, html_id);
    assert_eq!(span_ancestors[3].0, doc.root());

    // Testa remoção de filho
    doc.remove_child(div_id, span1_id).unwrap();
    let div_children_after: Vec<_> = doc.children(div_id).collect();
    assert_eq!(div_children_after.len(), 1);
    assert_eq!(div_children_after[0].0, span2_id);
}

#[test]
fn test_insert_before_mutation() {
    let mut doc = Document::new(None);
    let parent = doc.create_element("ul", Namespace::Html);
    let li2 = doc.create_element("li", Namespace::Html);
    let li1 = doc.create_element("li", Namespace::Html);

    doc.append_child(doc.root(), parent).unwrap();
    doc.append_child(parent, li2).unwrap();
    doc.insert_before(parent, li1, Some(li2)).unwrap();

    let children: Vec<_> = doc.children(parent).collect();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].0, li1);
    assert_eq!(children[1].0, li2);
}
