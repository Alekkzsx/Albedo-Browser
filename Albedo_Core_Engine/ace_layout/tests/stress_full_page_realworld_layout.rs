//! # Stress Suite 8: Full Real-World Page Layout (Wikipedia & 7-Layer Stacking Contexts)
//!
//! Requisito Normativo: W3C CSS 2.2 Appendix E (Stacking Contexts 7 Camadas),
//! Dumps Determinísticos de Layout e Orquestração Completa Integrada.
//! Valida:
//! 1. Construção de página completa estilo Wikipedia integrando simultaneamente:
//!    - Flexbox (Header com barra de navegação, logo e search)
//!    - CSS Grid (Layout geral de duas colunas e sub-grade de galeria)
//!    - BFC (Artigo enciclopédico, cabeçalhos, seções e citações)
//!    - IFC (Parágrafos densos com tipografia mista)
//!    - TFC (Infobox estruturada com tabela de metadados e estatísticas)
//!    - Posicionamento fora de fluxo (Header fixo e tooltip absoluto)
//!    - Árvore de Stacking Contexts canônica de 7 camadas (z-index < 0, z-index > 0, opacity < 1.0)
//! 2. Anotação canônica de camadas de pintura (Layers 1 a 7) e ordenação estrita.
//! 3. Gerador determinístico de dumps textuais de layout (`LayoutDump`).
//! 4. SLA de performance: layout completo da página em menos de 15 ms.

mod common;

use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::LogicalSize;
use ace_layout::engine::dump::LayoutDump;
use ace_layout::engine::layout_engine::LayoutEngine;
use ace_layout::positioning::stacking::StackingLayer;
use common::{html_to_dom_and_render_tree, BenchmarkTimer};

#[test]
fn test_stress_full_page_wikipedia_realworld_layout() {
    let timer = BenchmarkTimer::start();

    // HTML realista e completo estruturado como um artigo denso da Wikipedia
    let html = r#"
    <div style="width: 1200px">
      <!-- 1. Header: Barra de navegação com Flexbox -->
      <div style="display: flex; width: 1200px; height: 60px">
        <div style="width: 180px; height: 40px">Wikipedia Logo</div>
        <div style="width: 500px; height: 35px">Search Wikipedia...</div>
        <div style="width: 200px; height: 35px">User Tools</div>
      </div>

      <!-- 2. Corpo: CSS Grid com 2 colunas (TOC Sidebar e Artigo Principal) -->
      <div style="display: grid; width: 1200px">
        <!-- Sidebar TOC -->
        <div style="width: 250px">
          <div>Contents</div>
          <div>1 History</div>
          <div>2 Syntax and Features</div>
          <div>3 Memory Safety</div>
          <div>4 Ecosystem</div>
        </div>

        <!-- Conteúdo do Artigo Principal -->
        <div style="width: 900px">
          <!-- Título principal (BFC) -->
          <div style="width: 900px; height: 50px">Rust (programming language)</div>

          <!-- Infobox: Tabela estruturada TFC -->
          <table style="display: table; width: 320px">
            <tr style="display: table-row">
              <td style="display: table-cell; width: 120px">Paradigm</td>
              <td style="display: table-cell; width: 200px">Multi-paradigm: functional, concurrent</td>
            </tr>
            <tr style="display: table-row">
              <td style="display: table-cell; width: 120px">Designed by</td>
              <td style="display: table-cell; width: 200px">Graydon Hoare</td>
            </tr>
            <tr style="display: table-row">
              <td style="display: table-cell; width: 120px">First appeared</td>
              <td style="display: table-cell; width: 200px">July 7, 2010</td>
            </tr>
          </table>

          <!-- Parágrafos densos (IFC) -->
          <p style="width: 560px">
            Rust is a multi-paradigm, general-purpose programming language that emphasizes performance,
            type safety, and concurrency. Rust enforces memory safety without requiring a garbage collector.
          </p>

          <p style="width: 560px">
            Software developers use Rust to create software applications, simulation engines, operating systems,
            file systems, browser components, and virtual reality simulation engines.
          </p>

          <!-- Galeria de Imagens: Sub-grade CSS Grid -->
          <div style="display: grid; width: 600px">
            <div style="width: 180px; height: 120px">Rustacean Mascot Ferris</div>
            <div style="width: 180px; height: 120px">Cargo Package Manager Logo</div>
            <div style="width: 180px; height: 120px">Rust Compiler Architecture Diagram</div>
          </div>

          <!-- Elementos com Stacking Contexts e Posicionamento -->
          <!-- Top Bar Fixa (ICB Viewport) -->
          <div style="position: fixed; top: 0px; left: 0px; width: 1200px; height: 30px; z-index: 100">
            Official Wikipedia Fixed Top Banner
          </div>

          <!-- Tooltip Absoluto -->
          <div style="position: absolute; top: 20px; right: 30px; width: 150px; height: 50px; z-index: 50">
            Citation Reference [1]
          </div>

          <!-- Stacking Context Negativo (Layer 2) -->
          <div style="position: relative; z-index: -10; width: 200px; height: 40px">
            Background Watermark Layer
          </div>

          <!-- Stacking Context por Opacidade < 1.0 -->
          <div style="opacity: 0.75; width: 300px; height: 60px">
            Translucent Notification Notice Box
          </div>
        </div>
      </div>
    </div>
    "#;

    let (doc, render_tree) = html_to_dom_and_render_tree(html);

    let mut engine = LayoutEngine::new();
    let viewport = LogicalSize::new(LayoutUnit::from_px(1200), LayoutUnit::from_px(900));

    let fragment_tree = engine.layout_tree(&render_tree, &doc, viewport).unwrap();

    // 1. Valida integridade da árvore de fragmentos física
    assert!(
        fragment_tree.root.is_some(),
        "Root fragment must be present"
    );
    let total_fragments = fragment_tree.total_fragments();
    assert!(
        total_fragments >= 20,
        "Full page layout must produce multiple fragments, found {}",
        total_fragments
    );

    // 2. Valida Stacking Contexts e a ordem canônica das 7 camadas de pintura
    let paint_order = fragment_tree.paint_order();
    assert!(!paint_order.is_empty(), "Paint order must not be empty");

    // Encontra elementos em camadas canônicas
    let mut found_neg_layer = false;
    let mut found_pos_layer = false;
    let mut found_inflow_layer = false;

    let mut neg_paint_index = None;
    let mut pos_paint_index = None;
    let mut inflow_paint_index = None;

    for (idx, fragment) in paint_order.iter().enumerate() {
        match fragment.stacking_layer {
            StackingLayer::Layer2NegativeZIndex => {
                found_neg_layer = true;
                if neg_paint_index.is_none() {
                    neg_paint_index = Some(idx);
                }
            }
            StackingLayer::Layer3BlockBackgroundsAndBorders => {
                found_inflow_layer = true;
                if inflow_paint_index.is_none() {
                    inflow_paint_index = Some(idx);
                }
            }
            StackingLayer::Layer7PositiveZIndex => {
                found_pos_layer = true;
                if pos_paint_index.is_none() {
                    pos_paint_index = Some(idx);
                }
            }
            _ => {}
        }
    }

    assert!(found_neg_layer, "Must detect Layer 2 (negative z-index)");
    assert!(found_pos_layer, "Must detect Layer 7 (positive z-index)");
    assert!(
        found_inflow_layer,
        "Must detect Layer 3 (in-flow block backgrounds)"
    );

    // Invariante normativo: Camada 2 pinta ANTES da Camada 3, que pinta ANTES da Camada 7
    if let (Some(neg), Some(inflow), Some(pos)) =
        (neg_paint_index, inflow_paint_index, pos_paint_index)
    {
        assert!(
            neg < inflow,
            "CSS 2.2 Appendix E violation: Layer 2 ({}) must paint before Layer 3 ({})",
            neg,
            inflow
        );
        assert!(
            inflow < pos,
            "CSS 2.2 Appendix E violation: Layer 3 ({}) must paint before Layer 7 ({})",
            inflow,
            pos
        );
    }

    // 3. Valida Layout Dump textual determinístico
    let dump = LayoutDump::dump_fragment_tree(&fragment_tree);
    assert!(!dump.is_empty(), "Layout dump must not be empty");
    assert!(
        dump.contains("FragmentTree viewport: 1200.00x900.00"),
        "Dump must include exact fixed-point viewport dimensions"
    );
    assert!(
        dump.contains("layer:"),
        "Dump must annotate stacking layers for every box"
    );

    let report = timer.report(1, "Full_Wikipedia_Page_Layout");
    // SLA de performance: layout completo em menos de 15 ms
    assert!(
        report.elapsed_secs < 0.015,
        "Full real-world page layout took {:.4}s (exceeded 15 ms SLA limit)",
        report.elapsed_secs
    );
}
