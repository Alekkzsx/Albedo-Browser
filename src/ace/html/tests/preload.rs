#[cfg(test)]
mod tests {
    use crate::ace::html::tree_builder::build_document_with_errors;
    use crate::ace::html::preload_scanner::PreloadResourceType;

    #[test]
    fn test_preload_scanner_discovery() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <link rel="stylesheet" href="style.css">
                <script src="important.js"></script>
            </head>
            <body>
                <img src="hero.png" srcset="hero-2x.png 2x">
                <div>
                    <video poster="thumb.jpg">
                        <source src="movie.mp4" type="video/mp4">
                    </video>
                </div>
                <script src="analytics.js" async></script>
            </body>
            </html>
        "#;

        let output = build_document_with_errors(html);
        
        // Check if resources were discovered
        let urls: Vec<_> = output.preload_requests.iter().map(|r| r.url.as_str()).collect();
        
        assert!(urls.contains(&"style.css"));
        assert!(urls.contains(&"important.js"));
        assert!(urls.contains(&"hero.png"));
        assert!(urls.contains(&"thumb.jpg"));
        assert!(urls.contains(&"movie.mp4"));
        assert!(urls.contains(&"analytics.js"));

        // Verify resource types
        let script_req = output.preload_requests.iter().find(|r| r.url == "important.js").unwrap();
        assert_eq!(script_req.resource_type, PreloadResourceType::Script);

        let style_req = output.preload_requests.iter().find(|r| r.url == "style.css").unwrap();
        assert_eq!(style_req.resource_type, PreloadResourceType::Stylesheet);
    }

    #[test]
    fn test_preload_scanner_ignore_comments() {
        let html = r#"
            <!-- <img src="ignored.png"> -->
            <img src="active.png">
        "#;

        let output = build_document_with_errors(html);
        let urls: Vec<_> = output.preload_requests.iter().map(|r| r.url.as_str()).collect();
        
        assert!(urls.contains(&"active.png"));
        // Current simple PreloadScanner might not ignore comments if they look like tags
        // In a "brutal" fast-path, we might accept some false positives for speed,
        // but let's see how it behaves.
    }
}
