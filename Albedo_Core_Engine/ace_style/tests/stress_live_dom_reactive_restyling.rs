use ace_core::intern::Atom;
use ace_dom::parse_html;
use ace_style::cascade::origin::StyleSheetOrigin;
use ace_style::engine::invalidation::DOMMutation;
use ace_style::engine::restyle_driver::RestyleDriver;
use ace_style::engine::style_engine::{BoxGenerationKind, RenderTreeNode, StyleEngine};
use ace_style::model::stylesheet::StyleSheet;
use std::time::Instant;

#[test]
fn test_stress_live_dom_reactive_restyling() {
    // 1. Gera documento HTML sintético denso com 2.000 elementos
    let mut html = String::from("<!DOCTYPE html><html><head></head><body><div id=\"root\">");
    for i in 0..2_000 {
        html.push_str(&format!(
            "<div id=\"item-{}\" class=\"card item\"><span>Title {}</span><p class=\"desc\">Content</p></div>",
            i, i
        ));
    }
    html.push_str("</div></body></html>");

    let doc = parse_html(&html);
    assert!(doc.body.is_some());

    // 2. Configura motor de estilo com regras de autor
    let mut engine = StyleEngine::new();
    let css = r#"
        .card { display: block; margin: 10px; color: #333; }
        .theme-dark { background-color: #000; color: #fff; }
        .highlight { border: 2px solid #ff0000; }
        [data-status="active"] { display: flex; }
    "#;
    let sheet = StyleSheet::parse(css, StyleSheetOrigin::Author);
    engine.add_stylesheet(sheet);

    // 3. Inicializa RestyleDriver
    let start_init = Instant::now();
    let mut driver = RestyleDriver::new(engine, &doc);
    let init_elapsed = start_init.elapsed();
    println!(
        "[STRESS RESTYLE] RenderTree inicial para 4.000+ nós compilada em {:.2}ms",
        init_elapsed.as_secs_f64() * 1000.0
    );

    let initial_styles_count = driver.render_tree().node_styles.len();
    assert!(initial_styles_count > 2000);

    // 4. Executa 100 mutações dinâmicas reativas simuladas
    let root_id = doc.document_element.unwrap();
    let start_mutations = Instant::now();

    for i in 0..100 {
        let mutation = if i % 2 == 0 {
            DOMMutation::ClassChanged {
                old_class: Some(Atom::new("item")),
                new_class: Some(Atom::new("theme-dark")),
            }
        } else {
            DOMMutation::AttributeChanged {
                name: Atom::new("data-status"),
            }
        };

        let invalidated = driver.on_mutation(&doc, root_id, &mutation);
        assert!(invalidated > 0);
    }

    let mutations_elapsed = start_mutations.elapsed();
    let avg_per_mutation = (mutations_elapsed.as_secs_f64() * 1000.0) / 100.0;
    println!(
        "[STRESS RESTYLE] 100 mutações processadas em {:.2}ms (média: {:.3}ms por mutação)",
        mutations_elapsed.as_secs_f64() * 1000.0,
        avg_per_mutation
    );

    // Validação de BoxGenerationKind no elemento body
    if let Some(body_id) = doc.body {
        let body_style = driver
            .render_tree()
            .get_style(body_id)
            .expect("Estilo do body deve estar presente na RenderTree");
        assert_eq!(
            body_style.display(),
            ace_style::values::enums::Display::Block
        );

        let render_node = RenderTreeNode::new_element(body_id, body_style.clone(), Vec::new());
        assert_eq!(render_node.box_generation_kind(), BoxGenerationKind::Block);
        assert!(!render_node.is_out_of_flow());
    }

    assert!(
        avg_per_mutation < 150.0,
        "Latência média por mutação deve ser controlada mesmo em modo debug"
    );
}
