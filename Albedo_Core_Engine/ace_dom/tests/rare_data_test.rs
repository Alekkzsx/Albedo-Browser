use ace_dom::{parse_html, ElementData, Namespace};

#[test]
fn test_element_data_memory_density_and_rare_data() {
    // 1. Verifica tamanho compacto do ElementData (sem rare data)
    let el = ElementData::new("div", Namespace::Html);
    assert!(el.rare_data().is_none());
    assert_eq!(el.shadow_root(), None);
    assert_eq!(el.template_content(), None);

    // 2. Alocação sob demanda de RareData ao definir shadow_root ou template_content
    let mut el_with_rare = ElementData::new("template", Namespace::Html);
    assert!(el_with_rare.rare_data().is_none());

    let dummy_id = ace_core::id::NodeId::new();
    el_with_rare.set_template_content(Some(dummy_id));
    assert!(el_with_rare.rare_data().is_some());
    assert_eq!(el_with_rare.template_content(), Some(dummy_id));

    // 3. Verifica mutação de campos raros customizados
    let rare = el_with_rare.ensure_rare_data();
    rare.custom_element_definition = Some(Atom::new("my-custom-element"));
    rare.inline_style = Some("color: red; display: flex;".into());

    assert_eq!(
        el_with_rare.rare_data().unwrap().custom_element_definition.as_ref().map(|a| a.as_str()),
        Some("my-custom-element")
    );
    assert_eq!(
        el_with_rare.rare_data().unwrap().inline_style.as_deref(),
        Some("color: red; display: flex;")
    );
}

#[test]
fn test_template_and_shadow_dom_rare_data_integration() {
    let html = r#"
        <div id="host">
            <template shadowrootmode="open">
                <p class="inner">Shadow Content</p>
            </template>
        </div>
    "#;

    let doc = parse_html(html);
    let host_id = doc.get_element_by_id("host").expect("host element");
    let host_node = doc.get_node(host_id).unwrap();
    let host_el = host_node.as_element().unwrap();

    assert!(host_el.shadow_root().is_some());
    let shadow_id = host_el.shadow_root().unwrap();
    let shadow_node = doc.get_node(shadow_id).unwrap();
    assert!(shadow_node.is_shadow_root());
}
