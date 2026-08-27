//! # Bateria de Testes Normativos de Operações de Texto (WHATWG DOM §4.6 & §4.2.4)
//!
//! Cobre exaustivamente:
//! - `Text.splitText(offset)` para nós com e sem pai (orphaned), limites 0 e len, e segurança UTF-8 char boundary.
//! - `Node.normalize()` para unificação de nós contíguos, remoção de nós vazios, preservação de links na arena e fronteiras.
//! - Integração com WebIDL bindings (`TextBindings`, `NodeBindings`) e Live Ranges.
//! - Testes de estresse e invariantes estruturais da arena.

use ace_dom::bindings::{NodeBindings, TextBindings, WebIDLException};
use ace_dom::error::DomError;
use ace_dom::node::element::Namespace;
use ace_dom::parse_html;
use ace_dom::range::{BoundaryPoint, LiveRangeHandle, LiveRangeRegistry, Range};
use ace_dom::tree::Document;
use ace_core::id::NodeId;

#[test]
fn test_split_text_middle_with_parent() {
    let mut doc = Document::new(None);
    let p_id = doc.create_element("p", Namespace::Html);
    doc.append_child(doc.root(), p_id).unwrap();

    let text_id = doc.create_text_node("HelloWorld");
    doc.append_child(p_id, text_id).unwrap();

    let new_text_id = doc.split_text(text_id, 5).expect("split_text no meio");

    // Verifica conteúdos
    let left_node = doc.get_node(text_id).unwrap();
    assert_eq!(left_node.text_content().unwrap(), "Hello");

    let right_node = doc.get_node(new_text_id).unwrap();
    assert_eq!(right_node.text_content().unwrap(), "World");

    // Verifica integridade dos ponteiros na Arena
    assert_eq!(left_node.parent, Some(p_id));
    assert_eq!(left_node.prev_sibling, None);
    assert_eq!(left_node.next_sibling, Some(new_text_id));

    assert_eq!(right_node.parent, Some(p_id));
    assert_eq!(right_node.prev_sibling, Some(text_id));
    assert_eq!(right_node.next_sibling, None);

    let parent_node = doc.get_node(p_id).unwrap();
    assert_eq!(parent_node.first_child, Some(text_id));
    assert_eq!(parent_node.last_child, Some(new_text_id));

    // Verifica contagem de filhos
    let children: Vec<NodeId> = doc.children(p_id).map(|(id, _)| id).collect();
    assert_eq!(children, vec![text_id, new_text_id]);
}

#[test]
fn test_split_text_at_zero() {
    let mut doc = Document::new(None);
    let div_id = doc.create_element("div", Namespace::Html);
    doc.append_child(doc.root(), div_id).unwrap();

    let text_id = doc.create_text_node("AlbedoEngine");
    doc.append_child(div_id, text_id).unwrap();

    let new_text_id = doc.split_text(text_id, 0).expect("split_text no offset 0");

    let left = doc.get_node(text_id).unwrap();
    assert_eq!(left.text_content().unwrap(), "");

    let right = doc.get_node(new_text_id).unwrap();
    assert_eq!(right.text_content().unwrap(), "AlbedoEngine");

    assert_eq!(left.next_sibling, Some(new_text_id));
    assert_eq!(right.prev_sibling, Some(text_id));
}

#[test]
fn test_split_text_at_length() {
    let mut doc = Document::new(None);
    let div_id = doc.create_element("div", Namespace::Html);
    doc.append_child(doc.root(), div_id).unwrap();

    let text_id = doc.create_text_node("AlbedoEngine");
    doc.append_child(div_id, text_id).unwrap();

    let len = "AlbedoEngine".len();
    let new_text_id = doc.split_text(text_id, len).expect("split_text no offset len");

    let left = doc.get_node(text_id).unwrap();
    assert_eq!(left.text_content().unwrap(), "AlbedoEngine");

    let right = doc.get_node(new_text_id).unwrap();
    assert_eq!(right.text_content().unwrap(), "");

    assert_eq!(left.next_sibling, Some(new_text_id));
    assert_eq!(right.prev_sibling, Some(text_id));
    assert_eq!(doc.get_node(div_id).unwrap().last_child, Some(new_text_id));
}

#[test]
fn test_split_text_orphaned_node_without_parent() {
    let mut doc = Document::new(None);
    // Cria nó de texto isolado sem anexar a nenhum pai
    let orphan_id = doc.create_text_node("StandaloneText");

    let new_id = doc.split_text(orphan_id, 10).expect("split_text em orfao");

    let orig = doc.get_node(orphan_id).unwrap();
    assert_eq!(orig.text_content().unwrap(), "Standalone");
    assert_eq!(orig.parent, None);
    assert_eq!(orig.prev_sibling, None);
    assert_eq!(orig.next_sibling, None);

    let new_n = doc.get_node(new_id).unwrap();
    assert_eq!(new_n.text_content().unwrap(), "Text");
    assert_eq!(new_n.parent, None);
    assert_eq!(new_n.prev_sibling, None);
    assert_eq!(new_n.next_sibling, None);
}

#[test]
fn test_split_text_orphaned_node_at_zero_and_len() {
    let mut doc = Document::new(None);

    let orphan1 = doc.create_text_node("Alpha");
    let new1 = doc.split_text(orphan1, 0).unwrap();
    assert_eq!(doc.get_node(orphan1).unwrap().text_content().unwrap(), "");
    assert_eq!(doc.get_node(new1).unwrap().text_content().unwrap(), "Alpha");

    let orphan2 = doc.create_text_node("Beta");
    let new2 = doc.split_text(orphan2, 4).unwrap();
    assert_eq!(doc.get_node(orphan2).unwrap().text_content().unwrap(), "Beta");
    assert_eq!(doc.get_node(new2).unwrap().text_content().unwrap(), "");
}

#[test]
fn test_split_text_out_of_bounds_error() {
    let mut doc = Document::new(None);
    let text_id = doc.create_text_node("Short");

    let res = doc.split_text(text_id, 10);
    assert_eq!(res, Err(DomError::IndexSizeError));
}

#[test]
fn test_split_text_utf8_multibyte_boundary_safety() {
    let mut doc = Document::new(None);
    // "Olá 🦀 Mundo 🚀!" -> 'á' tem 2 bytes, '🦀' tem 4 bytes, '🚀' tem 4 bytes
    let text_content = "Olá 🦀 Mundo 🚀!";
    let text_id = doc.create_text_node(text_content);

    // Divisão em limites válidos de caracteres UTF-8
    // "Olá " -> offset 5 (O=1, l=1, á=2, ' '=1 -> 5 bytes)
    assert!(text_content.is_char_boundary(5));
    let right_id = doc.split_text(text_id, 5).expect("split_text em char boundary valido");
    assert_eq!(doc.get_node(text_id).unwrap().text_content().unwrap(), "Olá ");
    assert_eq!(doc.get_node(right_id).unwrap().text_content().unwrap(), "🦀 Mundo 🚀!");

    // Agora tenta dividir '🦀' (4 bytes: de 0 a 4 no right_id) no meio (offset 1, 2 ou 3)
    let err1 = doc.split_text(right_id, 1);
    assert_eq!(err1, Err(DomError::IndexSizeError));

    let err2 = doc.split_text(right_id, 2);
    assert_eq!(err2, Err(DomError::IndexSizeError));

    let err3 = doc.split_text(right_id, 3);
    assert_eq!(err3, Err(DomError::IndexSizeError));

    // No offset 4 (logo após o emoji do caranguejo '🦀') deve funcionar perfeitamente
    let after_crab = doc.split_text(right_id, 4).expect("split_text apos o emoji 🦀");
    assert_eq!(doc.get_node(right_id).unwrap().text_content().unwrap(), "🦀");
    assert_eq!(doc.get_node(after_crab).unwrap().text_content().unwrap(), " Mundo 🚀!");
}

#[test]
fn test_split_text_non_text_node_and_invalid_id() {
    let mut doc = Document::new(None);
    let el_id = doc.create_element("span", Namespace::Html);
    let comment_id = doc.create_comment("comentario");

    // Tentativa em Elemento
    let res1 = doc.split_text(el_id, 0);
    assert!(matches!(res1, Err(DomError::HierarchyRequestError(_))));

    // Tentativa em Comentário
    let res2 = doc.split_text(comment_id, 0);
    assert!(matches!(res2, Err(DomError::HierarchyRequestError(_))));

    // Tentativa em NodeId inexistente
    let fake_id = NodeId::new();
    let res3 = doc.split_text(fake_id, 0);
    assert_eq!(res3, Err(DomError::InvalidNodeId(fake_id)));
}

#[test]
fn test_split_text_consecutive_multi_splits() {
    let mut doc = Document::new(None);
    let container = doc.create_element("div", Namespace::Html);
    doc.append_child(doc.root(), container).unwrap();

    let t1 = doc.create_text_node("ABCDEFGHIJ");
    doc.append_child(container, t1).unwrap();

    // Divide t1 ("ABCDEFGHIJ") em 3 -> t1="ABC", t2="DEFGHIJ"
    let t2 = doc.split_text(t1, 3).unwrap();
    // Divide t2 ("DEFGHIJ") em 3 -> t2="DEF", t3="GHIJ"
    let t3 = doc.split_text(t2, 3).unwrap();
    // Divide t3 ("GHIJ") em 2 -> t3="GH", t4="IJ"
    let t4 = doc.split_text(t3, 2).unwrap();

    assert_eq!(doc.get_node(t1).unwrap().text_content().unwrap(), "ABC");
    assert_eq!(doc.get_node(t2).unwrap().text_content().unwrap(), "DEF");
    assert_eq!(doc.get_node(t3).unwrap().text_content().unwrap(), "GH");
    assert_eq!(doc.get_node(t4).unwrap().text_content().unwrap(), "IJ");

    // Verifica encadeamento completo de nós irmãos
    assert_eq!(doc.get_node(t1).unwrap().next_sibling, Some(t2));
    assert_eq!(doc.get_node(t2).unwrap().prev_sibling, Some(t1));
    assert_eq!(doc.get_node(t2).unwrap().next_sibling, Some(t3));
    assert_eq!(doc.get_node(t3).unwrap().prev_sibling, Some(t2));
    assert_eq!(doc.get_node(t3).unwrap().next_sibling, Some(t4));
    assert_eq!(doc.get_node(t4).unwrap().prev_sibling, Some(t3));
    assert_eq!(doc.get_node(t4).unwrap().next_sibling, None);

    assert_eq!(doc.get_node(container).unwrap().first_child, Some(t1));
    assert_eq!(doc.get_node(container).unwrap().last_child, Some(t4));
}

#[test]
fn test_normalize_consecutive_text_nodes() {
    let mut doc = Document::new(None);
    let p_id = doc.create_element("p", Namespace::Html);
    doc.append_child(doc.root(), p_id).unwrap();

    let t1 = doc.create_text_node("Hello");
    let t2 = doc.create_text_node(" ");
    let t3 = doc.create_text_node("Beautiful");
    let t4 = doc.create_text_node(" ");
    let t5 = doc.create_text_node("World");

    doc.append_child(p_id, t1).unwrap();
    doc.append_child(p_id, t2).unwrap();
    doc.append_child(p_id, t3).unwrap();
    doc.append_child(p_id, t4).unwrap();
    doc.append_child(p_id, t5).unwrap();

    assert_eq!(doc.children(p_id).count(), 5);

    doc.normalize(p_id).expect("normalize com sucesso");

    // Deve restar apenas 1 filho contendo todo o texto concatenado
    let children: Vec<NodeId> = doc.children(p_id).map(|(id, _)| id).collect();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0], t1);

    let unified_node = doc.get_node(t1).unwrap();
    assert_eq!(unified_node.text_content().unwrap(), "Hello Beautiful World");
    assert_eq!(unified_node.prev_sibling, None);
    assert_eq!(unified_node.next_sibling, None);
    assert_eq!(doc.get_node(p_id).unwrap().first_child, Some(t1));
    assert_eq!(doc.get_node(p_id).unwrap().last_child, Some(t1));
}

#[test]
fn test_normalize_empty_text_nodes_removal() {
    let mut doc = Document::new(None);
    let p_id = doc.create_element("p", Namespace::Html);
    doc.append_child(doc.root(), p_id).unwrap();

    let e1 = doc.create_text_node("");
    let t1 = doc.create_text_node("First");
    let e2 = doc.create_text_node("");
    let t2 = doc.create_text_node("Second");
    let e3 = doc.create_text_node("");

    doc.append_child(p_id, e1).unwrap();
    doc.append_child(p_id, t1).unwrap();
    doc.append_child(p_id, e2).unwrap();
    doc.append_child(p_id, t2).unwrap();
    doc.append_child(p_id, e3).unwrap();

    doc.normalize(p_id).unwrap();

    let children: Vec<NodeId> = doc.children(p_id).map(|(id, _)| id).collect();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0], t1);
    assert_eq!(doc.get_node(t1).unwrap().text_content().unwrap(), "FirstSecond");
}

#[test]
fn test_normalize_all_empty_text_nodes() {
    let mut doc = Document::new(None);
    let p_id = doc.create_element("p", Namespace::Html);
    doc.append_child(doc.root(), p_id).unwrap();

    let e1 = doc.create_text_node("");
    let e2 = doc.create_text_node("");
    let e3 = doc.create_text_node("");

    doc.append_child(p_id, e1).unwrap();
    doc.append_child(p_id, e2).unwrap();
    doc.append_child(p_id, e3).unwrap();

    doc.normalize(p_id).unwrap();

    assert_eq!(doc.children(p_id).count(), 0);
    let parent = doc.get_node(p_id).unwrap();
    assert_eq!(parent.first_child, None);
    assert_eq!(parent.last_child, None);
}

#[test]
fn test_normalize_mixed_elements_and_comments_boundaries() {
    let mut doc = Document::new(None);
    let container = doc.create_element("div", Namespace::Html);
    doc.append_child(doc.root(), container).unwrap();

    let t1 = doc.create_text_node("Part1");
    let t2 = doc.create_text_node("Part2");
    let comment = doc.create_comment("divisor");
    let t3 = doc.create_text_node("Part3");
    let t4 = doc.create_text_node("Part4");
    let span = doc.create_element("span", Namespace::Html);
    let t5 = doc.create_text_node("Part5");

    doc.append_child(container, t1).unwrap();
    doc.append_child(container, t2).unwrap();
    doc.append_child(container, comment).unwrap();
    doc.append_child(container, t3).unwrap();
    doc.append_child(container, t4).unwrap();
    doc.append_child(container, span).unwrap();
    doc.append_child(container, t5).unwrap();

    doc.normalize(container).unwrap();

    let children: Vec<NodeId> = doc.children(container).map(|(id, _)| id).collect();
    // Esperado: [t1 ("Part1Part2"), comment, t3 ("Part3Part4"), span, t5 ("Part5")]
    assert_eq!(children, vec![t1, comment, t3, span, t5]);

    assert_eq!(doc.get_node(t1).unwrap().text_content().unwrap(), "Part1Part2");
    assert_eq!(doc.get_node(t3).unwrap().text_content().unwrap(), "Part3Part4");
    assert_eq!(doc.get_node(t5).unwrap().text_content().unwrap(), "Part5");
}

#[test]
fn test_normalize_deeply_nested_tree() {
    let mut doc = Document::new(None);
    let div = doc.create_element("div", Namespace::Html);
    let p = doc.create_element("p", Namespace::Html);
    let b = doc.create_element("b", Namespace::Html);

    doc.append_child(doc.root(), div).unwrap();
    doc.append_child(div, p).unwrap();
    doc.append_child(p, b).unwrap();

    // Adiciona nós adjacentes no div
    let d1 = doc.create_text_node("D1");
    let d2 = doc.create_text_node("D2");
    doc.prepend_child(div, d2).unwrap();
    doc.prepend_child(div, d1).unwrap();

    // Adiciona nós adjacentes no b
    let b1 = doc.create_text_node("B1");
    let b2 = doc.create_text_node("B2");
    doc.append_child(b, b1).unwrap();
    doc.append_child(b, b2).unwrap();

    // Normaliza a partir da raiz
    doc.normalize(doc.root()).unwrap();

    assert_eq!(doc.get_node(d1).unwrap().text_content().unwrap(), "D1D2");
    assert_eq!(doc.get_node(b1).unwrap().text_content().unwrap(), "B1B2");
    assert_eq!(doc.children(b).count(), 1);
}

#[test]
fn test_normalize_idempotency() {
    let mut doc = Document::new(None);
    let p = doc.create_element("p", Namespace::Html);
    doc.append_child(doc.root(), p).unwrap();

    let t1 = doc.create_text_node("Test");
    let t2 = doc.create_text_node(" idempotency");
    doc.append_child(p, t1).unwrap();
    doc.append_child(p, t2).unwrap();

    for _ in 0..5 {
        doc.normalize(doc.root()).unwrap();
        assert_eq!(doc.children(p).count(), 1);
        assert_eq!(doc.get_node(t1).unwrap().text_content().unwrap(), "Test idempotency");
    }
}

#[test]
fn test_normalize_text_node_and_invalid_node() {
    let mut doc = Document::new(None);
    let text_id = doc.create_text_node("Leaf");

    // Normalizar um nó de texto não deve falhar
    assert!(doc.normalize(text_id).is_ok());

    // Normalizar ID inexistente retorna erro
    let fake_id = NodeId::new();
    assert_eq!(doc.normalize(fake_id), Err(DomError::InvalidNodeId(fake_id)));
}

#[test]
fn test_webidl_text_and_node_bindings() {
    let mut doc = parse_html(r#"<div id="container">FirstSecond</div>"#);
    let container = doc.get_element_by_id("container").unwrap();
    let text_node = doc.first_child(container).unwrap();

    // Valida getters de TextBindings
    assert_eq!(TextBindings::get_data(&doc, text_node).as_deref(), Some("FirstSecond"));
    assert_eq!(TextBindings::get_length(&doc, text_node), Some(11));

    // Executa splitText via WebIDL
    let new_text = TextBindings::split_text(&mut doc, text_node, 5).expect("split_text via WebIDL");
    assert_eq!(TextBindings::get_data(&doc, text_node).as_deref(), Some("First"));
    assert_eq!(TextBindings::get_data(&doc, new_text).as_deref(), Some("Second"));

    // Tenta splitText fora dos limites via WebIDL
    let err = TextBindings::split_text(&mut doc, text_node, 100);
    assert!(matches!(err, Err(WebIDLException::TypeError(_))));

    // Normaliza via NodeBindings
    NodeBindings::normalize(&mut doc, container).expect("normalize via WebIDL");
    assert_eq!(doc.children(container).count(), 1);
    assert_eq!(TextBindings::get_data(&doc, text_node).as_deref(), Some("FirstSecond"));
}

#[test]
fn test_live_range_integration_with_split_and_normalize() {
    let mut doc = parse_html(r#"<div id="box"><p id="txt">Engineering Next-Gen Browser</p></div>"#);
    let txt = doc.get_element_by_id("txt").unwrap();
    let text_node = doc.first_child(txt).unwrap();

    // Range 1: cobrindo "Next-Gen" (offset 13 a 21: estritamente após o split no offset 12)
    let range1 = Range::from_points(
        BoundaryPoint::new(text_node, 13),
        BoundaryPoint::new(text_node, 21),
    );
    let handle1 = LiveRangeHandle::new(range1);

    // Range 2: cobrindo "Engineering" (offset 0 a 11: antes do split no offset 12)
    let range2 = Range::from_points(
        BoundaryPoint::new(text_node, 0),
        BoundaryPoint::new(text_node, 11),
    );
    let handle2 = LiveRangeHandle::new(range2);

    let mut registry = LiveRangeRegistry::new();
    registry.register(&handle1);
    registry.register(&handle2);

    // Divide em "Engineering " (offset 12)
    let right_node = doc.split_text(text_node, 12).unwrap();
    registry.notify_split_text(text_node, right_node, 12);

    // Range 1: deve mover para right_node com offsets deslocados em 12
    let r1_after = handle1.get_range();
    assert_eq!(r1_after.start.node, right_node);
    assert_eq!(r1_after.start.offset, 1); // 13 - 12 = 1
    assert_eq!(r1_after.end.node, right_node);
    assert_eq!(r1_after.end.offset, 9); // 21 - 12 = 9

    // Range 2: deve permanecer inalterado no nó original
    let r2_after = handle2.get_range();
    assert_eq!(r2_after.start.node, text_node);
    assert_eq!(r2_after.start.offset, 0);
    assert_eq!(r2_after.end.node, text_node);
    assert_eq!(r2_after.end.offset, 11);
}

#[test]
fn test_stress_split_and_normalize_cycles() {
    let mut doc = Document::new(None);
    let container = doc.create_element("div", Namespace::Html);
    doc.append_child(doc.root(), container).unwrap();

    let initial_text = "TheQuickBrownFoxJumpsOverTheLazyDog";
    let base_text_id = doc.create_text_node(initial_text);
    doc.append_child(container, base_text_id).unwrap();

    // Realiza 100 ciclos de split em múltiplos pontos e posterior normalize
    for _ in 0..50 {
        // Divide em 3 pedaços
        let p2 = doc.split_text(base_text_id, 10).unwrap();
        let _p3 = doc.split_text(p2, 10).unwrap();

        assert_eq!(doc.children(container).count(), 3);

        // Normaliza de volta
        doc.normalize(container).unwrap();

        assert_eq!(doc.children(container).count(), 1);
        let current_text = doc.get_node(base_text_id).unwrap().text_content().unwrap();
        assert_eq!(current_text, initial_text);
    }
}
