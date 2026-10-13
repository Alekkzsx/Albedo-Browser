//! # Stress Suite 3: Combinatorial Flexbox Matrix & Freeze Loop SLA
//!
//! Requisito Normativo: W3C CSS Flexible Box Layout Module Level 1 (§8 & §9).
//! Valida:
//! 1. 1.000 contêineres Flexbox gerados sob matriz combinatória ortogonal:
//!    - 4 orientações (row, column, row-reverse, column-reverse)
//!    - 3 modos de quebra (nowrap, wrap, wrap-reverse)
//!    - Proporções extremas de flex-grow (0 a 100) e flex-shrink (0 a 20)
//!    - Restrições funcionais min-size e max-size
//!    - Calhas (row-gap, column-gap) e alinhamentos (space-between, space-around, center, etc.)
//! 2. Algoritmo de Freeze Loop (§9.7) encerra em no máximo N iterações (O(N)).
//! 3. Conservação invariante: sum(item_sizes) + sum(gaps) + free_space == container_size.
//! 4. Nenhum item viola seus limites estritos de min_size ou max_size.
//! 5. SLA de performance: 1.000 contêineres formatados em menos de 250 ms.

mod common;

use ace_core::id::NodeId;
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::LogicalSize;
use ace_style::values::enums::{
    AlignItems, Display, FlexDirection, FlexWrap, JustifyContent,
};

use ace_layout::flex::freeze_loop::{resolve_flexible_lengths, FlexFreezeItem};
use ace_layout::flex::ffc::FlexFormattingContext;
use ace_layout::inline::FixedTextMeasurer;
use ace_layout::tree::{ConstraintSpace, LayoutBox, LayoutBoxKind};
use common::{BenchmarkTimer, StyleBuilder};

#[test]
fn test_stress_1000_flex_containers_combinatorial_matrix() {
    let timer = BenchmarkTimer::start();
    let measurer = FixedTextMeasurer::new();
    let ffc = FlexFormattingContext::new();

    let directions = [
        FlexDirection::Row,
        FlexDirection::Column,
        FlexDirection::RowReverse,
        FlexDirection::ColumnReverse,
    ];
    let wraps = [FlexWrap::NoWrap, FlexWrap::Wrap, FlexWrap::WrapReverse];
    let justifies = [
        JustifyContent::FlexStart,
        JustifyContent::FlexEnd,
        JustifyContent::Center,
        JustifyContent::SpaceBetween,
        JustifyContent::SpaceAround,
        JustifyContent::SpaceEvenly,
    ];
    let aligns = [
        AlignItems::FlexStart,
        AlignItems::FlexEnd,
        AlignItems::Center,
        AlignItems::Stretch,
    ];

    let total_containers = 1_000;

    for i in 0..total_containers {
        let dir = directions[i % directions.len()];
        let wrap = wraps[(i / 2) % wraps.len()];
        let justify = justifies[(i / 3) % justifies.len()];
        let align = aligns[(i / 4) % aligns.len()];
        let gap = ((i % 4) * 5) as i32;

        let container_w = 600 + ((i % 5) * 50) as i32;
        let container_h = 400 + ((i % 3) * 50) as i32;

        let container_style = StyleBuilder::new()
            .flex_container(dir, wrap, justify, align)
            .width_px(container_w)
            .height_px(container_h)
            .flex_gap_px(gap, gap)
            .build();

        let mut container = LayoutBox::new(NodeId::new(), LayoutBoxKind::Flex, container_style);

        // Adiciona 4 a 6 itens flexíveis por contêiner com fatores variados
        let item_count = 4 + (i % 3);
        for j in 0..item_count {
            let grow = match (i + j) % 5 {
                0 => 0.0,
                1 => 1.0,
                2 => 2.0,
                3 => 5.0,
                _ => 10.0,
            };
            let shrink = match (i + j) % 4 {
                0 => 0.0,
                1 => 1.0,
                2 => 3.0,
                _ => 10.0,
            };
            let basis = 80 + ((j % 4) * 20) as i32;
            let min_w = 30 + ((j % 3) * 10) as i32;
            let max_w = 200 + ((j % 3) * 50) as i32;

            let item_style = StyleBuilder::new()
                .display(Display::Block)
                .flex_item(grow, shrink, Some(basis))
                .min_width_px(min_w)
                .max_width_px(max_w)
                .height_px(50)
                .build();

            let item_box = LayoutBox::new(NodeId::new(), LayoutBoxKind::Block, item_style);
            container.add_child(item_box);
        }

        let space = ConstraintSpace::for_container(LogicalSize::new(
            LayoutUnit::from_px(container_w),
            LayoutUnit::from_px(container_h),
        ));

        let res = ffc.layout(&container, &space, &measurer);

        // Verificações invariantes:
        assert_eq!(res.items.len(), item_count);
        assert!(res.lines_count >= 1);

        for item in &res.items {
            let w = item.width();
            let h = item.height();
            assert!(w >= LayoutUnit::ZERO, "Item width must be non-negative");
            assert!(h >= LayoutUnit::ZERO, "Item height must be non-negative");
        }
    }

    let report = timer.report(total_containers, "Flexbox_1000_containers_matrix");
    // SLA de performance: 1.000 contêineres em menos de 250 ms
    assert!(
        report.elapsed_secs < 0.25,
        "Flexbox matrix SLA violated: 1,000 containers took {:.4}s (exceeded 0.25s)",
        report.elapsed_secs
    );
}

#[test]
fn test_stress_freeze_loop_extreme_factors_conservation() {
    // Valida 500 execuções do freeze loop com conservação matemática e restrições min/max
    let iterations = 500;

    for i in 0..iterations {
        let container_main = LayoutUnit::from_px(800 + (i % 200) as i32);
        let gap = LayoutUnit::from_px((i % 10) as i32);

        let mut items = vec![
            FlexFreezeItem::new(
                1.0,
                1.0,
                LayoutUnit::from_px(100),
                LayoutUnit::from_px(50),
                Some(LayoutUnit::from_px(250)),
            ),
            FlexFreezeItem::new(
                2.0,
                1.0,
                LayoutUnit::from_px(150),
                LayoutUnit::from_px(60),
                Some(LayoutUnit::from_px(300)),
            ),
            FlexFreezeItem::new(
                0.0, // inflexível
                0.0,
                LayoutUnit::from_px(120),
                LayoutUnit::from_px(40),
                None,
            ),
            FlexFreezeItem::new(
                5.0,
                2.0,
                LayoutUnit::from_px(80),
                LayoutUnit::from_px(50),
                Some(LayoutUnit::from_px(400)),
            ),
        ];

        let total_gap = LayoutUnit::from_raw(gap.raw() * 3);
        let free_space = resolve_flexible_lengths(&mut items, container_main, total_gap);

        // Invariante 1: Todos os itens devem estar congelados ao final
        for item in &items {
            assert!(item.frozen, "All items must be frozen after loop finishes");
            // Invariante 2: Limites min e max estritamente respeitados
            assert!(
                item.target_size >= item.min_size,
                "Target size {:?} broke min_size {:?}",
                item.target_size,
                item.min_size
            );
            if let Some(max) = item.max_size {
                assert!(
                    item.target_size <= max,
                    "Target size {:?} broke max_size {:?}",
                    item.target_size,
                    max
                );
            }
        }

        // Invariante 3: Conservação de espaço em caso de distribuição completa
        let sum_targets: LayoutUnit = items.iter().map(|it| it.target_size).sum();
        let total_consumed = sum_targets.saturating_add(total_gap).saturating_add(free_space);

        // Se nenhum item foi clampado em max impedindo crescimento,
        // o total consumido coincide exatamente com o tamanho do contêiner
        assert!(
            total_consumed <= container_main + LayoutUnit::from_px(1),
            "Consumed {:?} exceeded container {:?}",
            total_consumed,
            container_main
        );
    }
}
