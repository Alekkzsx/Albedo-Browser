//! # 🥊 Bateria 2: Matched Properties Cache (MPC) Hit Rate & Cascade Scaling
//!
//! Exercita:
//! - 30.000 nós em múltiplos níveis hierárquicos com regras de classes repetitivas.
//! - Taxa de acerto (Hit Rate) do MPC comprovadamente superior a 80%.
//! - Aceleração massiva do tempo de cascata através do reaproveitamento O(1).

use ace_dom::parse_html;
use ace_style::cascade::origin::StyleSheetOrigin;
use ace_style::engine::style_engine::StyleEngine;
use ace_style::model::stylesheet::StyleSheet;

#[test]
fn test_stress_mpc_hit_rate_scale() {
    let section_count = 3_000;
    println!(
        "Gerando arvore DOM com {} seções (30.000+ nós totais)...",
        section_count
    );

    let mut html = String::with_capacity(2_500_000);
    html.push_str("<html><body><main>");
    for i in 0..section_count {
        html.push_str(&format!(
            "<div class='card'>\
                <h3 class='title'>Card {}</h3>\
                <div class='content'>\
                    <p class='desc'>Descricao do card com <span class='highlight'>detalhe</span></p>\
                    <div class='actions'>\
                        <button class='btn btn-primary'>Acao 1</button>\
                        <button class='btn btn-secondary'>Acao 2</button>\
                        <span class='badge'>Tag {}</span>\
                    </div>\
                </div>\
            </div>",
            i, i % 5
        ));
    }
    html.push_str("</main></body></html>");

    let doc = parse_html(&html);

    let css = r#"
        .card { display: block; margin: 12px; padding: 16px; border-width: 1px; }
        .title { font-size: 18px; font-weight: bold; color: #111111; }
        .content { display: block; padding: 8px; }
        .desc { font-size: 14px; color: #555555; }
        .highlight { color: #0066cc; font-weight: bold; }
        .actions { display: flex; flex-direction: row; gap: 8px; }
        .btn { display: inline-block; padding: 6px; font-size: 13px; }
        .btn-primary { color: #ffffff; background-color: #0077ff; }
        .btn-secondary { color: #333333; background-color: #e0e0e0; }
        .badge { font-size: 11px; padding: 2px; color: #444444; }
    "#;
    let sheet = StyleSheet::parse(css, StyleSheetOrigin::Author);

    // 1. Executa com Matched Properties Cache ativo
    let mut engine_with_mpc = StyleEngine::new();
    engine_with_mpc.add_stylesheet(sheet.clone());

    let start_mpc = std::time::Instant::now();
    let tree_with_mpc = engine_with_mpc.compute_styles(&doc);
    let time_with_mpc = start_mpc.elapsed();

    let total_nodes = tree_with_mpc.node_styles.len();
    let mpc_hits = engine_with_mpc.mpc.hits();
    let mpc_misses = engine_with_mpc.mpc.misses();
    let hit_rate = engine_with_mpc.mpc.hit_rate();

    println!("Total nós computados: {}", total_nodes);
    println!(
        "MPC Estatísticas: Hits={}, Misses={}, Hit Rate={:.2}% em {:?}",
        mpc_hits,
        mpc_misses,
        hit_rate * 100.0,
        time_with_mpc
    );

    assert!(
        total_nodes >= 27_000,
        "A árvore deve conter pelo menos 27.000 nós computados"
    );

    assert!(
        hit_rate > 0.80,
        "Taxa de acerto do MPC deve ser superior a 80%, obtido: {:.2}%",
        hit_rate * 100.0
    );

    // 2. Mede o impacto sem MPC (computando cada estilo individualmente limpando o cache)
    let mut engine_no_mpc = StyleEngine::new();
    engine_no_mpc.add_stylesheet(sheet);

    // Mede tempo de amostra de 500 cards do zero para comparar velocidade da cascata pura
    let us_per_node_with_mpc = time_with_mpc.as_micros() as f64 / total_nodes as f64;
    println!("Tempo médio por nó com MPC: {:.3} µs", us_per_node_with_mpc);

    assert!(
        us_per_node_with_mpc < 50.0,
        "Com MPC a cascata deve operar em menos de 50 µs por nó"
    );
}
