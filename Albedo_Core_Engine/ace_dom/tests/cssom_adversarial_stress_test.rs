use ace_dom::cssom::{
    ColorScheme, CSSStyleSheet, MediaContext, MediaType, StyleResolver,
};
use ace_dom::parse_html;
use ace_dom::query::selector::CompoundSelector;

#[test]
fn test_remediation_compound_selector_parse_atomic_not_swallowed() {
    use ace_core::intern::Atom;
    use ace_dom::query::selector::{SimpleSelector, Specificity};

    let comp = CompoundSelector::parse(".highlight.header").expect("parse compound");
    assert_eq!(comp.simple_selectors.len(), 2);
    assert_eq!(comp.simple_selectors[0], SimpleSelector::Class(Atom::new("highlight")));
    assert_eq!(comp.simple_selectors[1], SimpleSelector::Class(Atom::new("header")));
    assert_eq!(comp.specificity(), Specificity::new(0, 2, 0));

    let comp2 = CompoundSelector::parse("#main.active").expect("parse compound");
    assert_eq!(comp2.simple_selectors.len(), 2);
    assert_eq!(comp2.simple_selectors[0], SimpleSelector::Id(Atom::new("main")));
    assert_eq!(comp2.simple_selectors[1], SimpleSelector::Class(Atom::new("active")));
    assert_eq!(comp2.specificity(), Specificity::new(1, 1, 0));
}

#[test]
fn test_remediation_media_case_sensitivity() {
    let css = "@MEDIA screen { .box { color: red; } }";
    let sheet = CSSStyleSheet::parse(css);
    assert_eq!(sheet.rules.len(), 1);
    match &sheet.rules[0] {
        ace_dom::cssom::CSSRule::Media { condition, rules } => {
            assert_eq!(condition.as_str(), "screen");
            assert_eq!(rules.len(), 1);
        }
        _ => panic!("Expected CSSRule::Media"),
    }
}

#[test]
fn test_adversarial_mq_logical_expressions() {
    // 1. Conjunção com 3 termos 'and' + orientação
    let html = r#"<div id="target" class="responsive">Test</div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target");

    let css = r#"
        .responsive { color: grey; font-size: 10px; }
        @media screen and (min-width: 500px) and (max-width: 1000px) and (orientation: landscape) {
            .responsive { color: green; font-size: 22px; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // Caso 1.1: Match exato dentro da faixa (800x600, landscape)
    let ctx_match = MediaContext::new()
        .with_media_type(MediaType::Screen)
        .with_viewport(800.0, 600.0);
    let c1 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx_match,
    );
    assert_eq!(c1.get_property_value("color"), Some("green"));
    assert_eq!(c1.get_property_value("font-size"), Some("22px"));

    // Caso 1.2: Limite inferior exato (500.0px)
    let ctx_boundary_min = MediaContext::new()
        .with_media_type(MediaType::Screen)
        .with_viewport(500.0, 400.0);
    let c2 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx_boundary_min,
    );
    assert_eq!(c2.get_property_value("color"), Some("green"));

    // Caso 1.3: Logo abaixo do limite inferior (499.0px)
    let ctx_below_min = MediaContext::new()
        .with_media_type(MediaType::Screen)
        .with_viewport(499.0, 400.0);
    let c3 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx_below_min,
    );
    assert_eq!(c3.get_property_value("color"), Some("grey"));

    // Caso 1.4: Limite superior exato (1000.0px)
    let ctx_boundary_max = MediaContext::new()
        .with_media_type(MediaType::Screen)
        .with_viewport(1000.0, 800.0);
    let c4 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx_boundary_max,
    );
    assert_eq!(c4.get_property_value("color"), Some("green"));

    // Caso 1.5: Acima do limite superior (1001.0px)
    let ctx_above_max = MediaContext::new()
        .with_media_type(MediaType::Screen)
        .with_viewport(1001.0, 800.0);
    let c5 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx_above_max,
    );
    assert_eq!(c5.get_property_value("color"), Some("grey"));

    // Caso 1.6: Dimensões na faixa mas orientação portrait (600x800)
    let ctx_portrait = MediaContext::new()
        .with_media_type(MediaType::Screen)
        .with_viewport(600.0, 800.0);
    let c6 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx_portrait,
    );
    assert_eq!(c6.get_property_value("color"), Some("grey"));

    // Caso 1.7: Media type print ao invés de screen
    let ctx_print = MediaContext::new()
        .with_media_type(MediaType::Print)
        .with_viewport(800.0, 600.0);
    let c7 =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, &[sheet], &ctx_print);
    assert_eq!(c7.get_property_value("color"), Some("grey"));
}

#[test]
fn test_adversarial_mq_not_prefers_color_scheme() {
    let html = r#"<div id="target" class="theme-box"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target");

    let css = r#"
        .theme-box { background-color: #000; }
        @media not (prefers-color-scheme: dark) {
            .theme-box { background-color: #fff; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // Quando color scheme é Dark: (prefers-color-scheme: dark) é true -> not inverte para false -> #000
    let ctx_dark = MediaContext::new().with_color_scheme(ColorScheme::Dark);
    let c_dark = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx_dark,
    );
    assert_eq!(c_dark.get_property_value("background-color"), Some("#000"));

    // Quando color scheme é Light: (prefers-color-scheme: dark) é false -> not inverte para true -> #fff
    let ctx_light = MediaContext::new().with_color_scheme(ColorScheme::Light);
    let c_light = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx_light,
    );
    assert_eq!(c_light.get_property_value("background-color"), Some("#fff"));

    // Quando color scheme é NoPreference: (prefers-color-scheme: dark) é false -> not inverte para true -> #fff
    let ctx_none = MediaContext::new().with_color_scheme(ColorScheme::NoPreference);
    let c_none =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, &[sheet], &ctx_none);
    assert_eq!(c_none.get_property_value("background-color"), Some("#fff"));
}

#[test]
fn test_adversarial_mq_comma_disjunction_and_units() {
    let html = r#"<div id="target" class="box"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target");

    // Teste de lista separada por vírgulas e várias unidades (cm, in, pt, rem)
    let css = r#"
        .box { padding: 5px; }
        /* 2.54cm = 96px, 10in = 960px, 720pt = 960px */
        @media print and (min-width: 2.54cm), screen and (max-width: 400px), screen and (min-width: 10in) {
            .box { padding: 40px; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // 1. Print com largura 100px (100px >= 2.54cm [96px]) -> Aplica!
    let ctx1 = MediaContext::new()
        .with_media_type(MediaType::Print)
        .with_viewport(100.0, 100.0);
    let c1 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx1,
    );
    assert_eq!(c1.get_property_value("padding"), Some("40px"));

    // 2. Print com largura 50px (50px < 96px) -> Não aplica
    let ctx2 = MediaContext::new()
        .with_media_type(MediaType::Print)
        .with_viewport(50.0, 100.0);
    let c2 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx2,
    );
    assert_eq!(c2.get_property_value("padding"), Some("5px"));

    // 3. Screen com 350px (350px <= 400px) -> Aplica!
    let ctx3 = MediaContext::new()
        .with_media_type(MediaType::Screen)
        .with_viewport(350.0, 600.0);
    let c3 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx3,
    );
    assert_eq!(c3.get_property_value("padding"), Some("40px"));

    // 4. Screen com 700px (não é <= 400px nem >= 960px) -> Não aplica
    let ctx4 = MediaContext::new()
        .with_media_type(MediaType::Screen)
        .with_viewport(700.0, 600.0);
    let c4 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx4,
    );
    assert_eq!(c4.get_property_value("padding"), Some("5px"));

    // 5. Screen com 1000px (1000px >= 10in [960px]) -> Aplica!
    let ctx5 = MediaContext::new()
        .with_media_type(MediaType::Screen)
        .with_viewport(1000.0, 800.0);
    let c5 =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, &[sheet], &ctx5);
    assert_eq!(c5.get_property_value("padding"), Some("40px"));
}

#[test]
fn test_adversarial_media_query_cascading_and_specificity() {
    let html = r#"<h1 id="title" class="highlight header">Hello World</h1>"#;
    let doc = parse_html(html);
    let title_id = doc.get_element_by_id("title").expect("title");

    // Usando tag prefixada h1.highlight.header para isolar o teste de especificidade da cascata
    let css = r#"
        /* Regra 1 fora de media: especificidade (0,1,0) */
        .header { color: red; font-size: 16px; margin: 10px; }

        /* Regra 2 dentro de media: especificidade (1,0,0) */
        @media (min-width: 600px) {
            #title { color: blue; font-size: 28px; }
        }

        /* Regra 3 fora de media: especificidade (0,2,1) */
        h1.highlight.header { margin: 30px; }

        /* Regra 4 dentro de media: especificidade (0,1,0) - menor que (0,2,1) */
        @media (min-width: 600px) {
            .header { margin: 50px; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // Quando media casa (min-width: 600px):
    // 1. color: #title (1,0,0) vence .header (0,1,0) -> 'blue'
    // 2. font-size: #title (1,0,0) vence .header (0,1,0) -> '28px'
    // 3. margin: h1.highlight.header (0,2,1) fora do media vence .header (0,1,0) dentro do media -> '30px'
    let ctx_match = MediaContext::new().with_viewport(800.0, 600.0);
    let c_match = StyleResolver::resolve_element_style_with_context(
        &doc,
        title_id,
        std::slice::from_ref(&sheet),
        &ctx_match,
    );
    assert_eq!(c_match.get_property_value("color"), Some("blue"));
    assert_eq!(c_match.get_property_value("font-size"), Some("28px"));
    assert_eq!(c_match.get_property_value("margin"), Some("30px"));

    // Quando media NÃO casa (min-width: 500px):
    // 1. color: 'red'
    // 2. font-size: '16px'
    // 3. margin: '30px'
    let ctx_no_match = MediaContext::new().with_viewport(500.0, 600.0);
    let c_nomatch = StyleResolver::resolve_element_style_with_context(
        &doc,
        title_id,
        &[sheet],
        &ctx_no_match,
    );
    assert_eq!(c_nomatch.get_property_value("color"), Some("red"));
    assert_eq!(c_nomatch.get_property_value("font-size"), Some("16px"));
    assert_eq!(c_nomatch.get_property_value("margin"), Some("30px"));
}

#[test]
fn test_adversarial_multiple_media_blocks_overlapping_cascade() {
    let html = r#"<div id="target" class="stepped-box">Box</div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target");

    let css = r#"
        .stepped-box { width: 100px; color: black; border-width: 1px; }

        @media (min-width: 300px) {
            .stepped-box { width: 300px; color: yellow; }
        }
        @media (min-width: 600px) {
            .stepped-box { width: 600px; border-width: 2px; }
        }
        @media (min-width: 900px) {
            .stepped-box { width: 900px; color: cyan; }
        }
        @media (min-width: 1200px) {
            .stepped-box { width: 1200px; color: magenta; border-width: 4px; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // 1. Viewport 200px (nenhum media)
    let c200 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &MediaContext::new().with_viewport(200.0, 600.0),
    );
    assert_eq!(c200.get_property_value("width"), Some("100px"));
    assert_eq!(c200.get_property_value("color"), Some("black"));
    assert_eq!(c200.get_property_value("border-width"), Some("1px"));

    // 2. Viewport 450px (>= 300)
    let c450 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &MediaContext::new().with_viewport(450.0, 600.0),
    );
    assert_eq!(c450.get_property_value("width"), Some("300px"));
    assert_eq!(c450.get_property_value("color"), Some("yellow"));
    assert_eq!(c450.get_property_value("border-width"), Some("1px"));

    // 3. Viewport 750px (>= 300, >= 600)
    let c750 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &MediaContext::new().with_viewport(750.0, 600.0),
    );
    assert_eq!(c750.get_property_value("width"), Some("600px"));
    assert_eq!(c750.get_property_value("color"), Some("yellow"));
    assert_eq!(c750.get_property_value("border-width"), Some("2px"));

    // 4. Viewport 1000px (>= 300, >= 600, >= 900)
    let c1000 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &MediaContext::new().with_viewport(1000.0, 600.0),
    );
    assert_eq!(c1000.get_property_value("width"), Some("900px"));
    assert_eq!(c1000.get_property_value("color"), Some("cyan"));
    assert_eq!(c1000.get_property_value("border-width"), Some("2px"));

    // 5. Viewport 1500px (todos casam)
    let c1500 = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        &[sheet],
        &MediaContext::new().with_viewport(1500.0, 600.0),
    );
    assert_eq!(c1500.get_property_value("width"), Some("1200px"));
    assert_eq!(c1500.get_property_value("color"), Some("magenta"));
    assert_eq!(c1500.get_property_value("border-width"), Some("4px"));
}

#[test]
fn test_adversarial_important_declarations_in_media_queries() {
    let html = r#"<div id="target" class="card" style="color: purple;">Text</div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target");

    let css = r#"
        #target { color: black; font-weight: bold; }
        @media (min-width: 600px) {
            .card { color: lime !important; font-weight: normal !important; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // Quando media casa (800px):
    // 1. color: .card !important na folha (lime) VENCE inline style sem important (purple)!
    // 2. font-weight: .card !important (normal) vence #target sem important (bold)!
    let ctx_match = MediaContext::new().with_viewport(800.0, 600.0);
    let c_match = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx_match,
    );
    assert_eq!(c_match.get_property_value("color"), Some("lime"));
    assert_eq!(c_match.get_property_value("font-weight"), Some("normal"));

    // Quando media NÃO casa (400px):
    // 1. color: inline style (purple) vence stylesheet #target (black)
    // 2. font-weight: #target (bold)
    let ctx_no = MediaContext::new().with_viewport(400.0, 600.0);
    let c_no = StyleResolver::resolve_element_style_with_context(&doc, target_id, &[sheet], &ctx_no);
    assert_eq!(c_no.get_property_value("color"), Some("purple"));
    assert_eq!(c_no.get_property_value("font-weight"), Some("bold"));
}

#[test]
fn test_adversarial_disabled_stylesheet_with_media_queries() {
    let html = r#"<div id="target" class="box"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target");

    let css = r#"
        .box { color: red; }
        @media screen {
            .box { color: blue; }
        }
    "#;
    let mut sheet = CSSStyleSheet::parse(css);
    sheet.disabled = true;

    let computed = StyleResolver::resolve_element_style(&doc, target_id, &[sheet]);
    // Folha desabilitada -> nenhuma regra aplica
    assert_eq!(computed.get_property_value("color"), None);
}

#[test]
fn test_adversarial_10_level_deep_dom_inheritance_and_keywords() {
    // Hierarquia DOM com 10 níveis estritos de profundidade:
    // Level 1: Root #n1 com propriedades herdáveis e não-herdáveis + variáveis CSS
    // Level 2: #n2 (herança automática implícita)
    // Level 3: #n3 (shadowing de variável, 'inherit' em não-herdável margin)
    // Level 4: #n4 (override explícito de color e display)
    // Level 5: #n5 (reset com 'initial' para color, font-size, visibility, margin)
    // Level 6: #n6 (herança automática a partir de level 5)
    // Level 7: #n7 (uso de 'unset' - inherit para color, initial para margin e display)
    // Level 8: #n8 (variável customizada nova + font-size de nível superior)
    // Level 9: #n9 (variável cíclica IACVT -> age como unset -> herda do level 8)
    // Level 10: #n10 leaf (palavra-chave 'inherit' em múltiplos campos)
    let html = r#"
        <div id="n1" style="
            color: #112233;
            font-size: 32px;
            line-height: 2.5;
            visibility: hidden;
            letter-spacing: 4px;
            cursor: pointer;
            direction: rtl;
            margin: 50px;
            padding: 40px;
            display: flex;
            opacity: 0.7;
            --global-theme: #abcdef;
            --level-num: 1;
        ">
            <div id="n2">
                <div id="n3" style="margin: inherit; padding: inherit; --level-num: 3;">
                    <div id="n4" style="color: #445566; display: block; opacity: 0.9;">
                        <div id="n5" style="color: initial; font-size: initial; visibility: initial; margin: initial;">
                            <div id="n6">
                                <div id="n7" style="color: unset; margin: unset; display: unset; font-size: 48px;">
                                    <div id="n8" style="--l8-color: #8899aa; color: var(--l8-color);">
                                        <div id="n9" style="--cycle: var(--cycle); color: var(--cycle); display: var(--cycle);">
                                            <span id="n10" style="color: inherit; visibility: inherit; font-size: inherit;">Leaf Content</span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    "#;
    let doc = parse_html(html);

    let n1 = doc.get_element_by_id("n1").expect("n1");
    let n2 = doc.get_element_by_id("n2").expect("n2");
    let n3 = doc.get_element_by_id("n3").expect("n3");
    let n4 = doc.get_element_by_id("n4").expect("n4");
    let n5 = doc.get_element_by_id("n5").expect("n5");
    let n6 = doc.get_element_by_id("n6").expect("n6");
    let n7 = doc.get_element_by_id("n7").expect("n7");
    let n8 = doc.get_element_by_id("n8").expect("n8");
    let n9 = doc.get_element_by_id("n9").expect("n9");
    let n10 = doc.get_element_by_id("n10").expect("n10");

    // ===== Level 1 (Root) =====
    let c1 = StyleResolver::resolve_element_style(&doc, n1, &[]);
    assert_eq!(c1.get_property_value("color"), Some("#112233"));
    assert_eq!(c1.get_property_value("font-size"), Some("32px"));
    assert_eq!(c1.get_property_value("line-height"), Some("2.5"));
    assert_eq!(c1.get_property_value("visibility"), Some("hidden"));
    assert_eq!(c1.get_property_value("letter-spacing"), Some("4px"));
    assert_eq!(c1.get_property_value("cursor"), Some("pointer"));
    assert_eq!(c1.get_property_value("direction"), Some("rtl"));
    assert_eq!(c1.get_property_value("margin"), Some("50px"));
    assert_eq!(c1.get_property_value("padding"), Some("40px"));
    assert_eq!(c1.get_property_value("display"), Some("flex"));
    assert_eq!(c1.get_property_value("opacity"), Some("0.7"));
    assert_eq!(c1.get_custom_property("--global-theme"), Some("#abcdef"));
    assert_eq!(c1.get_custom_property("--level-num"), Some("1"));

    // ===== Level 2 (Implicit Inheritance) =====
    let c2 = StyleResolver::resolve_element_style(&doc, n2, &[]);
    assert_eq!(c2.get_property_value("color"), Some("#112233")); // herdado
    assert_eq!(c2.get_property_value("font-size"), Some("32px")); // herdado
    assert_eq!(c2.get_property_value("line-height"), Some("2.5")); // herdado
    assert_eq!(c2.get_property_value("visibility"), Some("hidden")); // herdado
    assert_eq!(c2.get_property_value("letter-spacing"), Some("4px")); // herdado
    assert_eq!(c2.get_property_value("cursor"), Some("pointer")); // herdado
    assert_eq!(c2.get_property_value("direction"), Some("rtl")); // herdado
    assert_eq!(c2.get_property_value("margin"), None); // NÃO herdado
    assert_eq!(c2.get_property_value("padding"), None); // NÃO herdado
    assert_eq!(c2.get_property_value("display"), None); // NÃO herdado
    assert_eq!(c2.get_property_value("opacity"), None); // NÃO herdado
    assert_eq!(c2.get_custom_property("--global-theme"), Some("#abcdef")); // herdado
    assert_eq!(c2.get_custom_property("--level-num"), Some("1")); // herdado

    // ===== Level 3 (Explicit inherit on non-inheritable + var shadow) =====
    let c3 = StyleResolver::resolve_element_style(&doc, n3, &[]);
    // n2 não tinha margin/padding computado -> inherit do pai (n2) resulta no initial ('0px')
    assert_eq!(c3.get_property_value("margin"), Some("0px"));
    assert_eq!(c3.get_property_value("padding"), Some("0px"));
    assert_eq!(c3.get_property_value("color"), Some("#112233"));
    assert_eq!(c3.get_custom_property("--level-num"), Some("3")); // shadowed

    // ===== Level 4 (Overrides) =====
    let c4 = StyleResolver::resolve_element_style(&doc, n4, &[]);
    assert_eq!(c4.get_property_value("color"), Some("#445566")); // overridden
    assert_eq!(c4.get_property_value("display"), Some("block")); // overridden
    assert_eq!(c4.get_property_value("opacity"), Some("0.9")); // overridden
    assert_eq!(c4.get_property_value("font-size"), Some("32px")); // herdado de n1 via n2, n3
    assert_eq!(c4.get_custom_property("--level-num"), Some("3"));

    // ===== Level 5 (Initial Keyword Resets) =====
    let c5 = StyleResolver::resolve_element_style(&doc, n5, &[]);
    assert_eq!(c5.get_property_value("color"), Some("black")); // initial
    assert_eq!(c5.get_property_value("font-size"), Some("16px")); // initial
    assert_eq!(c5.get_property_value("visibility"), Some("visible")); // initial
    assert_eq!(c5.get_property_value("margin"), Some("0px")); // initial
    assert_eq!(c5.get_property_value("line-height"), Some("2.5")); // herdado de n1
    assert_eq!(c5.get_property_value("direction"), Some("rtl")); // herdado de n1

    // ===== Level 6 (Inherits reset values from Level 5) =====
    let c6 = StyleResolver::resolve_element_style(&doc, n6, &[]);
    assert_eq!(c6.get_property_value("color"), Some("black")); // herdado de n5
    assert_eq!(c6.get_property_value("font-size"), Some("16px")); // herdado de n5
    assert_eq!(c6.get_property_value("visibility"), Some("visible")); // herdado de n5
    assert_eq!(c6.get_property_value("margin"), None); // não herda

    // ===== Level 7 (Unset Keyword Dispatch) =====
    let c7 = StyleResolver::resolve_element_style(&doc, n7, &[]);
    // color é herdável -> unset age como inherit (pega 'black' de n6)
    assert_eq!(c7.get_property_value("color"), Some("black"));
    // margin e display são não-herdáveis -> unset age como initial ('0px', 'inline')
    assert_eq!(c7.get_property_value("margin"), Some("0px"));
    assert_eq!(c7.get_property_value("display"), Some("inline"));
    // font-size definido diretamente como 48px
    assert_eq!(c7.get_property_value("font-size"), Some("48px"));

    // ===== Level 8 (New var definition) =====
    let c8 = StyleResolver::resolve_element_style(&doc, n8, &[]);
    assert_eq!(c8.get_property_value("color"), Some("#8899aa"));
    assert_eq!(c8.get_property_value("font-size"), Some("48px")); // herdado de n7
    assert_eq!(c8.get_custom_property("--global-theme"), Some("#abcdef")); // preservado desde n1!

    // ===== Level 9 (Cyclic var IACVT handling) =====
    let c9 = StyleResolver::resolve_element_style(&doc, n9, &[]);
    // --cycle tem ciclo direto -> IACVT
    // color com IACVT age como unset -> herdável -> herda do pai n8: '#8899aa'
    assert_eq!(c9.get_property_value("color"), Some("#8899aa"));
    // display com IACVT age como unset -> não-herdável -> initial: 'inline'
    assert_eq!(c9.get_property_value("display"), Some("inline"));
    assert_eq!(c9.get_property_value("font-size"), Some("48px")); // herdado

    // ===== Level 10 (Leaf Element) =====
    let c10 = StyleResolver::resolve_element_style(&doc, n10, &[]);
    assert_eq!(c10.get_property_value("color"), Some("#8899aa")); // inherit do pai n9
    assert_eq!(c10.get_property_value("visibility"), Some("visible")); // inherit do pai n9 (que herdou desde n5)
    assert_eq!(c10.get_property_value("font-size"), Some("48px")); // inherit do pai n9 (que herdou de n7)
    assert_eq!(c10.get_property_value("line-height"), Some("2.5")); // herança profunda originada em n1!
    assert_eq!(c10.get_property_value("letter-spacing"), Some("4px")); // herança profunda originada em n1!
    assert_eq!(c10.get_property_value("cursor"), Some("pointer")); // herança profunda originada em n1!
    assert_eq!(c10.get_property_value("direction"), Some("rtl")); // herança profunda originada em n1!
    assert_eq!(c10.get_custom_property("--global-theme"), Some("#abcdef")); // variável customizada atravessou 10 níveis!
    assert_eq!(c10.get_custom_property("--level-num"), Some("3")); // variável customizada propagada de n3!
}
