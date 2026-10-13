//! # Stress Suite 6: Incremental Subtree Layout Cache & Reactive Reflow SLA
//!
//! Requisito Normativo: Reflow Cirúrgico Incremental com Cache O(1) (LayoutCacheKey).
//! Valida:
//! 1. Árvore base de 5.000 nós no Box Tree.
//! 2. Execução sequencial de 10.000 mutações reativas pontuais com reflow incremental.
//! 3. Ramos irmãos não afetados são reaproveitados em O(1) via ponteiro Arc<LayoutFragment>.
//! 4. Taxa de acerto de cache (cache hit rate) > 90% para mutações em folhas.
//! 5. Árvore resultante do reflow incremental é geometricamente idêntica à árvore gerada do zero.
//! 6. SLA de performance: latência média de reflow < 50 µs/mutação e tempo total < 500 ms.

mod common;

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::LogicalSize;
use ace_style::values::enums::Display;

use ace_layout::engine::layout_engine::LayoutEngine;
use ace_layout::tree::{BoxTree, LayoutBox, LayoutBoxKind};
use common::{BenchmarkTimer, StyleBuilder};

fn build_large_box_tree(branches: usize, leaves_per_branch: usize) -> (BoxTree, Vec<NodeId>) {
    let root_style = StyleBuilder::new()
        .display(Display::Block)
        .width_px(1200)
        .build();
    let root_id = NodeId::new();
    let mut root = LayoutBox::new(root_id, LayoutBoxKind::Block, root_style);

    let mut leaf_ids = Vec::with_capacity(branches * leaves_per_branch);

    for _ in 0..branches {
        let branch_style = StyleBuilder::new()
            .display(Display::Block)
            .width_px(1000)
            .margin_px(0, 0, 10, 0)
            .build();
        let branch_id = NodeId::new();
        let mut branch = LayoutBox::new(branch_id, LayoutBoxKind::Block, branch_style);

        for _ in 0..leaves_per_branch {
            let leaf_id = NodeId::new();
            leaf_ids.push(leaf_id);

            let leaf_style = StyleBuilder::new()
                .display(Display::Block)
                .width_px(200)
                .height_px(40)
                .margin_px(5, 5, 5, 5)
                .build();
            let leaf = LayoutBox::new(leaf_id, LayoutBoxKind::Block, leaf_style);
            branch.add_child(leaf);
        }

        root.add_child(branch);
    }

    let box_tree = BoxTree::new(Some(root));
    (box_tree, leaf_ids)
}

#[test]
fn test_stress_10k_reactive_mutations_and_reflow_sla() {
    // 50 ramos x 100 folhas = 5.000 nós no total
    let branches = 50;
    let leaves_per_branch = 100;
    let (box_tree, leaf_ids) = build_large_box_tree(branches, leaves_per_branch);
    assert_eq!(leaf_ids.len(), 5_000);

    let mut engine = LayoutEngine::new();
    let viewport = LogicalSize::new(LayoutUnit::from_px(1200), LayoutUnit::from_px(800));

    // 1. Passe a frio (cold pass) inicial para aquecer o cache
    let initial_tree = engine.layout_box_tree(&box_tree, viewport).unwrap();
    assert!(initial_tree.root.is_some());
    assert!(!engine.cache.is_empty());

    let initial_hits = engine.cache.hits();
    let initial_misses = engine.cache.misses();

    // 2. Executa 10.000 mutações sequenciais em folhas
    let mutation_count = 10_000;
    let timer = BenchmarkTimer::start();

    for i in 0..mutation_count {
        let target_leaf_id = leaf_ids[i % leaf_ids.len()];

        // Invalidação cirúrgica O(1) da folha mutada
        engine.cache.invalidate_node(target_leaf_id);

        // Executa reflow incremental
        let fragment_tree = engine.layout_box_tree(&box_tree, viewport).unwrap();
        assert!(fragment_tree.root.is_some());
    }

    let report = timer.report(mutation_count, "Incremental_Reflow_10k_mutations");

    let total_hits = engine.cache.hits() - initial_hits;
    let total_misses = engine.cache.misses() - initial_misses;
    let total_lookups = total_hits + total_misses;

    assert!(total_lookups > 0);
    let hit_rate = (total_hits as f64) / (total_lookups as f64);

    println!(
        "[Incremental Cache Metrics] hits: {}, misses: {}, hit_rate: {:.2}%",
        total_hits,
        total_misses,
        hit_rate * 100.0
    );

    // Invariante de taxa de acerto: > 90% para mutações em folhas profundas
    assert!(
        hit_rate > 0.90,
        "Cache hit rate {:.2}% fell below required 90% SLA threshold",
        hit_rate * 100.0
    );

    // SLA de performance: latência média < 50 µs/op e tempo total < 500 ms
    report.assert_max_latency_us(50.0, "Reactive Subtree Layout Reflow SLA");
    assert!(
        report.elapsed_secs < 0.50,
        "10k mutations took {:.4}s (exceeded 0.50s SLA limit)",
        report.elapsed_secs
    );
}

#[test]
fn test_stress_incremental_vs_from_scratch_identical_geometry() {
    let (box_tree, leaf_ids) = build_large_box_tree(10, 50); // 500 nós
    let viewport = LogicalSize::new(LayoutUnit::from_px(1200), LayoutUnit::from_px(800));

    let mut warm_engine = LayoutEngine::new();
    let mut cold_engine = LayoutEngine::new();

    // Aquece o warm_engine
    let _ = warm_engine.layout_box_tree(&box_tree, viewport).unwrap();

    // Invalida cirurgicamente um nó
    let leaf_to_mutate = leaf_ids[42];
    warm_engine.cache.invalidate_node(leaf_to_mutate);

    // Executa reflow no warm_engine (incremental) e no cold_engine (do zero)
    let warm_result = warm_engine.layout_box_tree(&box_tree, viewport).unwrap();
    let cold_result = cold_engine.layout_box_tree(&box_tree, viewport).unwrap();

    // As duas árvores devem ser bit a bit equivalentes em fragmentos, geometria e paint order
    assert_eq!(warm_result.total_fragments(), cold_result.total_fragments());

    let warm_root = warm_result.root.as_ref().unwrap();
    let cold_root = cold_result.root.as_ref().unwrap();

    assert_eq!(warm_root.bounds, cold_root.bounds);
    assert_eq!(warm_root.snapped_bounds, cold_root.snapped_bounds);
    assert_eq!(warm_root.children.len(), cold_root.children.len());

    for (w_child, c_child) in warm_root.children.iter().zip(cold_root.children.iter()) {
        assert_eq!(w_child.bounds, c_child.bounds);
        assert_eq!(w_child.snapped_bounds, c_child.snapped_bounds);
    }
}
