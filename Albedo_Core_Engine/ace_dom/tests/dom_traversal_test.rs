//! # Bateria de Testes de Travessia DOM (TreeWalker & NodeIterator)

use ace_dom::node::element::Namespace;
use ace_dom::traversal::{NodeFilter, NodeIterator, TreeWalker};
use ace_dom::tree::Document;

#[test]
fn test_tree_walker_element_traversal() {
    let mut doc = Document::new(None);
    let root = doc.create_element("div", Namespace::Html);
    let c1 = doc.create_element("p", Namespace::Html);
    let text1 = doc.create_text_node("Texto ignorado pelo walker");
    let c2 = doc.create_element("span", Namespace::Html);

    doc.append_child(root, c1).unwrap();
    doc.append_child(root, text1).unwrap();
    doc.append_child(root, c2).unwrap();

    let mut walker = TreeWalker::new(root, NodeFilter::SHOW_ELEMENT);

    // Primeiro filho elemento deve ser c1 (pulando texto)
    let first = walker.first_child(&doc);
    assert_eq!(first, Some(c1));

    // Próximo elemento em ordem DFS
    let next = walker.next_node(&doc);
    assert_eq!(next, Some(c2));
}

#[test]
fn test_node_iterator_traversal() {
    let mut doc = Document::new(None);
    let root = doc.create_element("div", Namespace::Html);
    let p1 = doc.create_element("p", Namespace::Html);
    let p2 = doc.create_element("p", Namespace::Html);

    doc.append_child(root, p1).unwrap();
    doc.append_child(root, p2).unwrap();

    let mut iter = NodeIterator::new(root, NodeFilter::SHOW_ELEMENT);

    assert_eq!(iter.next_node(&doc), Some(root));
    assert_eq!(iter.next_node(&doc), Some(p1));
    assert_eq!(iter.next_node(&doc), Some(p2));
    assert_eq!(iter.next_node(&doc), None);
}
