//! Preload Scanner Example
//! 
//! Demonstrates how to use the preload scanner to discover resources
//! (stylesheets, scripts, images) before the full DOM is constructed.
//! This enables early resource loading for better page performance.

use ace::html::{PreloadScanner, PreloadResourceType, parse_html_integrated_with_options, ParserOptions};

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         ACE-HTML Preload Scanner Examples               ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    example_basic_preload();
    example_resource_types();
    example_responsive_images();
    example_integrated_parsing();
}

fn example_basic_preload() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 1: Basic Preload Scanning");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <link rel="stylesheet" href="styles.css">
    <link rel="stylesheet" href="theme.css">
    <script src="app.js"></script>
    <script src="vendor.js" defer></script>
</head>
<body>
    <img src="logo.png" alt="Logo">
    <img src="hero.jpg" alt="Hero">
</body>
</html>"#;

    let mut scanner = PreloadScanner::new();
    let requests = scanner.scan(html);
    
    println!("✅ Found {} resources to preload:\n", requests.len());
    
    for (i, req) in requests.iter().enumerate() {
        println!("   {}. [{:?}] {}", i + 1, req.resource_type, req.url);
        if !req.attributes.is_empty() {
            println!("      Attributes: {:?}", req.attributes);
        }
    }
    println!();
}

fn example_resource_types() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 2: Different Resource Types");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <!-- Stylesheets -->
    <link rel="stylesheet" href="main.css">
    <link rel="preload" href="critical.css" as="style">
    
    <!-- Scripts -->
    <script src="app.js"></script>
    <script src="analytics.js" async></script>
    <script type="module" src="module.js"></script>
    
    <!-- Fonts -->
    <link rel="preload" href="font.woff2" as="font" type="font/woff2" crossorigin>
    
    <!-- Prefetch -->
    <link rel="prefetch" href="next-page.html">
</head>
<body>
    <!-- Images -->
    <img src="banner.jpg" alt="Banner">
    <img src="icon.svg" alt="Icon">
    
    <!-- Video -->
    <video poster="poster.jpg">
        <source src="video.mp4" type="video/mp4">
    </video>
    
    <!-- Audio -->
    <audio src="sound.mp3"></audio>
</body>
</html>"#;

    let mut scanner = PreloadScanner::new();
    let requests = scanner.scan(html);
    
    println!("✅ Discovered {} resources:\n", requests.len());
    
    // Group by resource type
    let mut by_type: std::collections::HashMap<PreloadResourceType, Vec<&str>> = 
        std::collections::HashMap::new();
    
    for req in &requests {
        by_type.entry(req.resource_type)
            .or_insert_with(Vec::new)
            .push(&req.url);
    }
    
    for (resource_type, urls) in by_type.iter() {
        println!("   {:?} ({}):", resource_type, urls.len());
        for url in urls {
            println!("      • {}", url);
        }
        println!();
    }
}

fn example_responsive_images() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 3: Responsive Images (srcset)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<body>
    <!-- Responsive image with srcset -->
    <img src="default.jpg"
         srcset="small.jpg 480w, medium.jpg 800w, large.jpg 1200w"
         sizes="(max-width: 600px) 480px, (max-width: 1000px) 800px, 1200px"
         alt="Responsive">
    
    <!-- Picture element -->
    <picture>
        <source srcset="image.webp" type="image/webp">
        <source srcset="image.avif" type="image/avif">
        <img src="image.jpg" alt="Modern formats">
    </picture>
    
    <!-- High DPI images -->
    <img src="icon.png" srcset="icon@2x.png 2x, icon@3x.png 3x" alt="Icon">
</body>
</html>"#;

    let mut scanner = PreloadScanner::new();
    let requests = scanner.scan(html);
    
    println!("✅ Responsive image resources:\n");
    
    for req in &requests {
        if req.resource_type == PreloadResourceType::Image {
            println!("   • {}", req.url);
            if let Some(srcset) = req.attributes.get("srcset") {
                println!("     srcset: {}", srcset);
            }
        }
    }
    println!();
}

fn example_integrated_parsing() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 4: Integrated Parsing with Preload");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Integrated Example</title>
    <link rel="stylesheet" href="critical.css">
    <link rel="preload" href="font.woff2" as="font" crossorigin>
    <script src="app.js" defer></script>
</head>
<body>
    <header>
        <img src="logo.svg" alt="Logo">
    </header>
    <main>
        <img src="hero.jpg" alt="Hero" loading="lazy">
        <img src="content.png" alt="Content">
    </main>
</body>
</html>"#;

    let mut options = ParserOptions::default();
    options.collect_preloads = true;
    
    let result = parse_html_integrated_with_options(html, &options);
    
    println!("✅ Integrated parsing complete:");
    println!("   Nodes: {}", count_nodes(&result.document));
    println!("   Preload requests: {}\n", result.preload_requests.len());
    
    println!("   Critical resources (load immediately):");
    for req in &result.preload_requests {
        match req.resource_type {
            PreloadResourceType::Stylesheet | PreloadResourceType::Script => {
                let loading = req.attributes.get("defer")
                    .or(req.attributes.get("async"))
                    .map(|_| " (deferred)")
                    .unwrap_or("");
                println!("      • {} {}", req.url, loading);
            }
            _ => {}
        }
    }
    
    println!("\n   Images (can lazy load):");
    for req in &result.preload_requests {
        if req.resource_type == PreloadResourceType::Image {
            let loading = req.attributes.get("loading")
                .map(|v| format!(" (loading={})", v))
                .unwrap_or_default();
            println!("      • {}{}", req.url, loading);
        }
    }
    println!();
}

fn count_nodes(doc: &ace::html::HtmlDocument) -> usize {
    fn count_children(children: &[ace::html::HtmlNode]) -> usize {
        children.iter().map(|node| {
            match node {
                ace::html::HtmlNode::Element(el) => 1 + count_children(&el.children),
                _ => 1,
            }
        }).sum()
    }
    count_children(&doc.children)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_preload_stylesheet() {
        let html = r#"<link rel="stylesheet" href="style.css">"#;
        let mut scanner = PreloadScanner::new();
        let requests = scanner.scan(html);
        
        assert!(requests.iter().any(|r| 
            r.resource_type == PreloadResourceType::Stylesheet && r.url == "style.css"
        ));
    }
    
    #[test]
    fn test_preload_script() {
        let html = r#"<script src="app.js"></script>"#;
        let mut scanner = PreloadScanner::new();
        let requests = scanner.scan(html);
        
        assert!(requests.iter().any(|r| 
            r.resource_type == PreloadResourceType::Script && r.url == "app.js"
        ));
    }
    
    #[test]
    fn test_preload_image() {
        let html = r#"<img src="image.png" alt="Test">"#;
        let mut scanner = PreloadScanner::new();
        let requests = scanner.scan(html);
        
        assert!(requests.iter().any(|r| 
            r.resource_type == PreloadResourceType::Image && r.url == "image.png"
        ));
    }
    
    #[test]
    fn test_preload_multiple() {
        let html = r#"
            <link rel="stylesheet" href="a.css">
            <script src="b.js"></script>
            <img src="c.png">
        "#;
        let mut scanner = PreloadScanner::new();
        let requests = scanner.scan(html);
        
        assert!(requests.len() >= 3);
    }
}
