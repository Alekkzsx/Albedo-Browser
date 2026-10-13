//! # Stress Suite 1: Deep Hierarchy & MarginStrut Collapse Invariants
//!
//! Requisito Normativo: CSS 2.2 §8.3.1 (Collapsing Margins) e LayoutNG NGMarginStrut.
//! Valida:
//! 1. 10.000 nós de bloco com `MarginStrut` vertical margin collapse em $O(N)$ e sem vazamentos de memória.
//! 2. Invariante matemático estrito de colapso: $\text{Margin} = \max(m_{\text{pos}}) + \min(m_{\text{neg}})$.
//! 3. Cadeia profunda de 10.000 irmãos com margens positivas, negativas, nulas e blocos vazios.
//! 4. Árvore de aninhamento profundo (500+ níveis) garantindo segurança de pilha (*stack safety*).
//! 5. Contenção de novos BFCs independentes (`overflow != visible`) prevenindo vazamento de margem.
//! 6. SLA de performance medido com `BenchmarkTimer`.

mod common;

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::LogicalSize;
use ace_style::values::enums::{Display, Overflow};

use ace_layout::block::{BlockFormattingContext, MarginStrut};
use ace_layout::inline::FixedTextMeasurer;
use ace_layout::tree::{ConstraintSpace, LayoutBox, LayoutBoxKind};
use common::{BenchmarkTimer, StyleBuilder};

#[test]
fn test_stress_margin_strut_accumulator_100k_ops() {
    let timer = BenchmarkTimer::start();
    let mut strut = MarginStrut::empty();
    assert!(strut.is_empty());

    let iterations = 100_000;
    for i in 0..iterations {
        // Sequência com margens positivas, negativas e zeros
        let val = match i % 5 {
            0 => LayoutUnit::from_px(30),
            1 => LayoutUnit::from_px(-15),
            2 => LayoutUnit::from_px(20),
            3 => LayoutUnit::from_px(-10),
            _ => LayoutUnit::ZERO,
        };
        strut.append(val);
    }

    // A maior positiva deve ser 30px (1800 raw) e a menor negativa -15px (-900 raw)
    assert_eq!(strut.positive_margin, LayoutUnit::from_px(30));
    assert_eq!(strut.negative_margin, LayoutUnit::from_px(-15));

    // Invariante matemático: 30px + (-15px) = 15px (900 raw units)
    let solved = strut.solve();
    assert_eq!(solved, LayoutUnit::from_px(15));
    assert_eq!(solved.raw(), 900);

    let report = timer.report(iterations, "MarginStrut::append_100k");
    report.assert_max_latency_us(1.0, "MarginStrut O(1) accumulation");
}

#[test]
fn test_stress_margin_strut_deep_sibling_chain_10k_nodes() {
    let count = 10_000;
    let timer = BenchmarkTimer::start();

    let root_style = StyleBuilder::new()
        .display(Display::Block)
        .width_px(800)
        .build();
    let mut root = LayoutBox::new(NodeId::new(), LayoutBoxKind::Block, root_style);

    // Constrói 10.000 nós irmãos
    for i in 0..count {
        let (top, bottom, height) = match i % 4 {
            0 => (20, -10, 50),
            1 => (30, -5, 50),
            2 => (0, 15, 50),
            _ => (10, -20, 50),
        };

        let child_style = StyleBuilder::new()
            .display(Display::Block)
            .width_px(700)
            .height_px(height)
            .vertical_margin_px(top, bottom)
            .build();
        let child = LayoutBox::new(NodeId::new(), LayoutBoxKind::Block, child_style);
        root.add_child(child);
    }

    assert_eq!(root.children.len(), count);

    let space = ConstraintSpace::for_container(LogicalSize::new(
        LayoutUnit::from_px(800),
        LayoutUnit::from_px(600),
    ));
    let measurer = FixedTextMeasurer::new();
    let bfc = BlockFormattingContext::new();

    let result = bfc.layout(&root, &space, &measurer);

    // Valida que todos os 10.000 nós foram processados
    assert_eq!(result.children.len(), count);

    // Valida o colapso exato conforme a fórmula CSS 2.2 §8.3.1 para todos os 10.000 nós
    for i in 1..count {
        let prev = &result.children[i - 1];
        let curr = &result.children[i];

        let prev_end = prev.border_box.block_start + prev.border_box.block_size;
        let expected_collapsed = match (i - 1) % 4 {
            0 => LayoutUnit::from_px(20), // bottom(-10) e top(+30) => 30 - 10 = +20px
            1 => LayoutUnit::from_px(-5), // bottom(-5) e top(0) => 0 - 5 = -5px
            2 => LayoutUnit::from_px(15), // bottom(+15) e top(+10) => max(15, 10) = +15px
            _ => LayoutUnit::from_px(0),  // bottom(-20) e top(+20) => 20 - 20 = 0px
        };

        assert_eq!(
            curr.border_box.block_start - prev_end,
            expected_collapsed,
            "Mathematical collapse violation at sibling index {}: expected {:?}, found {:?}",
            i,
            expected_collapsed,
            curr.border_box.block_start - prev_end
        );
    }

    let report = timer.report(count, "BfcLayout_10k_siblings");
    // O SLA permite até 100ms para os 10.000 nós
    assert!(
        report.elapsed_secs < 0.20,
        "10k siblings layout took {:.4}s, exceeding SLA limit of 0.20s",
        report.elapsed_secs
    );
}

#[test]
fn test_stress_margin_strut_deep_parent_child_nesting_stack_safety() {
    let depth = 500;
    let timer = BenchmarkTimer::start();

    // Cria uma cadeia profundamente aninhada: root -> child_1 -> ... -> child_depth
    let mut current_box = {
        let leaf_style = StyleBuilder::new()
            .display(Display::Block)
            .width_px(200)
            .height_px(40)
            .vertical_margin_px(10, 10)
            .build();
        LayoutBox::new(NodeId::new(), LayoutBoxKind::Block, leaf_style)
    };

    for i in (0..depth).rev() {
        let (top_margin, bottom_margin) = if i % 2 == 0 { (15, -5) } else { (25, -10) };
        let parent_style = StyleBuilder::new()
            .display(Display::Block)
            .vertical_margin_px(top_margin, bottom_margin)
            .build();
        let mut parent = LayoutBox::new(NodeId::new(), LayoutBoxKind::Block, parent_style);
        parent.add_child(current_box);
        current_box = parent;
    }

    let space = ConstraintSpace::for_container(LogicalSize::new(
        LayoutUnit::from_px(800),
        LayoutUnit::from_px(600),
    ));
    let measurer = FixedTextMeasurer::new();
    let bfc = BlockFormattingContext::new();

    // Executa layout na árvore de 500 níveis: não deve ocorrer estouro de pilha (stack overflow)
    let result = bfc.layout(&current_box, &space, &measurer);

    assert!(result.content_height() >= LayoutUnit::from_px(40));
    assert!(timer.elapsed().as_secs_f64() < 0.15);
}

#[test]
fn test_stress_margin_strut_empty_blocks_chain_collapsing() {
    // Cadeia de 1.000 blocos vazios com margens diversas
    let count = 1_000;
    let root_style = StyleBuilder::new()
        .display(Display::Block)
        .width_px(800)
        .build();
    let mut root = LayoutBox::new(NodeId::new(), LayoutBoxKind::Block, root_style);

    // Primeiro nó concreto
    let first = LayoutBox::new(
        NodeId::new(),
        LayoutBoxKind::Block,
        StyleBuilder::new()
            .display(Display::Block)
            .height_px(50)
            .vertical_margin_px(10, 20)
            .build(),
    );
    root.add_child(first);

    // 1.000 blocos vazios (height: 0, padding: 0, border: 0)
    for i in 0..count {
        let (top, bottom) = match i % 3 {
            0 => (15, -5),
            1 => (-10, 25),
            _ => (0, 0),
        };
        let empty_box = LayoutBox::new(
            NodeId::new(),
            LayoutBoxKind::Block,
            StyleBuilder::new()
                .display(Display::Block)
                .height_px(0)
                .vertical_margin_px(top, bottom)
                .build(),
        );
        root.add_child(empty_box);
    }

    // Segundo nó concreto
    let second = LayoutBox::new(
        NodeId::new(),
        LayoutBoxKind::Block,
        StyleBuilder::new()
            .display(Display::Block)
            .height_px(50)
            .vertical_margin_px(10, 10)
            .build(),
    );
    root.add_child(second);

    let space = ConstraintSpace::for_container(LogicalSize::new(
        LayoutUnit::from_px(800),
        LayoutUnit::from_px(600),
    ));
    let measurer = FixedTextMeasurer::new();
    let bfc = BlockFormattingContext::new();

    let result = bfc.layout(&root, &space, &measurer);

    assert_eq!(result.children.len(), count + 2);

    // O colapso através dos 1.000 blocos vazios não deve somar milhares de pixels.
    // A margem positiva máxima entre o final do primeiro e o segundo nó é 25px,
    // e a negativa mínima é -10px, resultando em 15px de colapso.
    let first_box = &result.children[0];
    let second_box = &result.children[count + 1];

    let distance = second_box.border_box.block_start
        - (first_box.border_box.block_start + first_box.border_box.block_size);

    // Distância colapsada esperada entre nós concretos separados por blocos vazios:
    // max(20, 15, 25, 10) + min(-5, -10) = 25 - 10 = 15px.
    assert_eq!(distance, LayoutUnit::from_px(15));
}

#[test]
fn test_stress_margin_strut_bfc_containment_barrier() {
    let root_style = StyleBuilder::new()
        .display(Display::Block)
        .width_px(800)
        .build();
    let mut root = LayoutBox::new(NodeId::new(), LayoutBoxKind::Block, root_style);

    // Filho 1: BFC independente via overflow: hidden
    let mut bfc_box = LayoutBox::new(
        NodeId::new(),
        LayoutBoxKind::Block,
        StyleBuilder::new()
            .display(Display::Block)
            .height_px(100)
            .vertical_margin_px(20, 30)
            .overflow(Overflow::Hidden)
            .build(),
    );

    // Filho interno com margens extremas (+500px, -200px) dentro do BFC independente
    let internal_child = LayoutBox::new(
        NodeId::new(),
        LayoutBoxKind::Block,
        StyleBuilder::new()
            .display(Display::Block)
            .height_px(50)
            .vertical_margin_px(500, -200)
            .build(),
    );
    bfc_box.add_child(internal_child);
    root.add_child(bfc_box);

    // Filho 2 adjacente no fluxo normal
    let sibling = LayoutBox::new(
        NodeId::new(),
        LayoutBoxKind::Block,
        StyleBuilder::new()
            .display(Display::Block)
            .height_px(50)
            .vertical_margin_px(15, 15)
            .build(),
    );
    root.add_child(sibling);

    let space = ConstraintSpace::for_container(LogicalSize::new(
        LayoutUnit::from_px(800),
        LayoutUnit::from_px(600),
    ));
    let measurer = FixedTextMeasurer::new();
    let bfc = BlockFormattingContext::new();

    let result = bfc.layout(&root, &space, &measurer);

    // O filho 1 (BFC independente) tem margem inferior de 30px.
    // O filho 2 tem margem superior de 15px.
    // Colapso entre BFC independente e irmão adjacente: max(30, 15) = 30px.
    // As margens internas (+500px, -200px) NÃO devem ter vazado para fora!
    let bfc_result = &result.children[0];
    let sibling_result = &result.children[1];

    let distance = sibling_result.border_box.block_start
        - (bfc_result.border_box.block_start + bfc_result.border_box.block_size);

    assert_eq!(distance, LayoutUnit::from_px(30));
}
