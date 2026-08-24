//! # Bateria de Testes da Auditoria Extrema de Engenharia (ace_dom)
//!
//! Valida seletores de atributos CSS4, pseudo-classes de estado, split_text, normalize
//! e decodificação de entidades por longest prefix.

use ace_dom::entities::decode_character_reference;
use ace_dom::node::element::Namespace;
use ace_dom::parse_html;
use ace_dom::tree::Document;

#[test]
fn test_advanced_attribute_selectors_css4() {
    let html = r#"
    <div id="container">
      <a href="https://example.com/login" title="auth login" lang="en-US" class="btn primary">Link 1</a>
      <a href="http://insecure.org/api" title="api doc" lang="en" class="btn secondary">Link 2</a>
      <a href="https://albedo.dev/download.pdf" title="download file" lang="pt-BR">Link 3</a>
    </div>
    "#;

    let doc = parse_html(html);

    // 1. Prefixo [href^="https://"]
    let https_links = doc.query_selector_all(r#"[href^="https://"]"#);
    assert_eq!(https_links.len(), 2);

    // 2. Sufixo [href$=".pdf"]
    let pdf_link = doc.query_selector(r#"[href$=".pdf"]"#);
    assert!(pdf_link.is_some());

    // 3. Substring [href*="insecure"]
    let insecure_link = doc.query_selector(r#"[href*="insecure"]"#);
    assert!(insecure_link.is_some());

    // 4. Lista separada por espaço [title~="doc"]
    let doc_link = doc.query_selector(r#"[title~="doc"]"#);
    assert!(doc_link.is_some());

    // 5. Prefixo de idioma com hífen [lang|="en"]
    let en_links = doc.query_selector_all(r#"[lang|="en"]"#);
    assert_eq!(en_links.len(), 2);
}

#[test]
fn test_state_pseudo_classes() {
    let html = r#"
    <form id="form">
      <input type="text" id="user" required>
      <input type="password" id="pass" required>
      <input type="text" id="opt">
      <input type="checkbox" id="chk" checked>
      <button id="btn_submit" disabled>Enviar</button>
      <button id="btn_reset">Limpar</button>
    </form>
    "#;

    let doc = parse_html(html);

    // :required e :optional
    assert_eq!(doc.query_selector_all("input:required").len(), 2);
    assert_eq!(doc.query_selector_all("input:optional").len(), 2);

    // :checked
    assert_eq!(doc.query_selector_all("input:checked").len(), 1);

    // :disabled e :enabled
    assert_eq!(doc.query_selector_all("button:disabled").len(), 1);
    assert_eq!(doc.query_selector_all("button:enabled").len(), 1);
}

#[test]
fn test_dom_split_text_and_normalize() {
    let mut doc = Document::new(None);
    let root = doc.root();
    let p = doc.create_element("p", Namespace::Html);
    doc.append_child(root, p).unwrap();

    let text_id = doc.create_text_node("Hello World");
    doc.append_child(p, text_id).unwrap();

    // Divide "Hello World" em "Hello " e "World" no offset 6
    let right_text_id = doc.split_text(text_id, 6).expect("split text");
    assert_eq!(doc.get_node(text_id).unwrap().text_content().unwrap(), "Hello ");
    assert_eq!(doc.get_node(right_text_id).unwrap().text_content().unwrap(), "World");

    // Ambos são filhos de <p>
    let p_children: Vec<_> = doc.children(p).collect();
    assert_eq!(p_children.len(), 2);

    // Normaliza: deve fundir os nós de texto adjacentes de volta em "Hello World"
    doc.normalize(p).expect("normalize");
    let norm_children: Vec<_> = doc.children(p).collect();
    assert_eq!(norm_children.len(), 1);
    assert_eq!(
        doc.get_node(norm_children[0].0).unwrap().text_content().unwrap(),
        "Hello World"
    );
}

#[test]
fn test_dom_replace_prepend_insert_after_and_contains() {
    let mut doc = Document::new(None);
    let root = doc.root();
    let ul = doc.create_element("ul", Namespace::Html);
    doc.append_child(root, ul).unwrap();

    let li1 = doc.create_element("li", Namespace::Html);
    let li2 = doc.create_element("li", Namespace::Html);
    let li3 = doc.create_element("li", Namespace::Html);

    doc.append_child(ul, li2).unwrap();
    doc.prepend_child(ul, li1).unwrap();
    doc.insert_after(ul, li3, li2).unwrap();

    // Ordem esperada: li1, li2, li3
    let order: Vec<_> = doc.children(ul).map(|(id, _)| id).collect();
    assert_eq!(order, vec![li1, li2, li3]);

    // replace_child: substitui li2 por new_li
    let new_li = doc.create_element("li", Namespace::Html);
    doc.replace_child(ul, new_li, li2).unwrap();

    let new_order: Vec<_> = doc.children(ul).map(|(id, _)| id).collect();
    assert_eq!(new_order, vec![li1, new_li, li3]);

    // contains
    assert!(doc.contains(ul, new_li));
    assert!(doc.contains(root, new_li));
    assert!(!doc.contains(new_li, ul));
}

#[test]
fn test_entity_longest_prefix_resolution() {
    // &copy; -> ©
    let res1 = decode_character_reference("copy;extra").unwrap();
    assert_eq!(res1.0.as_str(), "©");
    assert_eq!(res1.1, 5); // Consumiu "copy;"

    // &not; -> ¬
    let res2 = decode_character_reference("not;123").unwrap();
    assert_eq!(res2.0.as_str(), "¬");
    assert_eq!(res2.1, 4); // Consumiu "not;"

    // &notin; -> ∉
    let res3 = decode_character_reference("notin;abc").unwrap();
    assert_eq!(res3.0.as_str(), "∉");
    assert_eq!(res3.1, 6); // Consumiu "notin;"
}
