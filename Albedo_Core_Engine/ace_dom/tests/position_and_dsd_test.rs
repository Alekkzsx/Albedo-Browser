//! # Bateria de Testes de compareDocumentPosition e Declarative Shadow DOM (ace_dom)

use ace_dom::node::element::Namespace;
use ace_dom::node::{DocumentPosition, ShadowMode};
use ace_dom::parse_html;
use ace_dom::tree::Document;

#[test]
fn test_compare_document_position_topological_relations() {
    let mut doc = Document::new(None);
    let root = doc.root();

    let parent = doc.create_element("div", Namespace::Html);
    let child1 = doc.create_element("p", Namespace::Html);
    let child2 = doc.create_element("span", Namespace::Html);

    doc.append_child(parent, child1).unwrap();
    doc.append_child(parent, child2).unwrap();
    doc.append_child(root, parent).unwrap();

    // 1. Mesmo nó retorna 0 (vazio)
    assert_eq!(doc.compare_document_position(child1, child1), DocumentPosition::empty());

    // 2. parent em relação a child1 (parent CONTAINS e PRECEDES child1)
    let parent_rel = doc.compare_document_position(child1, parent);
    assert!(parent_rel.contains(DocumentPosition::CONTAINS));
    assert!(parent_rel.contains(DocumentPosition::PRECEDING));

    // 3. child1 em relação a parent (child1 is CONTAINED_BY e FOLLOWS parent)
    let child_rel = doc.compare_document_position(parent, child1);
    assert!(child_rel.contains(DocumentPosition::CONTAINED_BY));
    assert!(child_rel.contains(DocumentPosition::FOLLOWING));

    // 4. child2 em relação a child1 (child2 FOLLOWS child1)
    let sibling_rel = doc.compare_document_position(child1, child2);
    assert_eq!(sibling_rel, DocumentPosition::FOLLOWING);

    // 5. child1 em relação a child2 (child1 PRECEDES child2)
    let sibling_rev = doc.compare_document_position(child2, child1);
    assert_eq!(sibling_rev, DocumentPosition::PRECEDING);
}

#[test]
fn test_declarative_shadow_dom_parsing() {
    let html = r#"
    <body>
      <div id="host">
        <template shadowrootmode="open">
          <h2>Título no Shadow DOM</h2>
          <slot></slot>
        </template>
        <span>Conteúdo do Light DOM</span>
      </div>
    </body>
    "#;

    let doc = parse_html(html);
    let host_id = doc.get_element_by_id("host").expect("host element");
    let host_node = doc.get_node(host_id).unwrap();
    let el = host_node.as_element().unwrap();

    // O elemento host deve possuir uma ShadowRoot anexada diretamente pelo parser
    let shadow_id = el.shadow_root().expect("ShadowRoot anexada via DSD");
    let shadow_node = doc.get_node(shadow_id).unwrap();

    if let ace_dom::NodeKind::ShadowRoot(ref s_data) = shadow_node.kind {
        assert_eq!(s_data.mode, ShadowMode::Open);
        assert_eq!(s_data.host, host_id);
    } else {
        panic!("Nó anexado não é uma ShadowRoot");
    }

    // A ShadowRoot deve conter o <h2> e o <slot>
    let shadow_children: Vec<_> = doc.children(shadow_id).collect();
    assert!(shadow_children.len() >= 2);
}
