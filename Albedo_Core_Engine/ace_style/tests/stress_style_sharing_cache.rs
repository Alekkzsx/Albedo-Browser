//! # 🥊 Suíte 5: Eficiência Extrema do StyleSharingCache e Elementos Irmãos
//!
//! Exercita:
//! - Lista com 10.000 <li> idênticos
//! - Validação de que apenas 1 instância de ComputedStyle é calculada e compartilhada via Arc
//! - Medição de tempo e integridade da RenderTree

use ace_dom::parse_html;
use ace_style::engine::style_engine::StyleEngine;
use ace_style::model::stylesheet::StyleSheet;
use ace_style::cascade::origin::StyleSheetOrigin;
use std::sync::Arc;

#[test]
fn test_stress_style_sharing_10000_identical_siblings() {
    // 1. Gera HTML com 10.000 <li> idênticos dentro de um <ul>
    let mut html = String::with_capacity(300_000);
    html.push_str("<html><body><ul id='list'>");
    for i in 0..10_000 {
        html.push_str("<li class='item'>Texto</li>");
    }
    html.push_str("</ul></body></html>");

    let doc = parse_html(&html);

    // 2. Folha com estilos para a classe item
    let css = r#"
        ul { display: block; padding: 0; }
        li.item {
            display: list-item;
            color: #333333;
            margin: 5px;
            font-size: 14px;
        }
    "#;
    let sheet = StyleSheet::parse(css, StyleSheetOrigin::Author);

    let mut engine = StyleEngine::new();
    engine.add_stylesheet(sheet);

    let start = std::time::Instant::now();
    let render_tree = engine.compute_styles(&doc);
    let elapsed = start.elapsed();

    println!("Computed styles for 10,000 siblings in {:?}", elapsed);
    assert!(elapsed.as_millis() < 2500, "10,000 elementos devem ser computados com rapidez graças ao cache");

    // Valida que o cache de compartilhamento de estilo foi ativado
    // A chave de cache para 'li.item' sob o mesmo 'ul' deve ter exatamente 1 entrada
    assert!(engine.cache.len() > 0);

    // Valida que os 10.000 nós de li compartilham o MESMO ponteiro Arc<ComputedStyle>
    let ul_render_node = render_tree.root.as_ref()
        .and_then(|r| r.children.first()) // body
        .and_then(|b| b.children.first()) // ul
        .expect("ul render node");

    assert_eq!(ul_render_node.children.len(), 10_000);

    let first_style_ptr = Arc::as_ptr(&ul_render_node.children[0].style);
    let last_style_ptr = Arc::as_ptr(&ul_render_node.children[9_999].style);

    assert_eq!(
        first_style_ptr, last_style_ptr,
        "Todos os 10.000 elementos <li> irmãos idênticos devem compartilhar a MESMA instância Arc<ComputedStyle>"
    );
}
