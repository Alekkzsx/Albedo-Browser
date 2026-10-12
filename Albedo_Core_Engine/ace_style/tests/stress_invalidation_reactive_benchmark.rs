//! # 🥊 Bateria 3: Invalidação Reativa Cirúrgica em Escala Industrial
//!
//! Exercita:
//! - 100.000 mutações reativas pontuais de classe em árvore densa de 50.000 nós.
//! - Garantia estrita de que menos de 1% da árvore é recalculada por ciclo de invalidação.
//! - Latência média de invalidação cirúrgica inferior a 0.1ms (100 microssegundos).

use ace_core::intern::Atom;
use ace_dom::parse_html;
use ace_style::cascade::origin::StyleSheetOrigin;
use ace_style::engine::invalidation::DOMMutation;
use ace_style::engine::style_engine::StyleEngine;
use ace_style::model::stylesheet::StyleSheet;

#[test]
fn test_stress_invalidation_reactive_benchmark() {
    let container_count = 5_000;
    println!(
        "Gerando arvore DOM com {} containers (50.000 nós)...",
        container_count
    );

    let mut html = String::with_capacity(3_000_000);
    html.push_str("<html><body><main id='app'>");
    for i in 0..container_count {
        html.push_str(&format!(
            "<div class='box' id='box-{}'>\
                <h4 class='title'>Titulo {}</h4>\
                <p class='desc'>Descricao {}</p>\
                <span class='badge'>Badge {}</span>\
                <div class='actions'>\
                    <button class='btn'>Botao 1</button>\
                    <button class='btn'>Botao 2</button>\
                    <span class='label'>Rotulo</span>\
                </div>\
                <span class='footer'>Rodape</span>\
                <span class='meta'>Meta</span>\
            </div>",
            i, i, i, i
        ));
    }
    html.push_str("</main></body></html>");

    let doc = parse_html(&html);

    // Regras com dependências hierárquicas e entre irmãos
    let css = r#"
        .box { display: block; padding: 10px; }
        .box.active .title { color: #ff0000; font-weight: bold; }
        .box.selected .badge { background-color: #00ff00; }
        .btn:disabled + .label { opacity: 0.5; }
    "#;
    let sheet = StyleSheet::parse(css, StyleSheetOrigin::Author);

    let mut engine = StyleEngine::new();
    engine.add_stylesheet(sheet);

    // Primeiro computa estilos da árvore inicial
    let initial_tree = engine.compute_styles(&doc);
    let total_nodes = initial_tree.node_styles.len();
    println!("Total nós computados: {}", total_nodes);
    assert!(
        total_nodes >= 50_000,
        "A árvore deve conter 50.000 nós para teste de carga"
    );

    // Coleta IDs dos containers .box
    let mut box_ids = Vec::with_capacity(container_count);
    for (id, node) in doc.descendants(doc.root()) {
        if let Some(el) = node.as_element() {
            if el.has_class("box") {
                box_ids.push(id);
            }
        }
    }
    assert_eq!(box_ids.len(), container_count);

    // Executa 100.000 mutações pontuais de classe
    let mutation_iterations = 100_000;
    println!(
        "Disparando {} mutações pontuais de classe na árvore...",
        mutation_iterations
    );

    let atom_active = Atom::new("active");
    let mutation_add = DOMMutation::ClassChanged {
        old_class: None,
        new_class: Some(atom_active.clone()),
    };
    let mutation_remove = DOMMutation::ClassChanged {
        old_class: Some(atom_active),
        new_class: None,
    };

    let start = std::time::Instant::now();
    let mut total_invalidated_nodes = 0;
    let mut max_invalidated_in_cycle = 0;

    for i in 0..mutation_iterations {
        let target_node_id = box_ids[i % box_ids.len()];
        let mutation = if i % 2 == 0 {
            &mutation_add
        } else {
            &mutation_remove
        };

        let result = engine.invalidate_mutation(&doc, target_node_id, mutation);
        total_invalidated_nodes += result.invalidated_count;
        if result.invalidated_count > max_invalidated_in_cycle {
            max_invalidated_in_cycle = result.invalidated_count;
        }

        // Critério Inegociável: Menos de 1% da árvore recalculada por ciclo
        let cycle_fraction = result.invalidated_count as f64 / total_nodes as f64;
        assert!(
            cycle_fraction < 0.01,
            "Ciclo {} invalidou {} nós ({:.2}%), que excede o limite de 1%",
            i,
            result.invalidated_count,
            cycle_fraction * 100.0
        );
    }

    let elapsed = start.elapsed();
    let avg_latency_us = (elapsed.as_micros() as f64) / (mutation_iterations as f64);
    let avg_latency_ms = avg_latency_us / 1000.0;

    println!(
        "{} mutações concluídas em {:?}",
        mutation_iterations, elapsed
    );
    println!(
        "Latência média por invalidação: {:.3} µs ({:.5} ms)",
        avg_latency_us, avg_latency_ms
    );
    println!(
        "Média de nós invalidados por ciclo: {:.2} nós (Máximo no ciclo: {} nós)",
        total_invalidated_nodes as f64 / mutation_iterations as f64,
        max_invalidated_in_cycle
    );

    // Critério Inegociável: Latência média < 0.1ms (100 µs)
    assert!(
        avg_latency_ms < 0.1,
        "Latência média de invalidação cirúrgica deve ser < 0.1ms, obtido: {:.5}ms",
        avg_latency_ms
    );
}
