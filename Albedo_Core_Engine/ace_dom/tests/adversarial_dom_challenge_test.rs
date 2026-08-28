//! # Testes Adversariais e Desafio de Estresse: DOM, Query, Range e Sanitizer
//!
//! Desafia rigorosamente:
//! 1. Performance e integridade de árvores DOM com 10.000+ nós sob mutações rápidas
//! 2. Motor CSS4 e Counting Bloom Filter sob colisões e cadeias profundas (500+ níveis)
//! 3. Auto-ajuste de Live Range sob splits, remoções e inserções adversariais
//! 4. HTML Sanitizer contra vetores de bypass de XSS e esquemas maliciosos

use ace_dom::node::Namespace;
use ace_dom::query::selector::{ComplexSelector, CompoundSelector};
use ace_dom::query::AncestorFilter;
use ace_dom::range::{LiveRangeHandle, LiveRangeRegistry, Range};
use ace_dom::sanitizer::{HTMLSanitizer, SanitizerConfig};
use ace_dom::tree::Document;
use std::time::Instant;

// =========================================================================
// 1. DESAFIO: ÁRVORE DOM GRANDE (10.000+ NÓS) E MUTAÇÕES RÁPIDAS
// =========================================================================

#[test]
fn test_large_dom_tree_10k_nodes_creation_and_traversal() {
    let start = Instant::now();
    let mut doc = Document::new(Some("https://example.com/stress-10k"));
    let root = doc.root();

    let container = doc.create_element("div", Namespace::Html);
    doc.append_child(root, container).unwrap();

    let node_count = 10_000;
    let mut child_ids = Vec::with_capacity(node_count);

    // Aloca 10.000 nós variados (elementos, textos, comentários)
    for i in 0..node_count {
        let node_id = match i % 3 {
            0 => {
                let el = doc.create_element("p", Namespace::Html);
                if let Some(el_data) = doc.get_node_mut(el).and_then(|n| n.as_element_mut()) {
                    el_data.set_attribute("id", format!("item-{}", i));
                    el_data.set_attribute("class", format!("list-item item-mod-{}", i % 10));
                    el_data.set_attribute("data-index", i.to_string());
                }
                el
            }
            1 => doc.create_text_node(format!("Texto do nó número {}", i)),
            _ => doc.create_comment(format!("Comentário do nó {}", i)),
        };

        doc.append_child(container, node_id).unwrap();
        child_ids.push(node_id);
    }

    let elapsed_alloc = start.elapsed();
    println!("Tempo para alocar 10.000 nós DOM: {:?}", elapsed_alloc);
    assert_eq!(doc.node_count(), node_count + 2); // root + container + 10k

    // Validação da integridade dos ponteiros da lista duplamente encadeada de 10.000 nós
    let mut count_forward = 0;
    let mut curr = doc.first_child(container);
    let mut prev_seen = None;

    while let Some(c_id) = curr {
        count_forward += 1;
        let node = doc.get_node(c_id).expect("Nó deve existir na arena");
        assert_eq!(node.parent, Some(container));
        assert_eq!(node.prev_sibling, prev_seen);

        prev_seen = Some(c_id);
        curr = node.next_sibling;
    }
    assert_eq!(count_forward, node_count);

    // Validação reversa (last_child para trás)
    let mut count_backward = 0;
    let mut curr_rev = doc.last_child(container);
    let mut next_seen = None;

    while let Some(c_id) = curr_rev {
        count_backward += 1;
        let node = doc.get_node(c_id).expect("Nó deve existir na arena");
        assert_eq!(node.next_sibling, next_seen);

        next_seen = Some(c_id);
        curr_rev = node.prev_sibling;
    }
    assert_eq!(count_backward, node_count);
}

#[test]
fn test_rapid_mutations_interleaved_on_large_dom() {
    let mut doc = Document::new(None);
    let root = doc.root();
    let container = doc.create_element("main", Namespace::Html);
    doc.append_child(root, container).unwrap();

    let initial_count = 3_000;
    let mut nodes = Vec::with_capacity(initial_count);
    for _i in 0..initial_count {
        let el = doc.create_element("div", Namespace::Html);
        doc.append_child(container, el).unwrap();
        nodes.push(el);
    }

    let start_mut = Instant::now();

    // 1. Inserções rápidas com insert_before no meio e no início
    for i in 0..1_000 {
        let new_el = doc.create_element("span", Namespace::Html);
        let ref_node = if i % 2 == 0 {
            doc.first_child(container)
        } else {
            Some(nodes[i * 2])
        };
        doc.insert_before(container, new_el, ref_node).unwrap();
    }

    // 2. Substituições com replace_child
    for i in 0..500 {
        let replacement = doc.create_element("article", Namespace::Html);
        let target = nodes[i * 4];
        doc.replace_child(container, replacement, target).unwrap();
    }

    // 3. Remoções rápidas com remove_child
    for &target in nodes.iter().take(1_000) {
        if doc.get_node(target).and_then(|n| n.parent) == Some(container) {
            doc.remove_child(container, target).unwrap();
            let node = doc.get_node(target).unwrap();
            assert_eq!(node.parent, None);
            assert_eq!(node.prev_sibling, None);
            assert_eq!(node.next_sibling, None);
        }
    }

    let elapsed_mut = start_mut.elapsed();
    println!("Tempo de mutações rápidas interleaved: {:?}", elapsed_mut);

    // Validação estrita de não-ciclicidade e consistência dos ponteiros
    let mut visited = std::collections::HashSet::new();
    let mut curr = doc.first_child(container);
    while let Some(c_id) = curr {
        assert!(visited.insert(c_id), "Ciclo detectado nos ponteiros de irmãos!");
        let node = doc.get_node(c_id).unwrap();
        curr = node.next_sibling;
    }
}

// =========================================================================
// 2. DESAFIO: MOTOR CSS4 & ANCESTOR BLOOM FILTER SOB COLISÕES E PROFUNDIDADE
// =========================================================================

#[test]
fn test_deep_descendant_chain_500_levels() {
    let mut doc = Document::new(None);
    let mut curr_parent = doc.root();

    let depth = 500;
    let mut level_ids = Vec::with_capacity(depth);

    for level in 0..depth {
        let tag = if level == depth - 1 { "span" } else { "div" };
        let el = doc.create_element(tag, Namespace::Html);
        if let Some(el_data) = doc.get_node_mut(el).and_then(|n| n.as_element_mut()) {
            el_data.set_attribute("class", format!("level-{} branch-alpha", level));
            el_data.set_attribute("id", format!("node-lvl-{}", level));
        }
        doc.append_child(curr_parent, el).unwrap();
        level_ids.push(el);
        curr_parent = el;
    }

    let leaf_id = *level_ids.last().unwrap();

    // 1. Seletor descendente simples
    let sel1 = ComplexSelector::parse("div.level-0 div.level-250 span.level-499").unwrap();
    assert!(sel1.matches(&doc, leaf_id), "RTL matcher deve casar em árvore com 500 níveis");

    // 2. Seletor com combinador de filho direto no final
    let sel2 = ComplexSelector::parse("div.level-498 > span.level-499").unwrap();
    assert!(sel2.matches(&doc, leaf_id));

    // 3. Seletor descendente que NÃO existe (deve falhar sem estouro de pilha)
    let sel_neg = ComplexSelector::parse("div.level-0 div.nonexistent span.level-499").unwrap();
    assert!(!sel_neg.matches(&doc, leaf_id), "Seletor inexistente deve retornar false");
}

#[test]
fn test_ancestor_bloom_filter_saturation_and_collision_soundness() {
    let mut filter = AncestorFilter::new();

    // Cria um ElementData com tag arbitrária
    let mut el = ace_dom::node::ElementData::new("div", Namespace::Html);
    el.set_attribute("id", "container");
    el.set_attribute("class", "alpha beta gamma");

    // Push 300 vezes para testar saturação de buckets u8 (limite 255)
    for _ in 0..300 {
        filter.push_element(&el);
    }

    let compound = CompoundSelector::parse("div#container.alpha").unwrap();
    assert!(!filter.fast_reject(&compound), "Elemento presente não deve ser rejeitado");

    // Pop 255 vezes (ou 300 vezes)
    for _ in 0..255 {
        filter.pop_element(&el);
    }

    // Se o contador saturou e decrementou, restam 45 instâncias na árvore lógica.
    // O filtro não deve dar falso negativo se o ancestral ainda estiver conceitualmente presente.
    // (Nota: se os buckets chegaram a 0 após 255 pops de 300 pushes, verificamos a matemática do filtro).
    let div_selector = CompoundSelector::parse("div").unwrap();
    let is_rejected_after_255_pops = filter.fast_reject(&div_selector);
    println!("Saturating bloom filter: is_rejected after 255 pops of 300 pushes = {}", is_rejected_after_255_pops);
    assert!(!is_rejected_after_255_pops, "Filtro saturado não deve dar falso negativo após 255 pops quando ainda restam 45 pushes");

    // Limpa filtro
    filter.clear();
    assert!(filter.fast_reject(&compound), "Filtro limpo deve rejeitar qualquer seletor");
}

// =========================================================================
// 3. DESAFIO: LIVE RANGE AUTO-ADJUSTMENT SOB MUTAÇÕES ADVERSARIAIS
// =========================================================================

#[test]
fn test_live_range_adversarial_text_splits_and_registry() {
    let mut doc = Document::new(None);
    let root = doc.root();
    let p = doc.create_element("p", Namespace::Html);
    doc.append_child(root, p).unwrap();

    // Texto de 100 caracteres: "0123456789..."
    let full_text = "0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789";
    let text_node = doc.create_text_node(full_text);
    doc.append_child(p, text_node).unwrap();

    let mut registry = LiveRangeRegistry::new();

    // Range A: cobre do offset 10 ao offset 80
    let mut range_a = Range::new(root);
    range_a.set_start(text_node, 10);
    range_a.set_end(text_node, 80);
    let handle_a = LiveRangeHandle::new(range_a);
    registry.register(&handle_a);

    // Range B: cobre do offset 60 ao 90 (estritamente após split em 50)
    let mut range_b = Range::new(root);
    range_b.set_start(text_node, 60);
    range_b.set_end(text_node, 90);
    let handle_b = LiveRangeHandle::new(range_b);
    registry.register(&handle_b);

    // Range C: cobre do offset 0 ao 30 (estritamente antes de split em 50)
    let mut range_c = Range::new(root);
    range_c.set_start(text_node, 0);
    range_c.set_end(text_node, 30);
    let handle_c = LiveRangeHandle::new(range_c);
    registry.register(&handle_c);

    // Executa split_text no offset 50
    let right_node = doc.split_text(text_node, 50).unwrap();
    registry.notify_split_text(text_node, right_node, 50);

    // Verifica Range A: início (10 <= 50) deve permanecer em text_node (offset 10);
    // fim (80 > 50) deve mover para right_node (offset 80 - 50 = 30)
    let ra_after = handle_a.get_range();
    assert_eq!(ra_after.start.node, text_node);
    assert_eq!(ra_after.start.offset, 10);
    assert_eq!(ra_after.end.node, right_node);
    assert_eq!(ra_after.end.offset, 30);

    // Verifica Range B: ambos (60 e 90 > 50) devem mover para right_node (10 e 40)
    let rb_after = handle_b.get_range();
    assert_eq!(rb_after.start.node, right_node);
    assert_eq!(rb_after.start.offset, 10);
    assert_eq!(rb_after.end.node, right_node);
    assert_eq!(rb_after.end.offset, 40);

    // Verifica Range C: ambos (0 e 30 <= 50) devem permanecer inalterados
    let rc_after = handle_c.get_range();
    assert_eq!(rc_after.start.node, text_node);
    assert_eq!(rc_after.start.offset, 0);
    assert_eq!(rc_after.end.node, text_node);
    assert_eq!(rc_after.end.offset, 30);
}

#[test]
fn test_live_range_boundary_adjustments_on_node_removal() {
    let mut doc = Document::new(None);
    let root = doc.root();
    let list = doc.create_element("ul", Namespace::Html);
    doc.append_child(root, list).unwrap();

    let li0 = doc.create_element("li", Namespace::Html);
    let li1 = doc.create_element("li", Namespace::Html);
    let li2 = doc.create_element("li", Namespace::Html);
    doc.append_child(list, li0).unwrap();
    doc.append_child(list, li1).unwrap();
    doc.append_child(list, li2).unwrap();

    let mut registry = LiveRangeRegistry::new();

    // Range cobrindo li1
    let mut range = Range::new(root);
    range.set_start(li1, 0);
    range.set_end(li1, 0);
    let handle = LiveRangeHandle::new(range);
    registry.register(&handle);

    // Remove li1 (índice 1)
    doc.remove_child(list, li1).unwrap();
    registry.notify_node_removal(li1, list, 1);

    let r_after = handle.get_range();
    // Ponto deve ter sido movido para o pai (list) no índice onde o nó estava
    assert_eq!(r_after.start.node, list);
    assert_eq!(r_after.start.offset, 1);
    assert_eq!(r_after.end.node, list);
    assert_eq!(r_after.end.offset, 1);
}

// =========================================================================
// 4. DESAFIO: HTML SANITIZER CONTRA BYPASSES DE XSS E PROTOCOL TRICKS
// =========================================================================

#[test]
fn test_html_sanitizer_xss_bypass_attempts() {
    let payloads = vec![
        // 1. Script embutido em diferentes caixas e espaços
        (
            "<SCRIPT SRC=\"https://evil.com/xss.js\"></SCRIPT><p>Safe</p>",
            "Script tag em maiúsculas",
            true,
        ),
        // 2. Manipulador de eventos on* em maiúsculas/misturado
        (
            "<img src=\"valid.png\" ONCLICK=\"alert(1)\" OnError=\"steal()\" />",
            "Event handler ONCLICK e OnError",
            true,
        ),
        // 3. Esquema javascript: em maiúsculas / misturado
        (
            "<a href=\"JaVaScRiPt:alert(1)\">Link</a>",
            "javascript: case-insensitive",
            true,
        ),
        // 4. Esquema vbscript:
        (
            "<a href=\"vbscript:msgbox(1)\">Link</a>",
            "vbscript: scheme",
            true,
        ),
        // 5. Esquema javascript: com espaços iniciais
        (
            "<a href=\"   javascript:alert(1)\">Link</a>",
            "javascript: com espaços à esquerda",
            true,
        ),
        // 6. SVG com tag script ou use perigoso
        (
            "<svg><script>alert('svg-xss')</script><circle cx=\"10\" cy=\"10\" r=\"5\"/></svg>",
            "Script dentro de SVG",
            true,
        ),
        // 7. Elementos bloqueados por padrão (iframe, object, embed, applet)
        (
            "<iframe src=\"https://evil.com\"></iframe><object data=\"bad\"></object><embed src=\"bad\"/>",
            "Tags iframe, object, embed bloqueadas",
            true,
        ),
    ];

    let config = SanitizerConfig::default();

    for (html, desc, _should_clean) in payloads {
        let doc = HTMLSanitizer::parse_html_safe(html, Some(config.clone()));

        // Nenhum elemento script, iframe, object ou embed deve existir
        assert!(doc.query_selector("script").is_none(), "{}: <script> não foi removido!", desc);
        assert!(doc.query_selector("iframe").is_none(), "{}: <iframe> não foi removido!", desc);
        assert!(doc.query_selector("object").is_none(), "{}: <object> não foi removido!", desc);
        assert!(doc.query_selector("embed").is_none(), "{}: <embed> não foi removido!", desc);
    }
}

// =========================================================================
// 5. TESTES EMPÍRICOS ADICIONAIS: CICLOS DOM, SANITIZER CASE & RANGE DESCENDANT
// =========================================================================

#[test]
fn test_dom_ancestor_cycle_attempt() {
    let mut doc = Document::new(None);
    let root = doc.root();

    let node_a = doc.create_element("div", Namespace::Html);
    let node_b = doc.create_element("div", Namespace::Html);
    let node_c = doc.create_element("div", Namespace::Html);

    doc.append_child(root, node_a).unwrap();
    doc.append_child(node_a, node_b).unwrap();
    doc.append_child(node_b, node_c).unwrap();

    // Tenta anexar o ancestral node_a como filho do descendente node_c
    // Conforme WHATWG DOM §4.2.4: deve retornar HierarchyRequestError
    let res = doc.append_child(node_c, node_a);
    println!("Resultado de append_child(grandchild, ancestor): {:?}", res);

    assert!(res.is_err(), "append_child de ancestral sob descendente deve falhar com HierarchyRequestError");
    match res {
        Err(ace_dom::error::DomError::HierarchyRequestError(_)) => {}
        other => panic!("Esperado HierarchyRequestError, obtido: {:?}", other),
    }

    // Tenta também com insert_before
    let res_insert = doc.insert_before(node_c, node_a, None);
    assert!(res_insert.is_err(), "insert_before de ancestral sob descendente deve falhar com HierarchyRequestError");
}

#[test]
fn test_sanitizer_case_sensitivity_on_blocked_attributes_and_tags() {
    let config = SanitizerConfig::default();

    // 1. Tag em maiúsculas criada via API ou SVG
    let mut doc = Document::new(None);
    let root = doc.root();
    let script_upper = doc.create_element("SCRIPT", Namespace::Html);
    doc.append_child(root, script_upper).unwrap();

    HTMLSanitizer::sanitize_subtree(&mut doc, root, &config);
    let script_survived = doc.contains(root, script_upper);
    println!("Tag <SCRIPT> maiúscula sobreviveu ao sanitizador: {}", script_survived);
    assert!(!script_survived, "Tag <SCRIPT> em maiúsculas deve ser removida pelo sanitizador");

    // 2. Atributo bloqueado em maiúsculas (ex: FORMACTION)
    let mut doc2 = Document::new(None);
    let root2 = doc2.root();
    let btn = doc2.create_element("button", Namespace::Html);
    if let Some(el) = doc2.get_node_mut(btn).and_then(|n| n.as_element_mut()) {
        el.set_attribute("FORMACTION", "https://evil.com/leak");
    }
    doc2.append_child(root2, btn).unwrap();

    HTMLSanitizer::sanitize_subtree(&mut doc2, root2, &config);
    let formaction_survived = doc2.get_node(btn)
        .and_then(|n| n.as_element())
        .map(|el| el.has_attribute("formaction"))
        .unwrap_or(false);
    println!("Atributo FORMACTION sobreviveu ao sanitizador: {}", formaction_survived);
    assert!(!formaction_survived, "Atributo FORMACTION em maiúsculas deve ser removido pelo sanitizador");
}

#[test]
fn test_live_range_descendant_node_removal_adjustment() {
    let mut doc = Document::new(None);
    let root = doc.root();
    let container = doc.create_element("div", Namespace::Html);
    let child_p = doc.create_element("p", Namespace::Html);
    let text = doc.create_text_node("Texto dentro do p");

    doc.append_child(root, container).unwrap();
    doc.append_child(container, child_p).unwrap();
    doc.append_child(child_p, text).unwrap();

    let mut registry = LiveRangeRegistry::new();
    let mut range = Range::new(root);
    range.set_start(text, 5);
    range.set_end(text, 10);
    let handle = LiveRangeHandle::new(range);
    registry.register(&handle);

    // Remove child_p (que contém text como descendente)
    doc.remove_child(container, child_p).unwrap();
    registry.notify_node_removal_with_doc(&doc, child_p, container, 0);

    let r_after = handle.get_range();
    println!(
        "Range após remoção do pai de text: start.node={:?} (esperado container={:?})",
        r_after.start.node, container
    );
    let updated_to_parent = r_after.start.node == container;
    println!("LiveRange atualizou nó descendente para o pai: {}", updated_to_parent);
    assert!(updated_to_parent, "LiveRange deve atualizar nó descendente para o pai container");
    assert_eq!(r_after.start.offset, 0);
    assert_eq!(r_after.end.node, container);
    assert_eq!(r_after.end.offset, 0);
}

