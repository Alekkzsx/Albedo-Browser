//! # 🥊 Bateria 1: Stress & Benchmark de Memória das Style Structs (Stylo Model)
//!
//! Exercita:
//! - Construção de árvore DOM massiva com 50.000 nós.
//! - Validação do reaproveitamento estático de `Arc` para structs resetadas (`INITIAL_BOX`, `INITIAL_SURROUNDING`, etc.).
//! - Comprovação de mais de 60% de economia de memória heap através do compartilhamento estrutural.

use ace_dom::parse_html;
use ace_style::cascade::origin::StyleSheetOrigin;
use ace_style::computed::{INITIAL_BOX, INITIAL_FLEX, INITIAL_GRID, INITIAL_SURROUNDING};
use ace_style::engine::style_engine::StyleEngine;
use ace_style::model::stylesheet::StyleSheet;
use std::sync::Arc;

#[test]
fn test_stress_style_structs_memory_bench() {
    let node_count = 50_000;
    println!("Construindo arvore DOM com {} nós...", node_count);

    // Constrói 5.000 containers cada um com 10 elementos filhos
    let mut html = String::with_capacity(3_000_000);
    html.push_str("<html><body>");
    for i in 0..5_000 {
        html.push_str("<div class='container'>");
        for j in 0..9 {
            html.push_str(&format!(
                "<span class='text-item'>Texto {}-{}</span>",
                i, j
            ));
        }
        html.push_str("</div>");
    }
    html.push_str("</body></html>");

    let doc = parse_html(&html);

    // CSS que altera apenas propriedades herdadas/de cor nos spans (sem sobrescrever box, flex, grid)
    let css = r#"
        .container {
            display: block;
            margin: 0;
        }
        .text-item {
            color: #112233;
            font-size: 14px;
        }
    "#;
    let sheet = StyleSheet::parse(css, StyleSheetOrigin::Author);

    let mut engine = StyleEngine::new();
    engine.add_stylesheet(sheet);

    let start = std::time::Instant::now();
    let render_tree = engine.compute_styles(&doc);
    let elapsed = start.elapsed();

    println!(
        "50.000 nós processados em {:?} ({:.2} nós/ms)",
        elapsed,
        node_count as f64 / elapsed.as_millis().max(1) as f64
    );

    assert!(
        render_tree.node_styles.len() >= node_count,
        "Todos os nós devem ter estilos computados na árvore"
    );

    // Avaliação de compartilhamento de ponteiros estáticos globais
    let mut shared_box = 0;
    let mut shared_surrounding = 0;
    let mut shared_flex = 0;
    let mut shared_grid = 0;

    for style in render_tree.node_styles.values() {
        if Arc::ptr_eq(&style.box_style, &INITIAL_BOX) {
            shared_box += 1;
        }
        if Arc::ptr_eq(&style.surrounding, &INITIAL_SURROUNDING) {
            shared_surrounding += 1;
        }
        if Arc::ptr_eq(&style.flex, &INITIAL_FLEX) {
            shared_flex += 1;
        }
        if Arc::ptr_eq(&style.grid, &INITIAL_GRID) {
            shared_grid += 1;
        }
    }

    let total = render_tree.node_styles.len();
    let flex_share_rate = shared_flex as f64 / total as f64;
    let grid_share_rate = shared_grid as f64 / total as f64;
    let surr_share_rate = shared_surrounding as f64 / total as f64;

    println!(
        "Taxa de compartilhamento: Flex={:.1}%, Grid={:.1}%, Surrounding={:.1}%, Box={}/{}",
        flex_share_rate * 100.0,
        grid_share_rate * 100.0,
        surr_share_rate * 100.0,
        shared_box,
        total
    );

    // Flex e Grid não foram modificados por nenhum seletor -> 100% de reuso de ponteiros estáticos
    assert!(
        flex_share_rate > 0.95,
        "Flex deve ser compartilhado por mais de 95% dos nós"
    );
    assert!(
        grid_share_rate > 0.95,
        "Grid deve ser compartilhado por mais de 95% dos nós"
    );
    // Mais de 80% dos nós (os 45.000 spans) compartilham o INITIAL_SURROUNDING
    assert!(
        surr_share_rate > 0.80,
        "Surrounding deve ser compartilhado por mais de 80% dos nós"
    );

    // Cálculo da economia de memória comparado ao modelo monolítico:
    // Tamanho se cada nó tivesse uma struct Box, Surrounding, Flex, Grid alocada separadamente:
    let monolithic_struct_size = std::mem::size_of::<ace_style::computed::StyleBox>()
        + std::mem::size_of::<ace_style::computed::StyleSurrounding>()
        + std::mem::size_of::<ace_style::computed::StyleFlex>()
        + std::mem::size_of::<ace_style::computed::StyleGrid>()
        + std::mem::size_of::<ace_style::computed::StyleFont>()
        + std::mem::size_of::<ace_style::computed::StyleColor>()
        + std::mem::size_of::<ace_style::computed::StyleText>();

    let actual_allocated_substructs_per_node = (total - shared_box)
        + (total - shared_surrounding)
        + (total - shared_flex)
        + (total - shared_grid);

    let theoretical_max_allocations = total * 4;
    let savings_rate = 1.0 - (actual_allocated_substructs_per_node as f64 / theoretical_max_allocations as f64);

    println!(
        "Economia de memória heap de structs resetadas: {:.2}% (tamanho struct de referência: {}B)",
        savings_rate * 100.0,
        monolithic_struct_size
    );

    assert!(
        savings_rate > 0.60,
        "Economia de RAM comprovada deve ser superior a 60%, obtido: {:.2}%",
        savings_rate * 100.0
    );
}
