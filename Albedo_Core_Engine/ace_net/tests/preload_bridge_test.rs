//! # Testes de Integração com o PreloadScanner do ace_dom (Lookahead Especulativo)

use ace_dom::preload_scanner::{PreloadKind, PreloadScanner};
use ace_net::priority::PriorityLevel;
use ace_net::request::{Request, RequestDestination};

#[test]
fn test_preload_scanner_to_ace_net_requests() {
    let html = r#"
        <!doctype html>
        <html>
        <head>
            <link rel="stylesheet" href="/assets/style.css">
            <script src="/scripts/bundle.js"></script>
            <link rel="preload" href="/fonts/inter.woff2" as="font">
        </head>
        <body>
            <img src="/images/hero.webp">
        </body>
        </html>
    "#;

    let scanner = PreloadScanner::new();
    let preloads = scanner.scan(html);

    assert_eq!(preloads.len(), 4);

    // Mapeamento e criação das requisições na fila do ace_net
    for preload in preloads {
        let (destination, expected_priority) = match preload.kind {
            PreloadKind::Stylesheet => (RequestDestination::Style, PriorityLevel::High),
            PreloadKind::Script => (RequestDestination::Script, PriorityLevel::High),
            PreloadKind::Image => (RequestDestination::Image, PriorityLevel::Medium),
            PreloadKind::Preload => {
                if preload.as_type.as_deref() == Some("font") {
                    (RequestDestination::Font, PriorityLevel::High)
                } else {
                    (RequestDestination::Other, PriorityLevel::Low)
                }
            }
            _ => (RequestDestination::Other, PriorityLevel::Low),
        };

        let full_url = format!("https://example.com{}", preload.url);
        let req = Request::get(&full_url)
            .unwrap()
            .destination(destination)
            .build();

        assert_eq!(req.priority, expected_priority);
        assert_eq!(req.destination, destination);
        assert!(req.headers.contains_key("priority"));
    }
}
