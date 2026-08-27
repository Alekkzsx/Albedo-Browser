use ace_dom::cssom::{
    ColorScheme, CSSRule, CSSStyleSheet, MediaContext, MediaType, Orientation, StyleResolver,
};
use ace_dom::parse_html;

#[test]
fn test_media_query_nested_stylesheet_parsing() {
    let css = r#"
        p { color: black; }
        @media screen and (min-width: 600px) {
            .box { width: 50%; margin: 10px; }
            .btn { color: red; }
        }
        @media (prefers-color-scheme: dark) {
            body { background-color: #121212; color: #ffffff; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);
    assert_eq!(sheet.rules.len(), 3);

    match &sheet.rules[0] {
        CSSRule::Style(ref r) => assert_eq!(r.selector_text.as_str(), "p"),
        _ => panic!("Expected style rule"),
    }

    match &sheet.rules[1] {
        CSSRule::Media { condition, rules } => {
            assert_eq!(condition.as_str(), "screen and (min-width: 600px)");
            assert_eq!(rules.len(), 2);
        }
        _ => panic!("Expected media rule"),
    }

    match &sheet.rules[2] {
        CSSRule::Media { condition, rules } => {
            assert_eq!(condition.as_str(), "(prefers-color-scheme: dark)");
            assert_eq!(rules.len(), 1);
        }
        _ => panic!("Expected media rule"),
    }
}

#[test]
fn test_media_query_viewport_min_max_width() {
    let html = r#"<div id="target" class="responsive-box">Content</div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let css = r#"
        .responsive-box { color: grey; font-size: 14px; }
        @media (min-width: 600px) and (max-width: 1000px) {
            .responsive-box { color: green; font-size: 20px; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // 1. Viewport 750px (dentro da faixa 600..1000)
    let ctx_match = MediaContext::new().with_viewport(750.0, 600.0);
    let computed_match =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, std::slice::from_ref(&sheet), &ctx_match);
    assert_eq!(computed_match.get_property_value("color"), Some("green"));
    assert_eq!(computed_match.get_property_value("font-size"), Some("20px"));

    // 2. Viewport 500px (abaixo de 600px)
    let ctx_small = MediaContext::new().with_viewport(500.0, 600.0);
    let computed_small =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, std::slice::from_ref(&sheet), &ctx_small);
    assert_eq!(computed_small.get_property_value("color"), Some("grey"));
    assert_eq!(computed_small.get_property_value("font-size"), Some("14px"));

    // 3. Viewport 1200px (acima de 1000px)
    let ctx_large = MediaContext::new().with_viewport(1200.0, 800.0);
    let computed_large =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, &[sheet], &ctx_large);
    assert_eq!(computed_large.get_property_value("color"), Some("grey"));
    assert_eq!(computed_large.get_property_value("font-size"), Some("14px"));
}

#[test]
fn test_media_query_prefers_color_scheme_switching() {
    let html = r#"<body id="app"><div id="target" class="card">Text</div></body>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let css = r#"
        .card { background-color: #ffffff; color: #000000; }
        @media (prefers-color-scheme: dark) {
            .card { background-color: #121212; color: #ffffff; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // Modo Claro
    let ctx_light = MediaContext::new().with_color_scheme(ColorScheme::Light);
    let comp_light =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, std::slice::from_ref(&sheet), &ctx_light);
    assert_eq!(comp_light.get_property_value("background-color"), Some("#ffffff"));
    assert_eq!(comp_light.get_property_value("color"), Some("#000000"));

    // Modo Escuro
    let ctx_dark = MediaContext::new().with_color_scheme(ColorScheme::Dark);
    let comp_dark =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, &[sheet], &ctx_dark);
    assert_eq!(comp_dark.get_property_value("background-color"), Some("#121212"));
    assert_eq!(comp_dark.get_property_value("color"), Some("#ffffff"));
}

#[test]
fn test_media_query_logical_and_or_not() {
    let html = r#"<div id="target" class="box"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let css = r#"
        /* Disjunção (OR): < 400px ou > 1200px */
        @media (max-width: 400px), (min-width: 1200px) {
            .box { margin: 50px; }
        }
        /* Negação (NOT) */
        @media not screen and (min-width: 600px) {
            .box { opacity: 0.2; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // Viewport 800px no Screen -> (max-width: 400) é falso, (min-width: 1200) é falso -> margin não aplica
    // not screen and (min-width: 600) -> screen and min-width 600 é true -> not inverte para false -> opacity não aplica
    let ctx_mid = MediaContext::new()
        .with_media_type(MediaType::Screen)
        .with_viewport(800.0, 600.0);
    let comp_mid =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, std::slice::from_ref(&sheet), &ctx_mid);
    assert_eq!(comp_mid.get_property_value("margin"), None);
    assert_eq!(comp_mid.get_property_value("opacity"), None);

    // Viewport 1400px no Screen -> (min-width: 1200px) é true -> margin aplica
    let ctx_wide = MediaContext::new()
        .with_media_type(MediaType::Screen)
        .with_viewport(1400.0, 900.0);
    let comp_wide =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, std::slice::from_ref(&sheet), &ctx_wide);
    assert_eq!(comp_wide.get_property_value("margin"), Some("50px"));

    // Media Type Print com Viewport 800px -> not screen and min-width 600 -> screen é false -> screen and min-width 600 é false -> not inverte para true -> opacity aplica!
    let ctx_print = MediaContext::new()
        .with_media_type(MediaType::Print)
        .with_viewport(800.0, 600.0);
    let comp_print =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, &[sheet], &ctx_print);
    assert_eq!(comp_print.get_property_value("opacity"), Some("0.2"));
}

#[test]
fn test_media_query_orientation() {
    let html = r#"<div id="target" class="flex-box"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    let css = r#"
        @media (orientation: portrait) {
            .flex-box { flex-direction: column; }
        }
        @media (orientation: landscape) {
            .flex-box { flex-direction: row; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // Portrait: width 600, height 900 (height >= width)
    let ctx_portrait = MediaContext::new().with_viewport(600.0, 900.0);
    assert_eq!(ctx_portrait.orientation, Orientation::Portrait);
    let comp_p = StyleResolver::resolve_element_style_with_context(
        &doc,
        target_id,
        std::slice::from_ref(&sheet),
        &ctx_portrait,
    );
    assert_eq!(comp_p.get_property_value("flex-direction"), Some("column"));

    // Landscape: width 1280, height 720 (width >= height)
    let ctx_landscape = MediaContext::new().with_viewport(1280.0, 720.0);
    assert_eq!(ctx_landscape.orientation, Orientation::Landscape);
    let comp_l =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, &[sheet], &ctx_landscape);
    assert_eq!(comp_l.get_property_value("flex-direction"), Some("row"));
}

#[test]
fn test_media_query_length_units_em() {
    let html = r#"<div id="target" class="box"></div>"#;
    let doc = parse_html(html);
    let target_id = doc.get_element_by_id("target").expect("target element");

    // 50em = 50 * 16px = 800px
    let css = r#"
        @media (min-width: 50em) {
            .box { padding: 30px; }
        }
    "#;
    let sheet = CSSStyleSheet::parse(css);

    // Viewport 850px >= 800px -> aplica padding: 30px
    let ctx_high = MediaContext::new().with_viewport(850.0, 600.0);
    let comp_high =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, std::slice::from_ref(&sheet), &ctx_high);
    assert_eq!(comp_high.get_property_value("padding"), Some("30px"));

    // Viewport 750px < 800px -> não aplica
    let ctx_low = MediaContext::new().with_viewport(750.0, 600.0);
    let comp_low =
        StyleResolver::resolve_element_style_with_context(&doc, target_id, &[sheet], &ctx_low);
    assert_eq!(comp_low.get_property_value("padding"), None);
}
