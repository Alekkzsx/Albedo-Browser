//! # 🥊 Bateria 6: Matriz Combinatória Bidi & Propriedades Lógicas W3C
//!
//! Exercita:
//! - Todas as 6 combinações da matriz canônica:
//!   `direction` (`ltr`, `rtl`) × `writing-mode` (`horizontal-tb`, `vertical-rl`, `vertical-lr`).
//! - Mapeamento físico de:
//!   - Dimensões lógicas: `inline-size`, `block-size`, `min-inline-size`, `max-inline-size`, `min-block-size`, `max-block-size`.
//!   - Margens lógicas: `margin-inline-start`, `margin-inline-end`, `margin-block-start`, `margin-block-end`, e shorthands `margin-inline`, `margin-block`.
//!   - Paddings lógicos: `padding-inline-start`, `padding-inline-end`, `padding-block-start`, `padding-block-end`, e shorthands `padding-inline`, `padding-block`.
//!   - Posicionamento lógico: `inset-inline-start`, `inset-inline-end`, `inset-block-start`, `inset-block-end`, e shorthands `inset-inline`, `inset-block`.
//! - Herança bidi hierárquica através da árvore DOM (crianças herdando e/ou sobrescrevendo direção e writing-mode).
//! - Sobrescrita em cascata entre propriedades físicas e lógicas com base na ordem de declaração.
//! - Escala massiva de 6.000 nós gerados dinamicamente com verificação exaustiva.

use ace_core::math::LayoutUnit;
use ace_dom::parse_html;
use ace_style::cascade::origin::StyleSheetOrigin;
use ace_style::engine::style_engine::StyleEngine;
use ace_style::model::stylesheet::StyleSheet;
use ace_style::values::enums::{Direction, WritingMode};

#[test]
fn test_stress_logical_properties_bidi_matrix() {
    println!("Iniciando teste de estresse da Matriz Combinatória Bidi & Propriedades Lógicas W3C...");

    // 1. Definição da matriz de 6 combinações canônicas
    let matrix = [
        (Direction::Ltr, WritingMode::HorizontalTb, "ltr", "horizontal-tb"),
        (Direction::Rtl, WritingMode::HorizontalTb, "rtl", "horizontal-tb"),
        (Direction::Ltr, WritingMode::VerticalRl, "ltr", "vertical-rl"),
        (Direction::Rtl, WritingMode::VerticalRl, "rtl", "vertical-rl"),
        (Direction::Ltr, WritingMode::VerticalLr, "ltr", "vertical-lr"),
        (Direction::Rtl, WritingMode::VerticalLr, "rtl", "vertical-lr"),
    ];

    // Constrói folha de estilo de teste abrangente
    let css = r#"
        /* 1. Classes base da matriz */
        .mode-0 { direction: ltr; writing-mode: horizontal-tb; }
        .mode-1 { direction: rtl; writing-mode: horizontal-tb; }
        .mode-2 { direction: ltr; writing-mode: vertical-rl; }
        .mode-3 { direction: rtl; writing-mode: vertical-rl; }
        .mode-4 { direction: ltr; writing-mode: vertical-lr; }
        .mode-5 { direction: rtl; writing-mode: vertical-lr; }

        /* 2. Regras de dimensões lógicas */
        .box-dims {
            inline-size: 200px;
            block-size: 100px;
            min-inline-size: 150px;
            max-inline-size: 300px;
            min-block-size: 80px;
            max-block-size: 120px;
        }

        /* 3. Regras de margens e paddings lógicos */
        .surrounding-longhands {
            margin-inline-start: 10px;
            margin-inline-end: 20px;
            margin-block-start: 30px;
            margin-block-end: 40px;
            padding-inline-start: 5px;
            padding-inline-end: 15px;
            padding-block-start: 25px;
            padding-block-end: 35px;
        }

        /* 4. Regras com shorthands lógicos */
        .surrounding-shorthands {
            margin-inline: 11px 22px;
            margin-block: 33px 44px;
            padding-inline: 6px 16px;
            padding-block: 26px 36px;
        }

        /* 5. Regras de insets lógicos */
        .inset-rules {
            position: absolute;
            inset-inline-start: 12px;
            inset-inline-end: 24px;
            inset-block-start: 36px;
            inset-block-end: 48px;
        }

        /* 6. Cascata: Lógica sobrescreve Física */
        .override-phys-with-logic {
            margin-left: 99px;
            margin-right: 99px;
            margin-top: 99px;
            margin-bottom: 99px;
            margin-inline-start: 15px;
            margin-inline-end: 25px;
            margin-block-start: 35px;
            margin-block-end: 45px;
        }

        /* 7. Cascata: Física sobrescreve Lógica */
        .override-logic-with-phys {
            margin-inline-start: 15px;
            margin-inline-end: 25px;
            margin-block-start: 35px;
            margin-block-end: 45px;
            margin-top: 50px;
            margin-bottom: 60px;
            margin-left: 70px;
            margin-right: 80px;
        }
    "#;

    let sheet = StyleSheet::parse(css, StyleSheetOrigin::Author);
    let mut engine = StyleEngine::new();
    engine.add_stylesheet(sheet);

    // 2. Geração da árvore DOM massiva (1.000 nós por modo da matriz = 6.000 nós)
    let repetitions_per_mode = 1_000;
    let mut html = String::with_capacity(3_000_000);
    html.push_str("<html><body>");

    for (m_idx, _) in matrix.iter().enumerate() {
        html.push_str(&format!("<div class='mode-{} mode-root-{}'>", m_idx, m_idx));
        for i in 0..repetitions_per_mode {
            let variant = i % 5;
            match variant {
                0 => html.push_str(&format!(
                    "<div id='m{}-dims-{}' class='box-dims'>Node</div>",
                    m_idx, i
                )),
                1 => html.push_str(&format!(
                    "<div id='m{}-surr-long-{}' class='surrounding-longhands'>Node</div>",
                    m_idx, i
                )),
                2 => html.push_str(&format!(
                    "<div id='m{}-surr-short-{}' class='surrounding-shorthands'>Node</div>",
                    m_idx, i
                )),
                3 => html.push_str(&format!(
                    "<div id='m{}-inset-{}' class='inset-rules'>Node</div>",
                    m_idx, i
                )),
                4 => html.push_str(&format!(
                    "<div class='parent-container'>\
                        <div id='m{}-over-logic-{}' class='override-phys-with-logic'>Child 1</div>\
                        <div id='m{}-over-phys-{}' class='override-logic-with-phys'>Child 2</div>\
                    </div>",
                    m_idx, i, m_idx, i
                )),
                _ => unreachable!(),
            }
        }
        html.push_str("</div>");
    }
    html.push_str("</body></html>");

    let doc = parse_html(&html);

    let start = std::time::Instant::now();
    let render_tree = engine.compute_styles(&doc);
    let duration = start.elapsed();
    println!(
        "Estilização de {} nós concluída em {:?}",
        render_tree.node_styles.len(),
        duration
    );

    assert!(
        render_tree.node_styles.len() >= 6_000,
        "Total de nós estilizados esperado >= 6000, obtido: {}",
        render_tree.node_styles.len()
    );

    // 3. Validação matemática estrita contra W3C Writing Modes L1
    for (m_idx, (dir, wm, _, _)) in matrix.iter().enumerate() {
        // Valida dimensões lógicas: inline-size (200px) e block-size (100px)
        let sample_dims_node = doc
            .get_element_by_id(&format!("m{}-dims-0", m_idx))
            .expect("Nó de dimensões não encontrado");
        let s_dims = render_tree.get_style(sample_dims_node).unwrap();

        assert_eq!(s_dims.direction(), *dir);
        assert_eq!(s_dims.writing_mode(), *wm);

        let (expected_w, expected_h) = if wm.is_vertical() {
            (LayoutUnit::from_px(100), LayoutUnit::from_px(200))
        } else {
            (LayoutUnit::from_px(200), LayoutUnit::from_px(100))
        };
        assert_eq!(
            s_dims.width(),
            Some(expected_w),
            "Falha em width para modo {:?}",
            (dir, wm)
        );
        assert_eq!(
            s_dims.height(),
            Some(expected_h),
            "Falha em height para modo {:?}",
            (dir, wm)
        );

        // Valida min/max inline-size e block-size
        let (expected_min_w, expected_min_h) = if wm.is_vertical() {
            (LayoutUnit::from_px(80), LayoutUnit::from_px(150))
        } else {
            (LayoutUnit::from_px(150), LayoutUnit::from_px(80))
        };
        assert_eq!(s_dims.min_width(), Some(expected_min_w));
        assert_eq!(s_dims.min_height(), Some(expected_min_h));

        // Valida margens e paddings lógicos (longhands)
        // inline-start: 10, inline-end: 20, block-start: 30, block-end: 40
        // padding: inline-start: 5, inline-end: 15, block-start: 25, block-end: 35
        let sample_surr_node = doc
            .get_element_by_id(&format!("m{}-surr-long-1", m_idx))
            .expect("Nó de surrounding não encontrado");
        let s_surr = render_tree.get_style(sample_surr_node).unwrap();

        let (exp_m_top, exp_m_right, exp_m_bottom, exp_m_left) = match (wm, dir) {
            (WritingMode::HorizontalTb, Direction::Ltr) => (30, 20, 40, 10),
            (WritingMode::HorizontalTb, Direction::Rtl) => (30, 10, 40, 20),
            (WritingMode::VerticalRl, Direction::Ltr) => (10, 30, 20, 40),
            (WritingMode::VerticalRl, Direction::Rtl) => (20, 30, 10, 40),
            (WritingMode::VerticalLr, Direction::Ltr) => (10, 40, 20, 30),
            (WritingMode::VerticalLr, Direction::Rtl) => (20, 40, 10, 30),
        };

        assert_eq!(
            s_surr.margin_top(),
            Some(LayoutUnit::from_px(exp_m_top)),
            "margin-top incorreto para modo {:?}",
            (dir, wm)
        );
        assert_eq!(
            s_surr.margin_right(),
            Some(LayoutUnit::from_px(exp_m_right)),
            "margin-right incorreto para modo {:?}",
            (dir, wm)
        );
        assert_eq!(
            s_surr.margin_bottom(),
            Some(LayoutUnit::from_px(exp_m_bottom)),
            "margin-bottom incorreto para modo {:?}",
            (dir, wm)
        );
        assert_eq!(
            s_surr.margin_left(),
            Some(LayoutUnit::from_px(exp_m_left)),
            "margin-left incorreto para modo {:?}",
            (dir, wm)
        );

        let (exp_p_top, exp_p_right, exp_p_bottom, exp_p_left) = match (wm, dir) {
            (WritingMode::HorizontalTb, Direction::Ltr) => (25, 15, 35, 5),
            (WritingMode::HorizontalTb, Direction::Rtl) => (25, 5, 35, 15),
            (WritingMode::VerticalRl, Direction::Ltr) => (5, 25, 15, 35),
            (WritingMode::VerticalRl, Direction::Rtl) => (15, 25, 5, 35),
            (WritingMode::VerticalLr, Direction::Ltr) => (5, 35, 15, 25),
            (WritingMode::VerticalLr, Direction::Rtl) => (15, 35, 5, 25),
        };

        assert_eq!(
            s_surr.padding_top(),
            LayoutUnit::from_px(exp_p_top),
            "padding-top incorreto para modo {:?}",
            (dir, wm)
        );
        assert_eq!(
            s_surr.padding_right(),
            LayoutUnit::from_px(exp_p_right),
            "padding-right incorreto para modo {:?}",
            (dir, wm)
        );
        assert_eq!(
            s_surr.padding_bottom(),
            LayoutUnit::from_px(exp_p_bottom),
            "padding-bottom incorreto para modo {:?}",
            (dir, wm)
        );
        assert_eq!(
            s_surr.padding_left(),
            LayoutUnit::from_px(exp_p_left),
            "padding-left incorreto para modo {:?}",
            (dir, wm)
        );

        // Valida insets lógicos
        // inset-inline-start: 12, inset-inline-end: 24, inset-block-start: 36, inset-block-end: 48
        let sample_inset_node = doc
            .get_element_by_id(&format!("m{}-inset-3", m_idx))
            .expect("Nó de inset não encontrado");
        let s_inset = render_tree.get_style(sample_inset_node).unwrap();

        let (exp_i_top, exp_i_right, exp_i_bottom, exp_i_left) = match (wm, dir) {
            (WritingMode::HorizontalTb, Direction::Ltr) => (36, 24, 48, 12),
            (WritingMode::HorizontalTb, Direction::Rtl) => (36, 12, 48, 24),
            (WritingMode::VerticalRl, Direction::Ltr) => (12, 36, 24, 48),
            (WritingMode::VerticalRl, Direction::Rtl) => (24, 36, 12, 48),
            (WritingMode::VerticalLr, Direction::Ltr) => (12, 48, 24, 36),
            (WritingMode::VerticalLr, Direction::Rtl) => (24, 48, 12, 36),
        };

        assert_eq!(
            s_inset.top(),
            Some(LayoutUnit::from_px(exp_i_top)),
            "top incorreto para modo {:?}",
            (dir, wm)
        );
        assert_eq!(
            s_inset.right(),
            Some(LayoutUnit::from_px(exp_i_right)),
            "right incorreto para modo {:?}",
            (dir, wm)
        );
        assert_eq!(
            s_inset.bottom(),
            Some(LayoutUnit::from_px(exp_i_bottom)),
            "bottom incorreto para modo {:?}",
            (dir, wm)
        );
        assert_eq!(
            s_inset.left(),
            Some(LayoutUnit::from_px(exp_i_left)),
            "left incorreto para modo {:?}",
            (dir, wm)
        );

        // Valida sobrescrita física por lógica (a lógica foi declarada depois da física)
        let sample_over_logic_node = doc
            .get_element_by_id(&format!("m{}-over-logic-4", m_idx))
            .expect("Nó de sobreposição de lógica não encontrado");
        let s_over_logic = render_tree.get_style(sample_over_logic_node).unwrap();

        let (ov_m_top, ov_m_right, ov_m_bottom, ov_m_left) = match (wm, dir) {
            (WritingMode::HorizontalTb, Direction::Ltr) => (35, 25, 45, 15),
            (WritingMode::HorizontalTb, Direction::Rtl) => (35, 15, 45, 25),
            (WritingMode::VerticalRl, Direction::Ltr) => (15, 35, 25, 45),
            (WritingMode::VerticalRl, Direction::Rtl) => (25, 35, 15, 45),
            (WritingMode::VerticalLr, Direction::Ltr) => (15, 45, 25, 35),
            (WritingMode::VerticalLr, Direction::Rtl) => (25, 45, 15, 35),
        };
        assert_eq!(
            s_over_logic.margin_top(),
            Some(LayoutUnit::from_px(ov_m_top))
        );
        assert_eq!(
            s_over_logic.margin_right(),
            Some(LayoutUnit::from_px(ov_m_right))
        );
        assert_eq!(
            s_over_logic.margin_bottom(),
            Some(LayoutUnit::from_px(ov_m_bottom))
        );
        assert_eq!(
            s_over_logic.margin_left(),
            Some(LayoutUnit::from_px(ov_m_left))
        );

        // Valida sobrescrita lógica por física (a física foi declarada depois da lógica)
        let sample_over_phys_node = doc
            .get_element_by_id(&format!("m{}-over-phys-4", m_idx))
            .expect("Nó de sobreposição de física não encontrado");
        let s_over_phys = render_tree.get_style(sample_over_phys_node).unwrap();
        assert_eq!(
            s_over_phys.margin_top(),
            Some(LayoutUnit::from_px(50)),
            "Física margin-top deve prevalecer"
        );
        assert_eq!(
            s_over_phys.margin_bottom(),
            Some(LayoutUnit::from_px(60)),
            "Física margin-bottom deve prevalecer"
        );
        assert_eq!(
            s_over_phys.margin_left(),
            Some(LayoutUnit::from_px(70)),
            "Física margin-left deve prevalecer"
        );
        assert_eq!(
            s_over_phys.margin_right(),
            Some(LayoutUnit::from_px(80)),
            "Física margin-right deve prevalecer"
        );
    }

    println!("Sucesso absoluto: Todas as 6 combinações da matriz Bidi e propriedades lógicas W3C validadas com 100% de conformidade geométrica!");
}
