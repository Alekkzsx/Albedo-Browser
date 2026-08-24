//! # Bateria de Testes Supremos do ace_dom (Extreme Hardening Suite)
//!
//! Valida Arca de Noé, Tags <template>, Foreign Content SVG/MathML, Slot Assignment e Fast-Path SIMD.

use ace_dom::node::element::Namespace;
use ace_dom::node::ShadowMode;
use ace_dom::parse_html;
use ace_dom::tree::Document;

#[test]
fn test_noahs_ark_clause_formatting_limits() {
    let mut doc = Document::new(None);
    let mut active = ace_dom::ActiveFormattingElements::new();

    // Cria 6 elementos <b> idênticos com o mesmo atributo class="bold"
    for _ in 0..6 {
        let el_id = doc.create_element("b", Namespace::Html);
        if let Some(el) = doc.get_node_mut(el_id).and_then(|n| n.as_element_mut()) {
            el.set_attribute("class", "bold");
        }
        active.push_element(&doc, el_id);
    }

    // A Arca de Noé deve limitar a exatamente 3 elementos idênticos antes de um marcador
    assert_eq!(active.len(), 3);
}

#[test]
fn test_template_element_inactive_fragment_isolation() {
    let html = r#"
    <body>
      <div id="main">Conteúdo Principal</div>
      <template id="my-template">
        <div class="template-item">Item 1</div>
        <p>Parágrafo do template</p>
      </template>
    </body>
    "#;

    let doc = parse_html(html);
    let template_id = doc.get_element_by_id("my-template").expect("template encontrado");
    let template_node = doc.get_node(template_id).unwrap();
    let el = template_node.as_element().unwrap();

    // Os filhos da tag <template> NÃO devem aparecer como filhos diretos do nó template no DOM visível
    assert_eq!(doc.children(template_id).count(), 0);

    // O fragmento isolado deve existir em template_content e conter os 2 nós filhos
    let frag_id = el.template_content.expect("template_content presente");
    let frag_children: Vec<_> = doc.children(frag_id).collect();
    assert!(frag_children.len() >= 2);

    // O seletor no documento principal não deve encontrar elementos inertes do template
    assert!(doc.query_selector(".template-item").is_none());
}

#[test]
fn test_foreign_content_svg_case_fixups() {
    let html = r#"
    <body>
      <svg viewbox="0 0 100 100" clippathunits="userSpaceOnUse">
        <lineargradient id="grad1">
          <foreignobject width="50" height="50">
            <div>HTML dentro de SVG</div>
          </foreignobject>
        </lineargradient>
      </svg>
    </body>
    "#;

    let doc = parse_html(html);

    // Verifica se a tag <linearGradient> e <foreignObject> foram corrigidas para CamelCase
    let grad_id = doc.query_selector("linearGradient").expect("linearGradient encontrado via CamelCase");
    assert_eq!(doc.get_node(grad_id).unwrap().tag_name().unwrap().as_str(), "linearGradient");

    let fo_id = doc.query_selector("foreignObject").expect("foreignObject encontrado");
    assert_eq!(doc.get_node(fo_id).unwrap().tag_name().unwrap().as_str(), "foreignObject");

    // Verifica atributos corrigidos
    let svg_id = doc.query_selector("svg").expect("svg encontrado");
    let svg_el = doc.get_node(svg_id).unwrap().as_element().unwrap();
    assert!(svg_el.get_attribute("viewBox").is_some());
    assert!(svg_el.get_attribute("clipPathUnits").is_some());
}

#[test]
fn test_shadow_dom_slot_assignment_and_flat_tree() {
    let mut doc = Document::new(None);
    let host_id = doc.create_element("user-card", Namespace::Html);

    // Filhos do Light DOM (slotables)
    let named_child = doc.create_element("span", Namespace::Html);
    if let Some(el) = doc.get_node_mut(named_child).and_then(|n| n.as_element_mut()) {
        el.set_attribute("slot", "username");
    }
    let default_child = doc.create_element("p", Namespace::Html);

    doc.append_child(host_id, named_child).unwrap();
    doc.append_child(host_id, default_child).unwrap();

    // Cria Shadow DOM
    let shadow_id = doc.attach_shadow(host_id, ShadowMode::Open).unwrap();

    // Slots no Shadow DOM
    let slot_named = doc.create_element("slot", Namespace::Html);
    if let Some(el) = doc.get_node_mut(slot_named).and_then(|n| n.as_element_mut()) {
        el.set_attribute("name", "username");
    }

    let slot_default = doc.create_element("slot", Namespace::Html);

    doc.append_child(shadow_id, slot_named).unwrap();
    doc.append_child(shadow_id, slot_default).unwrap();

    // Verifica atribuição de nós aos slots
    let assigned_named = ace_dom::node::FlatTreeResolver::get_assigned_nodes(&doc, slot_named);
    assert_eq!(assigned_named, vec![named_child]);

    let assigned_default = ace_dom::node::FlatTreeResolver::get_assigned_nodes(&doc, slot_default);
    assert_eq!(assigned_default, vec![default_child]);

    // Flat tree do hospedeiro projeta os nós dentro dos slots da Shadow DOM
    let flat_children = doc.flat_tree_children(host_id);
    assert_eq!(flat_children.len(), 2);
}

#[test]
fn test_fast_path_simd_large_text_tokenization() {
    // Bloco contíguo de 20.000 caracteres para estressar o Fast-Path SIMD (memchr3)
    let large_paragraph = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(350);
    let html = format!("<div id=\"target\">{}</div>", large_paragraph);

    let doc = parse_html(&html);
    let target_id = doc.get_element_by_id("target").expect("target encontrado");
    let text_child = doc.children(target_id).next().unwrap().1;

    let content = text_child.text_content().unwrap();
    assert_eq!(content.len(), large_paragraph.len());
    assert_eq!(content, large_paragraph);
}
