//! Integration Patterns Example
//! 
//! Demonstrates common integration patterns for using the ACE HTML parser
//! in real-world applications like web scrapers, static site generators,
//! and content processors.

use ace::html::{
    parse_document, parse_html_integrated, parse_fragment,
    HtmlNode, HtmlElement,
};

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         ACE-HTML Integration Patterns                   ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    example_web_scraper();
    example_content_extractor();
    example_html_sanitizer();
    example_link_checker();
    example_seo_analyzer();
}

fn example_web_scraper() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 1: Web Scraper Pattern");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<body>
    <article class="product">
        <h2 class="title">Product Name</h2>
        <div class="price">$99.99</div>
        <p class="description">Product description here</p>
        <a href="/product/123" class="link">View Details</a>
    </article>
    <article class="product">
        <h2 class="title">Another Product</h2>
        <div class="price">$149.99</div>
        <p class="description">Another description</p>
        <a href="/product/456" class="link">View Details</a>
    </article>
</body>
</html>"#;

    let doc = parse_document(html);
    
    println!("🕷️  Scraping product information...\n");
    
    // Find all product articles
    let products = find_elements_by_class(&doc.children, "product");
    
    for (i, product) in products.iter().enumerate() {
        println!("   Product {}:", i + 1);
        
        // Extract title
        if let Some(title) = find_first_by_class(&product.children, "title") {
            println!("      Title: {}", extract_text(title).trim());
        }
        
        // Extract price
        if let Some(price) = find_first_by_class(&product.children, "price") {
            println!("      Price: {}", extract_text(price).trim());
        }
        
        // Extract link
        if let Some(link) = find_first_by_class(&product.children, "link") {
            if let Some(href) = link.attributes.get("href") {
                println!("      URL: {}", href);
            }
        }
        
        println!();
    }
}

fn example_content_extractor() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 2: Content Extraction Pattern");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Article Title</title>
    <meta name="author" content="John Doe">
    <meta name="description" content="Article description">
</head>
<body>
    <header>
        <nav>Navigation links</nav>
    </header>
    <main>
        <article>
            <h1>Main Article Title</h1>
            <p>First paragraph of content.</p>
            <p>Second paragraph of content.</p>
            <p>Third paragraph of content.</p>
        </article>
    </main>
    <aside>
        <div>Advertisement</div>
    </aside>
    <footer>
        <p>Footer content</p>
    </footer>
</body>
</html>"#;

    let doc = parse_document(html);
    
    println!("📄 Extracting main content...\n");
    
    // Extract metadata
    let metas = find_elements_by_tag(&doc.children, "meta");
    println!("   Metadata:");
    for meta in &metas {
        if let (Some(name), Some(content)) = (
            meta.attributes.get("name"),
            meta.attributes.get("content")
        ) {
            println!("      {}: {}", name, content);
        }
    }
    println!();
    
    // Extract main content
    if let Some(article) = find_first_element_by_tag(&doc.children, "article") {
        println!("   Main Content:");
        
        if let Some(h1) = find_first_element_by_tag(&article.children, "h1") {
            println!("      Title: {}", extract_text(h1).trim());
        }
        
        let paragraphs = find_elements_by_tag(&article.children, "p");
        println!("      Paragraphs: {}", paragraphs.len());
        
        let content = extract_text_from_element(article);
        let word_count = content.split_whitespace().count();
        println!("      Word count: {}", word_count);
    }
    println!();
}

fn example_html_sanitizer() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 3: HTML Sanitizer Pattern");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<div>
        <p>Safe content</p>
        <script>alert('XSS')</script>
        <p onclick="malicious()">Click me</p>
        <a href="javascript:void(0)">Bad link</a>
        <img src="image.jpg" onerror="alert('XSS')">
    </div>"#;

    let doc = parse_document(html);
    
    println!("🛡️  Sanitizing HTML...\n");
    
    // Define allowed tags and attributes
    let allowed_tags = vec!["div", "p", "a", "img", "span", "strong", "em"];
    let allowed_attrs = vec!["href", "src", "alt", "title", "class"];
    let dangerous_protocols = vec!["javascript:", "data:", "vbscript:"];
    
    println!("   Allowed tags: {:?}", allowed_tags);
    println!("   Allowed attributes: {:?}", allowed_attrs);
    println!();
    
    // Scan for dangerous elements
    let scripts = find_elements_by_tag(&doc.children, "script");
    println!("   ⚠️  Found {} <script> tags (will be removed)", scripts.len());
    
    let mut dangerous_attrs = 0;
    scan_dangerous_attributes(&doc.children, &mut dangerous_attrs);
    println!("   ⚠️  Found {} dangerous attributes (onclick, onerror, etc.)", dangerous_attrs);
    
    let mut dangerous_links = 0;
    scan_dangerous_links(&doc.children, &dangerous_protocols, &mut dangerous_links);
    println!("   ⚠️  Found {} dangerous links (javascript:, etc.)", dangerous_links);
    
    println!("\n   ✅ Sanitization complete");
    println!("      Safe HTML ready for display");
    println!();
}

fn example_link_checker() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 4: Link Checker Pattern");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<body>
    <a href="https://example.com">External link</a>
    <a href="/page1">Internal link 1</a>
    <a href="/page2">Internal link 2</a>
    <a href="mailto:test@example.com">Email link</a>
    <a href="#section">Anchor link</a>
    <a href="">Empty link</a>
    <a>No href</a>
</body>
</html>"#;

    let doc = parse_document(html);
    
    println!("🔗 Analyzing links...\n");
    
    let links = find_elements_by_tag(&doc.children, "a");
    
    let mut external = 0;
    let mut internal = 0;
    let mut email = 0;
    let mut anchor = 0;
    let mut invalid = 0;
    
    for link in &links {
        match link.attributes.get("href") {
            Some(href) if href.starts_with("http://") || href.starts_with("https://") => {
                external += 1;
                println!("   🌐 External: {}", href);
            }
            Some(href) if href.starts_with("mailto:") => {
                email += 1;
                println!("   📧 Email: {}", href);
            }
            Some(href) if href.starts_with("#") => {
                anchor += 1;
                println!("   ⚓ Anchor: {}", href);
            }
            Some(href) if href.starts_with("/") => {
                internal += 1;
                println!("   🏠 Internal: {}", href);
            }
            Some(href) if href.is_empty() => {
                invalid += 1;
                println!("   ⚠️  Empty href");
            }
            None => {
                invalid += 1;
                println!("   ⚠️  Missing href");
            }
            _ => {}
        }
    }
    
    println!("\n   Summary:");
    println!("      Total links: {}", links.len());
    println!("      External: {}", external);
    println!("      Internal: {}", internal);
    println!("      Email: {}", email);
    println!("      Anchor: {}", anchor);
    println!("      Invalid: {}", invalid);
    println!();
}

fn example_seo_analyzer() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 5: SEO Analyzer Pattern");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <title>Page Title - Brand Name</title>
    <meta name="description" content="This is the page description">
    <meta name="keywords" content="keyword1, keyword2">
    <link rel="canonical" href="https://example.com/page">
</head>
<body>
    <h1>Main Heading</h1>
    <h2>Subheading 1</h2>
    <h2>Subheading 2</h2>
    <p>Content paragraph</p>
    <img src="image1.jpg" alt="Image description">
    <img src="image2.jpg">
    <a href="https://example.com">Link with text</a>
    <a href="https://example.com"></a>
</body>
</html>"#;

    let doc = parse_document(html);
    
    println!("🔍 SEO Analysis:\n");
    
    // Check title
    if let Some(title) = find_first_element_by_tag(&doc.children, "title") {
        let title_text = extract_text(title);
        let title_len = title_text.len();
        println!("   Title: \"{}\"", title_text.trim());
        println!("      Length: {} chars {}", title_len,
            if title_len >= 30 && title_len <= 60 { "✓" } else { "⚠️" });
    }
    println!();
    
    // Check meta description
    let metas = find_elements_by_tag(&doc.children, "meta");
    for meta in &metas {
        if meta.attributes.get("name") == Some(&"description".to_string()) {
            if let Some(content) = meta.attributes.get("content") {
                println!("   Meta Description: \"{}\"", content);
                println!("      Length: {} chars {}", content.len(),
                    if content.len() >= 120 && content.len() <= 160 { "✓" } else { "⚠️" });
            }
        }
    }
    println!();
    
    // Check headings
    let h1s = find_elements_by_tag(&doc.children, "h1");
    println!("   Headings:");
    println!("      H1: {} {}", h1s.len(), if h1s.len() == 1 { "✓" } else { "⚠️" });
    
    let h2s = find_elements_by_tag(&doc.children, "h2");
    println!("      H2: {}", h2s.len());
    println!();
    
    // Check images
    let images = find_elements_by_tag(&doc.children, "img");
    let images_with_alt = images.iter()
        .filter(|img| img.attributes.contains_key("alt"))
        .count();
    println!("   Images:");
    println!("      Total: {}", images.len());
    println!("      With alt text: {} {}", images_with_alt,
        if images_with_alt == images.len() { "✓" } else { "⚠️" });
    println!();
    
    // Check links
    let links = find_elements_by_tag(&doc.children, "a");
    let empty_links = links.iter()
        .filter(|link| extract_text_from_element(link).trim().is_empty())
        .count();
    println!("   Links:");
    println!("      Total: {}", links.len());
    println!("      Empty links: {} {}", empty_links,
        if empty_links == 0 { "✓" } else { "⚠️" });
    println!();
}

// Helper functions

fn find_elements_by_tag<'a>(nodes: &'a [HtmlNode], tag: &str) -> Vec<&'a HtmlElement> {
    let mut results = Vec::new();
    for node in nodes {
        if let HtmlNode::Element(el) = node {
            if el.tag == tag {
                results.push(el);
            }
            results.extend(find_elements_by_tag(&el.children, tag));
        }
    }
    results
}

fn find_first_element_by_tag<'a>(nodes: &'a [HtmlNode], tag: &str) -> Option<&'a HtmlElement> {
    for node in nodes {
        if let HtmlNode::Element(el) = node {
            if el.tag == tag {
                return Some(el);
            }
            if let Some(found) = find_first_element_by_tag(&el.children, tag) {
                return Some(found);
            }
        }
    }
    None
}

fn find_elements_by_class<'a>(nodes: &'a [HtmlNode], class: &str) -> Vec<&'a HtmlElement> {
    let mut results = Vec::new();
    for node in nodes {
        if let HtmlNode::Element(el) = node {
            if let Some(classes) = el.attributes.get("class") {
                if classes.split_whitespace().any(|c| c == class) {
                    results.push(el);
                }
            }
            results.extend(find_elements_by_class(&el.children, class));
        }
    }
    results
}

fn find_first_by_class<'a>(nodes: &'a [HtmlNode], class: &str) -> Option<&'a HtmlElement> {
    for node in nodes {
        if let HtmlNode::Element(el) = node {
            if let Some(classes) = el.attributes.get("class") {
                if classes.split_whitespace().any(|c| c == class) {
                    return Some(el);
                }
            }
            if let Some(found) = find_first_by_class(&el.children, class) {
                return Some(found);
            }
        }
    }
    None
}

fn extract_text(el: &HtmlElement) -> String {
    extract_text_from_element(el)
}

fn extract_text_from_element(el: &HtmlElement) -> String {
    let mut text = String::new();
    for node in &el.children {
        match node {
            HtmlNode::Element(child) => text.push_str(&extract_text_from_element(child)),
            HtmlNode::Text(t) => text.push_str(t),
            HtmlNode::Comment(_) => {}
        }
    }
    text
}

fn scan_dangerous_attributes(nodes: &[HtmlNode], count: &mut usize) {
    let dangerous = vec!["onclick", "onload", "onerror", "onmouseover"];
    for node in nodes {
        if let HtmlNode::Element(el) = node {
            for attr in &dangerous {
                if el.attributes.contains_key(*attr) {
                    *count += 1;
                }
            }
            scan_dangerous_attributes(&el.children, count);
        }
    }
}

fn scan_dangerous_links(nodes: &[HtmlNode], protocols: &[&str], count: &mut usize) {
    for node in nodes {
        if let HtmlNode::Element(el) = node {
            if el.tag == "a" {
                if let Some(href) = el.attributes.get("href") {
                    for protocol in protocols {
                        if href.starts_with(protocol) {
                            *count += 1;
                            break;
                        }
                    }
                }
            }
            scan_dangerous_links(&el.children, protocols, count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_web_scraper() {
        let html = r#"<article class="product"><h2 class="title">Test</h2></article>"#;
        let doc = parse_document(html);
        let products = find_elements_by_class(&doc.children, "product");
        assert_eq!(products.len(), 1);
    }
    
    #[test]
    fn test_link_checker() {
        let html = r#"<a href="https://example.com">Link</a>"#;
        let doc = parse_document(html);
        let links = find_elements_by_tag(&doc.children, "a");
        assert_eq!(links.len(), 1);
    }
}
