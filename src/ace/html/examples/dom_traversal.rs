//! DOM Tree Traversal Example
//! 
//! Demonstrates how to work with the parsed DOM tree, including
//! traversal, querying, and manipulation patterns.

use ace::html::{parse_document, HtmlNode, HtmlElement};

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         ACE-HTML DOM Traversal Examples                 ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    example_basic_traversal();
    example_find_elements();
    example_extract_text();
    example_attribute_access();
    example_tree_statistics();
}

fn example_basic_traversal() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 1: Basic Tree Traversal");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Traversal Example</title>
</head>
<body>
    <header>
        <h1>Title</h1>
    </header>
    <main>
        <p>Paragraph 1</p>
        <p>Paragraph 2</p>
    </main>
</body>
</html>"#;

    let doc = parse_document(html);
    
    println!("✅ Document structure:\n");
    print_tree(&doc.children, 0);
    println!();
}

fn example_find_elements() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 2: Finding Elements by Tag Name");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<body>
    <div class="container">
        <p>First paragraph</p>
        <div class="nested">
            <p>Nested paragraph</p>
            <span>Span text</span>
        </div>
        <p>Last paragraph</p>
    </div>
</body>
</html>"#;

    let doc = parse_document(html);
    
    // Find all <p> elements
    let paragraphs = find_elements_by_tag(&doc.children, "p");
    println!("✅ Found {} <p> elements:\n", paragraphs.len());
    
    for (i, p) in paragraphs.iter().enumerate() {
        let text = extract_text_from_element(p);
        println!("   {}. {}", i + 1, text);
    }
    
    println!();
    
    // Find all <div> elements
    let divs = find_elements_by_tag(&doc.children, "div");
    println!("   Found {} <div> elements", divs.len());
    
    for div in &divs {
        if let Some(class) = div.attributes.get("class") {
            println!("      • class=\"{}\"", class);
        }
    }
    println!();
}

fn example_extract_text() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 3: Extracting Text Content");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<body>
    <article>
        <h1>Article Title</h1>
        <p>First paragraph with <strong>bold text</strong>.</p>
        <p>Second paragraph with <em>italic text</em>.</p>
        <!-- This is a comment -->
        <p>Third paragraph.</p>
    </article>
</body>
</html>"#;

    let doc = parse_document(html);
    
    // Find article element
    if let Some(article) = find_first_element_by_tag(&doc.children, "article") {
        let text = extract_all_text(&article.children);
        
        println!("✅ Extracted text from <article>:\n");
        println!("{}", text.trim());
        println!();
    }
}

fn example_attribute_access() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 4: Accessing Attributes");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<body>
    <a href="https://example.com" target="_blank" rel="noopener">Link 1</a>
    <a href="/page" class="internal">Link 2</a>
    <img src="image.jpg" alt="Description" width="800" height="600">
    <input type="text" name="username" placeholder="Enter name" required>
</body>
</html>"#;

    let doc = parse_document(html);
    
    println!("✅ Element attributes:\n");
    
    // Find all <a> elements
    let links = find_elements_by_tag(&doc.children, "a");
    println!("   Links:");
    for link in &links {
        if let Some(href) = link.attributes.get("href") {
            let target = link.attributes.get("target")
                .map(|t| format!(" target=\"{}\"", t))
                .unwrap_or_default();
            println!("      • href=\"{}\"{}", href, target);
        }
    }
    println!();
    
    // Find <img> element
    if let Some(img) = find_first_element_by_tag(&doc.children, "img") {
        println!("   Image:");
        println!("      • src: {}", img.attributes.get("src").unwrap_or(&"".to_string()));
        println!("      • alt: {}", img.attributes.get("alt").unwrap_or(&"".to_string()));
        if let Some(width) = img.attributes.get("width") {
            println!("      • dimensions: {}x{}", width, 
                img.attributes.get("height").unwrap_or(&"?".to_string()));
        }
    }
    println!();
    
    // Find <input> element
    if let Some(input) = find_first_element_by_tag(&doc.children, "input") {
        println!("   Input:");
        for (key, value) in &input.attributes {
            println!("      • {}=\"{}\"", key, value);
        }
    }
    println!();
}

fn example_tree_statistics() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 5: Tree Statistics");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Statistics Example</title>
    <meta charset="UTF-8">
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <header>
        <nav>
            <ul>
                <li><a href="/">Home</a></li>
                <li><a href="/about">About</a></li>
                <li><a href="/contact">Contact</a></li>
            </ul>
        </nav>
    </header>
    <main>
        <article>
            <h1>Title</h1>
            <p>Paragraph 1</p>
            <p>Paragraph 2</p>
            <p>Paragraph 3</p>
        </article>
    </main>
    <footer>
        <p>Copyright 2024</p>
    </footer>
</body>
</html>"#;

    let doc = parse_document(html);
    
    let stats = calculate_tree_stats(&doc.children);
    
    println!("✅ Tree statistics:\n");
    println!("   Total nodes: {}", stats.total_nodes);
    println!("   Elements: {}", stats.element_count);
    println!("   Text nodes: {}", stats.text_count);
    println!("   Comments: {}", stats.comment_count);
    println!("   Max depth: {}", stats.max_depth);
    println!();
    
    println!("   Element breakdown:");
    let mut sorted_tags: Vec<_> = stats.tag_counts.iter().collect();
    sorted_tags.sort_by(|a, b| b.1.cmp(a.1));
    
    for (tag, count) in sorted_tags.iter().take(10) {
        println!("      • <{}>: {}", tag, count);
    }
    println!();
}

// Helper functions

fn print_tree(nodes: &[HtmlNode], depth: usize) {
    let indent = "  ".repeat(depth);
    
    for node in nodes {
        match node {
            HtmlNode::Element(el) => {
                let attrs = if el.attributes.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", el.attributes.keys()
                        .map(|k| k.as_str())
                        .collect::<Vec<_>>()
                        .join(", "))
                };
                println!("{}<{}>{}", indent, el.tag, attrs);
                print_tree(&el.children, depth + 1);
            }
            HtmlNode::Text(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    println!("{}\"{}\"", indent, trimmed);
                }
            }
            HtmlNode::Comment(comment) => {
                println!("{}<!-- {} -->", indent, comment.trim());
            }
        }
    }
}

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

fn extract_text_from_element(el: &HtmlElement) -> String {
    extract_all_text(&el.children)
}

fn extract_all_text(nodes: &[HtmlNode]) -> String {
    let mut text = String::new();
    
    for node in nodes {
        match node {
            HtmlNode::Element(el) => {
                text.push_str(&extract_all_text(&el.children));
            }
            HtmlNode::Text(t) => {
                text.push_str(t);
            }
            HtmlNode::Comment(_) => {}
        }
    }
    
    text
}

struct TreeStats {
    total_nodes: usize,
    element_count: usize,
    text_count: usize,
    comment_count: usize,
    max_depth: usize,
    tag_counts: std::collections::HashMap<String, usize>,
}

fn calculate_tree_stats(nodes: &[HtmlNode]) -> TreeStats {
    let mut stats = TreeStats {
        total_nodes: 0,
        element_count: 0,
        text_count: 0,
        comment_count: 0,
        max_depth: 0,
        tag_counts: std::collections::HashMap::new(),
    };
    
    calculate_stats_recursive(nodes, 0, &mut stats);
    stats
}

fn calculate_stats_recursive(nodes: &[HtmlNode], depth: usize, stats: &mut TreeStats) {
    stats.max_depth = stats.max_depth.max(depth);
    
    for node in nodes {
        stats.total_nodes += 1;
        
        match node {
            HtmlNode::Element(el) => {
                stats.element_count += 1;
                *stats.tag_counts.entry(el.tag.clone()).or_insert(0) += 1;
                calculate_stats_recursive(&el.children, depth + 1, stats);
            }
            HtmlNode::Text(_) => {
                stats.text_count += 1;
            }
            HtmlNode::Comment(_) => {
                stats.comment_count += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_find_elements() {
        let html = "<html><body><p>1</p><div><p>2</p></div></body></html>";
        let doc = parse_document(html);
        let paragraphs = find_elements_by_tag(&doc.children, "p");
        assert_eq!(paragraphs.len(), 2);
    }
    
    #[test]
    fn test_extract_text() {
        let html = "<div>Hello <span>World</span>!</div>";
        let doc = parse_document(html);
        if let Some(div) = find_first_element_by_tag(&doc.children, "div") {
            let text = extract_text_from_element(div);
            assert!(text.contains("Hello"));
            assert!(text.contains("World"));
        }
    }
    
    #[test]
    fn test_tree_stats() {
        let html = "<html><body><div><p>text</p></div></body></html>";
        let doc = parse_document(html);
        let stats = calculate_tree_stats(&doc.children);
        assert!(stats.element_count > 0);
        assert!(stats.text_count > 0);
    }
}
