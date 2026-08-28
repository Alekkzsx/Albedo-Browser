//! # Bateria de Testes de Fragment Parsing e Serialização HTML5 (ace_dom)

use ace_dom::node::element::Namespace;
use ace_dom::parse_fragment;
use ace_dom::tree::Document;

#[test]
fn test_parse_fragment_with_table_context() {
    let mut doc = Document::new(None);
    let table_id = doc.create_element("table", Namespace::Html);

    // Fragmento contendo apenas <tr> e <td> soltos
    let fragment_html = "<tr><td>Dado 1</td><td>Dado 2</td></tr>";
    let frag_id = parse_fragment(&mut doc, Some(table_id), fragment_html);

    // O fragmento deve conter o nó <tbody> gerado implicitamente, que contém o <tr>
    let frag_children: Vec<_> = doc.children(frag_id).collect();
    assert_eq!(frag_children.len(), 1);
    let tbody_id = frag_children[0].0;
    assert_eq!(frag_children[0].1.tag_name().unwrap().as_str(), "tbody");

    let tbody_children: Vec<_> = doc.children(tbody_id).collect();
    assert_eq!(tbody_children.len(), 1);
    assert_eq!(tbody_children[0].1.tag_name().unwrap().as_str(), "tr");
}

#[test]
fn test_html5_serializer_outer_and_inner_html() {
    let mut doc = Document::new(None);
    let div_id = doc.create_element("div", Namespace::Html);
    if let Some(el) = doc.get_node_mut(div_id).and_then(|n| n.as_element_mut()) {
        el.set_attribute("class", "card");
        el.set_attribute("data-info", "A&B \"test\"");
    }

    let p_id = doc.create_element("p", Namespace::Html);
    let text_id = doc.create_text_node("Texto com <tags> & entidades");
    let img_id = doc.create_element("img", Namespace::Html); // Void element

    doc.append_child(p_id, text_id).unwrap();
    doc.append_child(div_id, p_id).unwrap();
    doc.append_child(div_id, img_id).unwrap();

    let inner = doc.inner_html(div_id);
    assert_eq!(inner, "<p>Texto com &lt;tags&gt; &amp; entidades</p><img>");

    let outer = doc.outer_html(div_id);
    assert_eq!(
        outer,
        "<div class=\"card\" data-info=\"A&amp;B &quot;test&quot;\"><p>Texto com &lt;tags&gt; &amp; entidades</p><img></div>"
    );
}

#[test]
fn test_html5_serializer_rawtext_elements() {
    let mut doc = Document::new(None);
    let style_id = doc.create_element("style", Namespace::Html);
    let css_text = doc.create_text_node("div > p { color: red & blue; }");
    doc.append_child(style_id, css_text).unwrap();

    let outer = doc.outer_html(style_id);
    // Em RAWTEXT, os caracteres > e & não devem ser escapados
    assert_eq!(outer, "<style>div > p { color: red & blue; }</style>");
}
