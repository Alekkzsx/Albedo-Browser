//! # Bateria de Testes do Refinamento Extremo do ace_dom (Milestone 1 & Avançados)
//!
//! Valida CompoundSelector com quotes/brackets/dots, Pseudo-classes (:nth-child, :not, :is),
//! MutationObserver com subtree ancestor walk, e DOM Range mutation APIs.

use ace_dom::node::element::Namespace;
use ace_dom::observer::{MutationObserver, MutationObserverInit, MutationRecord};
use ace_dom::parse_html;
use ace_dom::range::Range;
use ace_dom::tree::Document;

#[test]
fn test_compound_selector_with_quotes_and_brackets() {
    let html = r#"
    <body>
      <input type="text" data-version="1.0" class="form-ctrl.special" id="usr.name" />
      <div title="site.com" class="card.primary">
        <span data-info="v2.5.alpha">Detalhes</span>
      </div>
    </body>
    "#;

    let doc = parse_html(html);

    // Seletor com ponto dentro do valor do atributo
    let input_id = doc.query_selector("input[data-version=\"1.0\"]").expect("input encontrado");
    assert_eq!(doc.get_node(input_id).unwrap().tag_name().unwrap().as_str(), "input");

    let span_id = doc.query_selector("span[data-info='v2.5.alpha']").expect("span encontrado");
    assert_eq!(doc.get_node(span_id).unwrap().tag_name().unwrap().as_str(), "span");

    // Seletor com atributo case-insensitive
    let input_case = doc.query_selector("input[data-version=\"1.0\" i]").expect("case-insensitive match");
    assert_eq!(input_case, input_id);
}

#[test]
fn test_nth_child_and_not_pseudo_classes() {
    let html = r#"
    <ul>
      <li class="item">Item 1</li>
      <li class="item">Item 2</li>
      <li class="item">Item 3</li>
      <li class="item special">Item 4</li>
      <li class="item">Item 5</li>
    </ul>
    "#;

    let doc = parse_html(html);

    // :nth-child(2)
    let item2 = doc.query_selector("li:nth-child(2)").expect("item 2");
    let item2_text = doc.children(item2).next().unwrap().1.text_content().unwrap();
    assert!(item2_text.contains("Item 2"));

    // :nth-child(even) -> itens 2 e 4
    let evens = doc.query_selector_all("li:nth-child(even)");
    assert_eq!(evens.len(), 2);

    // :nth-child(odd) -> itens 1, 3 e 5
    let odds = doc.query_selector_all("li:nth-child(odd)");
    assert_eq!(odds.len(), 3);

    // :nth-child(2n+1) -> mesmos dos odds
    let odds_formula = doc.query_selector_all("li:nth-child(2n+1)");
    assert_eq!(odds_formula.len(), 3);

    // :not(.special)
    let not_special = doc.query_selector_all("li:not(.special)");
    assert_eq!(not_special.len(), 4);
}

#[test]
fn test_mutation_observer_subtree_ancestor_walk() {
    let mut doc = Document::new(None);
    let body_id = doc.create_element("body", Namespace::Html);
    let div_id = doc.create_element("div", Namespace::Html);
    let p_id = doc.create_element("p", Namespace::Html);

    doc.append_child(doc.root(), body_id).unwrap();
    doc.append_child(body_id, div_id).unwrap();
    doc.append_child(div_id, p_id).unwrap();

    let mut observer = MutationObserver::new();
    // Observa o body com subtree: true
    observer.observe(
        body_id,
        MutationObserverInit {
            child_list: true,
            subtree: true,
            ..Default::default()
        },
    );

    // Cria mutação no nível do <p> (descendente de body)
    let span_id = doc.create_element("span", Namespace::Html);
    doc.append_child(p_id, span_id).unwrap();

    let record = MutationRecord::child_list(p_id, vec![span_id], vec![], None, None);
    observer.notify_mutation_tree(&doc, record);

    let records = observer.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].target, p_id);
}

#[test]
fn test_dom_range_mutations_delete_and_extract() {
    let mut doc = Document::new(None);
    let div_id = doc.create_element("div", Namespace::Html);
    doc.append_child(doc.root(), div_id).unwrap();

    let text_id = doc.create_text_node("Hello, World!");
    doc.append_child(div_id, text_id).unwrap();

    // Range seleciona "World" (offset 7 a 12)
    let mut range = Range::new(text_id);
    range.set_start(text_id, 7);
    range.set_end(text_id, 12);

    // Extrai o conteúdo
    let frag_id = range.extract_contents(&mut doc).expect("conteúdo extraído");
    let frag_children: Vec<_> = doc.children(frag_id).collect();
    assert_eq!(frag_children.len(), 1);
    assert_eq!(frag_children[0].1.text_content().unwrap(), "World");

    // O nó de texto original deve ter "Hello, !"
    let remaining_text = doc.get_node(text_id).unwrap().text_content().unwrap();
    assert_eq!(remaining_text, "Hello, !");
}
