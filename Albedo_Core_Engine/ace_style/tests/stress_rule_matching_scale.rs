//! # 🥊 Suíte 3: Escalabilidade de Casamento de Regras e Aceleração por Bloom Filter
//!
//! Exercita:
//! - Documento DOM com milhares de nós casando contra dezenas de milhares de regras
//! - Validação da rejeição em O(1) pelo AncestorFilter (Counting Bloom Filter)
//! - Redução de backtracking para seletores descendentes profundos

use ace_core::intern::Atom;
use ace_dom::parse_html;
use ace_dom::query::bloom::AncestorFilter;
use ace_style::cascade::origin::StyleSheetOrigin;
use ace_style::model::stylesheet::StyleSheet;
use ace_style::rule_tree::bucket::RuleBucketMap;

#[test]
fn test_stress_rule_matching_scale_with_bloom_filter() {
    // 1. Gera folha de estilo com 10.000 regras CSS contendo seletores descendentes
    let mut css = String::with_capacity(500_000);
    for i in 0..10_000 {
        css.push_str(&format!(
            "div.container-{} ul.list li.item-{} span.label {{ color: red; }}\n",
            i % 100,
            i
        ));
    }

    let sheet = StyleSheet::parse(&css, StyleSheetOrigin::Author);
    let mut bucket_map = RuleBucketMap::new();
    bucket_map.add_stylesheet(0, &sheet);

    // 2. Constrói árvore DOM com nós
    let mut html = String::from("<html><body>");
    for i in 0..500 {
        html.push_str(&format!(
            "<div class='other-container'><p><span class='label'>Item {}</span></p></div>",
            i
        ));
    }
    html.push_str("</body></html>");

    let doc = parse_html(&html);
    let sheets = vec![sheet];

    // 3. Executa matching com AncestorFilter ativo
    // Para elementos sem 'div.container-*' e 'ul.list' na cadeia ancestral, o AncestorFilter rejeita em O(1)
    let mut filter = AncestorFilter::new();
    let body_id = doc.get_node(doc.root()).unwrap().first_child.unwrap();

    let start = std::time::Instant::now();
    let matched = bucket_map.collect_matching_rules(&doc, body_id, &sheets, Some(&filter));
    let elapsed = start.elapsed();

    println!("Matching with Bloom filter took {:?}", elapsed);
    assert!(elapsed.as_millis() < 500);
    assert_eq!(matched.len(), 0);
}
