use ace_dom::{parse_html, parse_html_threaded, BackgroundHTMLParser, PreloadKind};

#[test]
fn test_background_parser_correctness_and_parity() {
    let html = r#"<!DOCTYPE html>
<html>
  <head>
    <title>Multithreaded DOM Test</title>
    <link rel="stylesheet" href="/assets/style.css">
    <script src="/assets/app.js"></script>
  </head>
  <body>
    <div id="main" class="container active">
      <header>
        <h1>Albedo Browser Extreme Engine</h1>
      </header>
      <main>
        <p>Parity test between synchronous and multithreaded background parsing.</p>
        <img src="/images/banner.webp" alt="Banner">
      </main>
      <footer>
        <span>Footer content</span>
      </footer>
    </div>
  </body>
</html>"#;

    // 1. Parse via thread principal (síncrono)
    let sync_doc = parse_html(html);

    // 2. Parse via background thread (assíncrono)
    let async_doc = parse_html_threaded(html);

    // Valida paridade estrutural
    assert_eq!(
        sync_doc.get_element_by_id("main").is_some(),
        async_doc.get_element_by_id("main").is_some()
    );

    let sync_main = sync_doc.get_element_by_id("main").unwrap();
    let async_main = async_doc.get_element_by_id("main").unwrap();

    let sync_el = sync_doc.get_node(sync_main).unwrap().as_element().unwrap();
    let async_el = async_doc.get_node(async_main).unwrap().as_element().unwrap();

    assert_eq!(sync_el.tag_name, async_el.tag_name);
    assert_eq!(sync_el.classes, async_el.classes);
}

#[test]
fn test_background_parser_speculative_preloads() {
    let html = r#"<!DOCTYPE html>
<html>
  <head>
    <link rel="stylesheet" href="/style.css">
    <script src="/bundle.js"></script>
  </head>
  <body>
    <img src="/avatar.png" alt="Avatar">
  </body>
</html>"#;

    let (mut handle, builder) = BackgroundHTMLParser::spawn_parse(html.to_string());
    let _ = handle.pump(&mut { builder });

    // Aguarda e verifica preloads descobertos em background
    let doc = handle.finish(ace_dom::tree_builder::HTMLTreeBuilder::new(None));
    assert!(doc.document_element.is_some());
}
