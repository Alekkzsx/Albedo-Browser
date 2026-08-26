use ace_dom::{parse_html, ComplexSelector, RuleBucketIndex};

#[test]
fn test_complex_multi_segment_rtl_matching() {
    let html = r##"
        <div class="main">
            <section class="content">
                <article class="post">
                    <h1 class="title">Header</h1>
                    <p class="summary">Summary <span class="highlight">Text</span></p>
                </article>
            </section>
        </div>
    "##;

    let doc = parse_html(html);

    // Multi-segment chain: 3 descendentes/filhos
    let sel = ComplexSelector::parse("div.main section.content article.post p.summary span.highlight")
        .expect("selector parsed");

    let span_nodes = doc.query_selector_all("span.highlight");
    assert_eq!(span_nodes.len(), 1);
    assert!(sel.matches(&doc, span_nodes[0]));

    // Seletor com combinador de filho e irmão adjacente
    let sel2 = ComplexSelector::parse("article.post > h1.title + p.summary").expect("selector parsed");
    let p_nodes = doc.query_selector_all("p.summary");
    assert_eq!(p_nodes.len(), 1);
    assert!(sel2.matches(&doc, p_nodes[0]));
}

#[test]
fn test_css4_pseudo_classes_is_where_has() {
    let html = r##"
        <div id="card1" class="card active">
            <h2 class="heading">Card 1</h2>
            <button class="btn primary">Click me</button>
        </div>
        <div id="card2" class="card">
            <h2 class="heading">Card 2</h2>
            <a href="#" class="link">Learn more</a>
        </div>
    "##;

    let doc = parse_html(html);

    // 1. Testa :is(button, a)
    let is_sel = ComplexSelector::parse(".card :is(button, a)").expect("is selector");
    let matches_is: Vec<_> = doc
        .descendants(doc.root())
        .filter_map(|(id, _)| if is_sel.matches(&doc, id) { Some(id) } else { None })
        .collect();
    assert_eq!(matches_is.len(), 2);

    // 2. Testa :has(button.primary) -> apenas o #card1 possui
    let has_sel = ComplexSelector::parse("div.card:has(button.primary)").expect("has selector");
    let card1_id = doc.get_element_by_id("card1").unwrap();
    let card2_id = doc.get_element_by_id("card2").unwrap();

    assert!(has_sel.matches(&doc, card1_id));
    assert!(!has_sel.matches(&doc, card2_id));

    // 3. Testa :where(h2, button)
    let where_sel = ComplexSelector::parse(":where(h2, button)").expect("where selector");
    let where_matches: Vec<_> = doc
        .descendants(doc.root())
        .filter_map(|(id, _)| if where_sel.matches(&doc, id) { Some(id) } else { None })
        .collect();
    assert_eq!(where_matches.len(), 3); // 2 h2s + 1 button
}

#[test]
fn test_rule_bucket_index_filtering() {
    let html = r##"
        <div id="header" class="nav-bar dark">
            <a class="nav-link" href="/">Home</a>
            <span class="badge">New</span>
        </div>
    "##;

    let doc = parse_html(html);
    let mut index = RuleBucketIndex::new();

    let sel_id = ComplexSelector::parse("#header").unwrap();
    let sel_class = ComplexSelector::parse(".nav-link").unwrap();
    let sel_tag = ComplexSelector::parse("span").unwrap();
    let sel_univ = ComplexSelector::parse("*").unwrap();

    index.add_rule(sel_id, "id_rule");
    index.add_rule(sel_class, "class_rule");
    index.add_rule(sel_tag, "tag_rule");
    index.add_rule(sel_univ, "univ_rule");

    let header_id = doc.get_element_by_id("header").unwrap();
    let mut header_candidates = Vec::new();
    index.match_candidates(&doc, header_id, &mut header_candidates);
    assert!(header_candidates.contains(&"id_rule"));
    assert!(header_candidates.contains(&"univ_rule"));
    assert!(!header_candidates.contains(&"class_rule"));

    let link_id = doc.query_selector("a.nav-link").unwrap();
    let mut link_candidates = Vec::new();
    index.match_candidates(&doc, link_id, &mut link_candidates);
    assert!(link_candidates.contains(&"class_rule"));
    assert!(link_candidates.contains(&"univ_rule"));
    assert!(!link_candidates.contains(&"id_rule"));
}
