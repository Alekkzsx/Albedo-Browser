//! # Bateria Avançada de Testes do ace_dom (Hardening Suite)
//!
//! Valida RAWTEXT, RCDATA, ScriptData, Tabelas, Foster Parenting, classList, Shadow DOM, clone_node e Combinadores CSS.

use ace_dom::node::element::Namespace;
use ace_dom::node::ShadowMode;
use ace_dom::parse_html;
use ace_dom::tree::Document;

#[test]
fn test_script_and_style_rawtext_parsing() {
    let html = r#"
    <head>
      <title>Meu &amp; Título</title>
      <style>
        div > p { color: red; content: "<span>não é tag</span>"; }
      </style>
    </head>
    <body>
      <script>
        if (a < 10 && b > 20) {
          console.log("<div id='fake'></div>");
        }
      </script>
    </body>
    "#;

    let doc = parse_html(html);

    // O título deve ter a entidade &amp; resolvida (RCDATA)
    let title_id = doc.query_selector("title").expect("title encontrado");
    let title_text = doc.children(title_id).next().unwrap().1.text_content().unwrap();
    assert_eq!(title_text, "Meu & Título");

    // O style deve conter o CSS bruto sem tags filhas
    let style_id = doc.query_selector("style").expect("style encontrado");
    let style_children: Vec<_> = doc.children(style_id).collect();
    assert_eq!(style_children.len(), 1);
    let style_text = style_children[0].1.text_content().unwrap();
    assert!(style_text.contains("div > p"));
    assert!(style_text.contains("<span>não é tag</span>"));

    // O script deve conter o JS bruto sem interpretar `<div>`
    let script_id = doc.query_selector("script").expect("script encontrado");
    let script_children: Vec<_> = doc.children(script_id).collect();
    assert_eq!(script_children.len(), 1);
    let script_text = script_children[0].1.text_content().unwrap();
    assert!(script_text.contains("if (a < 10 && b > 20)"));

    // Não deve haver nenhum elemento <div id="fake"> criado na árvore
    assert!(doc.get_element_by_id("fake").is_none());
}

#[test]
fn test_table_parsing_and_foster_parenting() {
    let html = r#"
    <body>
      <table id="my-table">
        <div id="stray">Elemento Fora de Lugar</div>
        <tr>
          <td>Célula 1</td>
          <td>Célula 2</td>
        </tr>
      </table>
    </body>
    "#;

    let doc = parse_html(html);
    let table_id = doc.get_element_by_id("my-table").expect("tabela encontrada");
    let stray_id = doc.get_element_by_id("stray").expect("stray encontrado");

    // Foster parenting: o elemento stray deve ter sido inserido antes da tabela
    let stray_node = doc.get_node(stray_id).unwrap();
    let table_node = doc.get_node(table_id).unwrap();

    assert_eq!(stray_node.parent, table_node.parent);
    assert_eq!(stray_node.next_sibling, Some(table_id));

    // A tabela deve ter gerado <tbody> e <tr> com <td>
    let cells = doc.query_selector_all("td");
    assert_eq!(cells.len(), 2);
}

#[test]
fn test_dom_token_list_class_list_mutations() {
    let mut doc = Document::new(None);
    let div_id = doc.create_element("div", Namespace::Html);

    // Adiciona classes
    {
        let div_node = doc.get_node_mut(div_id).unwrap();
        let el = div_node.as_element_mut().unwrap();
        let mut list = el.class_list();
        list.add("btn");
        list.add("btn-primary");
        list.add("active");
        assert!(list.contains("btn"));
        assert!(list.contains("btn-primary"));
        assert_eq!(list.value(), "btn btn-primary active");
    }

    // Toggle e Replace
    {
        let div_node = doc.get_node_mut(div_id).unwrap();
        let el = div_node.as_element_mut().unwrap();
        let mut list = el.class_list();
        assert!(!list.toggle("active")); // Remove active
        assert!(!list.contains("active"));
        assert!(list.replace("btn-primary", "btn-secondary"));
        assert!(list.contains("btn-secondary"));
        assert!(!list.contains("btn-primary"));
    }
}

#[test]
fn test_shadow_dom_attachment_and_lookup() {
    let mut doc = Document::new(None);
    let host_id = doc.create_element("custom-element", Namespace::Html);

    let shadow_id = doc.attach_shadow(host_id, ShadowMode::Open).expect("shadow anexada");
    assert_eq!(doc.get_shadow_root(host_id), Some(shadow_id));

    // Não deve permitir anexar duas shadow roots no mesmo hospedeiro
    assert!(doc.attach_shadow(host_id, ShadowMode::Closed).is_err());
}

#[test]
fn test_clone_node_deep_and_shallow() {
    let mut doc = Document::new(None);
    let parent = doc.create_element("div", Namespace::Html);
    let child1 = doc.create_element("p", Namespace::Html);
    let child2 = doc.create_text_node("Texto");

    doc.append_child(parent, child1).unwrap();
    doc.append_child(parent, child2).unwrap();

    // Clone Shallow (apenas o pai)
    let shallow = doc.clone_node(parent, false).unwrap();
    assert_eq!(doc.children(shallow).count(), 0);

    // Clone Deep (pai e todos os filhos recursivos)
    let deep = doc.clone_node(parent, true).unwrap();
    let deep_children: Vec<_> = doc.children(deep).collect();
    assert_eq!(deep_children.len(), 2);
    assert_eq!(deep_children[0].1.tag_name().unwrap().as_str(), "p");
    assert_eq!(deep_children[1].1.text_content().unwrap(), "Texto");
}

#[test]
fn test_complex_selectors_with_combinators() {
    let html = r#"
    <div id="container">
      <div class="card">
        <h1 class="title">Título</h1>
        <p class="desc">Descrição imediata</p>
        <span class="badge">Badge</span>
      </div>
    </div>
    "#;

    let doc = parse_html(html);

    // Filho direto `>`
    let desc = doc.query_selector(".card > .desc").expect("desc encontrada via >");
    assert_eq!(doc.get_node(desc).unwrap().tag_name().unwrap().as_str(), "p");

    // Irmão adjacente `+`
    let desc_adj = doc.query_selector("h1 + p").expect("p encontrado via +");
    assert_eq!(desc, desc_adj);

    // Descendente geral (espaço)
    let badge = doc.query_selector("#container .badge").expect("badge encontrada via espaço");
    assert_eq!(doc.get_node(badge).unwrap().tag_name().unwrap().as_str(), "span");

    // Pseudo-classe :first-child
    let first = doc.query_selector("h1:first-child").expect("h1 é first-child");
    assert_eq!(doc.get_node(first).unwrap().tag_name().unwrap().as_str(), "h1");
}
