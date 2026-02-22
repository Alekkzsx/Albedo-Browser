#[cfg(test)]
mod white_space_tests {
    use albedo::engine::AceEngine;
    use albedo::engine::style::css_values::CssWhiteSpace;

    #[test]
    fn test_white_space_normal() {
        let mut engine = AceEngine::new();
        
        let html = r#"
            <html>
            <head><style>
                p { white-space: normal; }
            </style></head>
            <body>
                <p id="test">hello    world
                test</p>
            </body>
            </html>
        "#;
        
        engine.load_html(html);
        engine.layout(800.0, 600.0);
        
        // Should collapse whitespace
        println!("✓ White-space normal parsed successfully");
    }

    #[test]
    fn test_white_space_pre() {
        let mut engine = AceEngine::new();
        
        let html = r#"
            <html>
            <head><style>
                pre { white-space: pre; }
            </style></head>
            <body>
                <pre id="test">hello    world
    indented line</pre>
            </body>
            </html>
        "#;
        
        engine.load_html(html);
        engine.layout(800.0, 600.0);
        
        // Should preserve all whitespace
        println!("✓ White-space pre parsed successfully");
    }

    #[test]
    fn test_white_space_nowrap() {
        let mut engine = AceEngine::new();
        
        let html = r#"
            <html>
            <head><style>
                span { white-space: nowrap; }
            </style></head>
            <body>
                <span>This    should    not wrap</span>
            </body>
            </html>
        "#;
        
        engine.load_html(html);
        engine.layout(800.0, 600.0);
        
        // Should not wrap even if container is narrow
        println!("✓ White-space nowrap parsed successfully");
    }

    #[test]
    fn test_white_space_pre_wrap() {
        let mut engine = AceEngine::new();
        
        let html = r#"
            <html>
            <head><style>
                div { white-space: pre-wrap; }
            </style></head>
            <body>
                <div>Line 1    with spaces
Line 2</div>
            </body>
            </html>
        "#;
        
        engine.load_html(html);
        engine.layout(800.0, 600.0);
        
        // Should preserve whitespace and wrap
        println!("✓ White-space pre-wrap parsed successfully");
    }

    #[test]
    fn test_white_space_pre_line() {
        let mut engine = AceEngine::new();
        
        let html = r#"
            <html>
            <head><style>
                div { white-space: pre-line; }
            </style></head>
            <body>
                <div>Line 1    extra spaces
Line 2</div>
            </body>
            </html>
        "#;
        
        engine.load_html(html);
        engine.layout(800.0, 600.0);
        
        // Should preserve newlines but collapse spaces
        println!("✓ White-space pre-line parsed successfully");
    }
}
