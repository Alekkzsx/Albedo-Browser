//! # 🥊 Bateria 4: Injeção Hierárquica de Pseudo-Elementos (::before & ::after)
//!
//! Exercita:
//! - Árvore densa com múltiplos níveis de aninhamento e regras de pseudo-elementos.
//! - Injeção precisa de nós do tipo `RenderNodeKind::AnonymousBefore` e `AnonymousAfter`.
//! - Respeito rigoroso à ordem de pintura (painting order: ::before -> filhos DOM -> ::after).
//! - Propagação da propriedade `content` e herança contextual de estilos.

use ace_dom::parse_html;
use ace_style::cascade::origin::StyleSheetOrigin;
use ace_style::engine::style_engine::{RenderNodeKind, StyleEngine};
use ace_style::model::stylesheet::StyleSheet;

#[test]
fn test_stress_pseudo_elements_hierarchical() {
    let quote_count = 1_000;
    println!(
        "Gerando arvore com {} blocos com ::before e ::after...",
        quote_count
    );

    let mut html = String::with_capacity(500_000);
    html.push_str("<html><body><div id='root'>");
    for i in 0..quote_count {
        html.push_str(&format!(
            "<div class='quote-card'>\
                <span class='badge'>Tag {}</span>\
                <blockquote class='quote'>Citacao de teste {}</blockquote>\
                <div class='item'>Item da lista</div>\
            </div>",
            i, i
        ));
    }
    html.push_str("</div></body></html>");

    let doc = parse_html(&html);

    let css = r#"
        .quote::before {
            content: "“";
            color: #888888;
            font-size: 24px;
        }
        .quote::after {
            content: "”";
            color: #888888;
            font-size: 24px;
        }
        .badge::before {
            content: "★ ";
            color: #ffaa00;
        }
        .item::after {
            content: " ->";
            color: #333333;
        }
    "#;
    let sheet = StyleSheet::parse(css, StyleSheetOrigin::Author);

    let mut engine = StyleEngine::new();
    engine.add_stylesheet(sheet);

    let render_tree = engine.compute_styles(&doc);
    assert!(render_tree.root.is_some());

    let mut total_quotes_checked = 0;
    let mut total_badges_checked = 0;
    let mut total_items_checked = 0;
    let mut total_anonymous_before = 0;
    let mut total_anonymous_after = 0;

    // Função de validação recursiva sobre a RenderTree
    fn inspect_node(
        node: &ace_style::engine::RenderTreeNode,
        quotes: &mut usize,
        badges: &mut usize,
        items: &mut usize,
        before_count: &mut usize,
        after_count: &mut usize,
    ) {
        if node.kind == RenderNodeKind::AnonymousBefore {
            *before_count += 1;
        } else if node.kind == RenderNodeKind::AnonymousAfter {
            *after_count += 1;
        }

        // Se tiver filhos, valida ordem de pintura
        if node.children.len() >= 2 {
            // Se tiver ::before, ele deve OBRIGATORIAMENTE ser o primeiro filho
            let first_child = &node.children[0];
            if first_child.kind == RenderNodeKind::AnonymousBefore {
                assert!(first_child.style.content().is_some());
            }

            // Se tiver ::after, ele deve OBRIGATORIAMENTE ser o último filho
            let last_child = node.children.last().unwrap();
            if last_child.kind == RenderNodeKind::AnonymousAfter {
                assert!(last_child.style.content().is_some());
            }
        }

        // Contabiliza componentes específicos
        if node.children.len() == 3 {
            // .quote tem ::before, texto DOM, ::after
            let has_before = node.children[0].kind == RenderNodeKind::AnonymousBefore
                && node.children[0].style.content() == Some("“");
            let has_after = node.children[2].kind == RenderNodeKind::AnonymousAfter
                && node.children[2].style.content() == Some("”");
            if has_before && has_after {
                *quotes += 1;
            }
        } else if node.children.len() == 2 {
            if node.children[0].kind == RenderNodeKind::AnonymousBefore
                && node.children[0].style.content() == Some("★ ")
            {
                *badges += 1;
            } else if node.children[1].kind == RenderNodeKind::AnonymousAfter
                && node.children[1].style.content() == Some(" ->")
            {
                *items += 1;
            }
        }

        for child in &node.children {
            inspect_node(child, quotes, badges, items, before_count, after_count);
        }
    }

    if let Some(ref root) = render_tree.root {
        inspect_node(
            root,
            &mut total_quotes_checked,
            &mut total_badges_checked,
            &mut total_items_checked,
            &mut total_anonymous_before,
            &mut total_anonymous_after,
        );
    }

    println!(
        "Validados: Quotes={}, Badges={}, Items={}, Total Anonymous Before={}, After={}",
        total_quotes_checked,
        total_badges_checked,
        total_items_checked,
        total_anonymous_before,
        total_anonymous_after
    );

    // Critério Inegociável: 100% dos nós anônimos injetados corretamente
    assert_eq!(
        total_quotes_checked, quote_count,
        "100% dos quotes devem possuir ::before e ::after injetados"
    );
    assert_eq!(
        total_badges_checked, quote_count,
        "100% dos badges devem possuir ::before injetado"
    );
    assert_eq!(
        total_items_checked, quote_count,
        "100% dos items devem possuir ::after injetado"
    );
    assert_eq!(
        total_anonymous_before,
        quote_count * 2,
        "Total de nós AnonymousBefore deve ser exatamente 2.000"
    );
    assert_eq!(
        total_anonymous_after,
        quote_count * 2,
        "Total de nós AnonymousAfter deve ser exatamente 2.000"
    );
}
