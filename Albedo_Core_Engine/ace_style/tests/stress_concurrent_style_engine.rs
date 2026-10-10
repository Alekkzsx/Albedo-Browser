//! # 🥊 Suíte 6: Concorrência Multithread e Computação Paralela via Rayon
//!
//! Exercita:
//! - Resolução paralela de estilos com Rayon (`compute_parallel`)
//! - Ausência de condições de corrida (data races) e determinismo dos resultados

use ace_dom::parse_html;
use ace_style::engine::style_engine::StyleEngine;
use ace_style::model::stylesheet::StyleSheet;
use ace_style::cascade::origin::StyleSheetOrigin;

#[test]
fn test_stress_concurrent_style_engine_rayon() {
    // Constrói árvore DOM com 100 sub-árvores sob o body
    let mut html = String::with_capacity(200_000);
    html.push_str("<html><body>");
    for i in 0..100 {
        html.push_str(&format!(
            "<div class='section-{}'><h2>Titulo {}</h2><p class='text'>Paragrafo com <span>destaque</span></p></div>",
            i % 5,
            i
        ));
    }
    html.push_str("</body></html>");

    let doc = parse_html(&html);

    let css = r#"
        div { display: block; margin: 10px; }
        .section-0 { background-color: #ff0000; }
        .section-1 { background-color: #00ff00; }
        .section-2 { background-color: #0000ff; }
        h2 { font-size: 1.5em; font-weight: bold; }
        p.text { font-size: 14px; color: #222222; }
        span { color: #555555; }
    "#;
    let sheet = StyleSheet::parse(css, StyleSheetOrigin::Author);

    let mut engine = StyleEngine::new();
    engine.add_stylesheet(sheet);

    // 1. Executa computação sequencial
    let seq_render_tree = engine.compute_styles(&doc);

    // 2. Executa computação paralela via Rayon
    let par_render_tree = engine.compute_parallel(&doc);

    assert!(seq_render_tree.root.is_some());
    assert!(par_render_tree.root.is_some());

    // 3. Valida que os resultados são determinísticos e idênticos entre sequencial e paralelo
    assert_eq!(seq_render_tree.node_styles.len(), par_render_tree.node_styles.len());

    for (node_id, seq_style) in &seq_render_tree.node_styles {
        let par_style = par_render_tree.get_style(*node_id).expect("estilo deve existir no paralelo");
        assert_eq!(seq_style.as_ref(), par_style.as_ref(), "Estilo do nó {:?} deve ser idêntico", node_id);
    }
}
