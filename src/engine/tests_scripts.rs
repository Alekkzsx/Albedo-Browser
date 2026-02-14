
#[cfg(test)]
mod tests {
    use crate::engine::AceEngine;

    #[test]
    fn test_extract_scripts() {
        let html = r#"
            <html>
                <head>
                    <script>console.log('Script 1');</script>
                </head>
                <body>
                    <h1>Hello</h1>
                    <script>
                        var x = 10;
                        console.log(x);
                    </script>
                </body>
            </html>
        "#;
        
        let mut engine = AceEngine::new();
        let scripts = engine.load_html(html);
        
        assert_eq!(scripts.len(), 2);
        assert_eq!(scripts[0].content, "console.log('Script 1');");
        assert!(scripts[0].src.is_none());
        assert!(scripts[1].content.contains("var x = 10;"));
        assert!(scripts[1].src.is_none());
    }

    #[test]
    fn test_extract_external_script() {
        let html = r#"
            <html>
                <body>
                    <script src="https://example.com/script.js"></script>
                    <script>console.log('inline');</script>
                </body>
            </html>
        "#;
        
        let mut engine = AceEngine::new();
        let scripts = engine.load_html(html);
        
        assert_eq!(scripts.len(), 2);
        assert_eq!(scripts[0].src.as_deref(), Some("https://example.com/script.js"));
        assert!(scripts[0].content.is_empty());
        
        assert_eq!(scripts[1].src, None);
        assert_eq!(scripts[1].content, "console.log('inline');");
    }
}
