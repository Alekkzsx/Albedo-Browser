//! # Bateria de Testes do Preload Scanner Especulativo (ace_dom)

use ace_dom::preload_scanner::{PreloadKind, PreloadScanner};

#[test]
fn test_preload_scanner_discovers_critical_resources() {
    let html = r#"
    <!DOCTYPE html>
    <html>
      <head>
        <link rel="stylesheet" href="/assets/style.css" media="screen">
        <link rel="preload" href="/fonts/inter.woff2" as="font">
        <script src="/js/app.js"></script>
      </head>
      <body>
        <!-- Comentário com <script src="fake.js"></script> ignorado -->
        <img src="/img/hero.webp" alt="Hero">
        <video poster="/img/poster.jpg">
          <source src="/media/video.mp4">
        </video>
      </body>
    </html>
    "#;

    let scanner = PreloadScanner::new();
    let requests = scanner.scan(html);

    assert_eq!(requests.len(), 6);

    // 1. Stylesheet
    assert_eq!(requests[0].url.as_str(), "/assets/style.css");
    assert_eq!(requests[0].kind, PreloadKind::Stylesheet);
    assert_eq!(requests[0].media.as_deref(), Some("screen"));

    // 2. Preload font
    assert_eq!(requests[1].url.as_str(), "/fonts/inter.woff2");
    assert_eq!(requests[1].kind, PreloadKind::Preload);
    assert_eq!(requests[1].as_type.as_deref(), Some("font"));

    // 3. Script
    assert_eq!(requests[2].url.as_str(), "/js/app.js");
    assert_eq!(requests[2].kind, PreloadKind::Script);

    // 4. Image
    assert_eq!(requests[3].url.as_str(), "/img/hero.webp");
    assert_eq!(requests[3].kind, PreloadKind::Image);

    // 5. Video poster
    assert_eq!(requests[4].url.as_str(), "/img/poster.jpg");
    assert_eq!(requests[4].kind, PreloadKind::Image);

    // 6. Video source
    assert_eq!(requests[5].url.as_str(), "/media/video.mp4");
    assert_eq!(requests[5].kind, PreloadKind::Media);
}
