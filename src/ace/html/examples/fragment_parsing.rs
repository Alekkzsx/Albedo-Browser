//! Fragment Parsing Example
//! 
//! Demonstrates how to parse HTML fragments (like innerHTML) with different
//! context elements. Fragment parsing is used when inserting HTML into an
//! existing document.

use ace::html::{
    parse_fragment, parse_fragment_with_context, parse_fragment_with_errors_and_context,
    FragmentContext, ParserOptions, Namespace,
};

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         ACE-HTML Fragment Parsing Examples              ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    example_basic_fragment();
    example_table_context();
    example_svg_context();
    example_list_context();
    example_error_handling();
}

fn example_basic_fragment() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 1: Basic Fragment Parsing (body context)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<div class="container">
        <h1>Hello World</h1>
        <p>This is a fragment.</p>
    </div>"#;

    // Parse with default context (body)
    let nodes = parse_fragment(html, Some("body"));
    
    println!("✅ Parsed {} root node(s)", nodes.len());
    println!("   Fragment: {}", html.trim());
    println!();
}

fn example_table_context() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 2: Table Context (tbody)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<tr>
        <td>Cell 1</td>
        <td>Cell 2</td>
        <td>Cell 3</td>
    </tr>"#;

    // Parse as if inside a <tbody> element
    let context = FragmentContext::new("tbody");
    let nodes = parse_fragment_with_context(html, Some(&context), &ParserOptions::default());
    
    println!("✅ Parsed table row in tbody context");
    println!("   Nodes: {}", nodes.len());
    println!("   Context: tbody (table body)");
    println!();
    
    // Compare with body context (incorrect)
    let nodes_wrong = parse_fragment(html, Some("body"));
    println!("⚠️  Same HTML in body context: {} nodes", nodes_wrong.len());
    println!("   (Different parsing behavior!)");
    println!();
}

fn example_svg_context() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 3: SVG Context (foreign content)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<circle cx="50" cy="50" r="40" fill="blue"/>
    <rect x="10" y="10" width="80" height="80" fill="red"/>"#;

    // Parse as SVG content
    let context = FragmentContext::new("svg")
        .with_namespace(Namespace::Svg);
    
    let nodes = parse_fragment_with_context(html, Some(&context), &ParserOptions::default());
    
    println!("✅ Parsed SVG elements");
    println!("   Nodes: {}", nodes.len());
    println!("   Namespace: SVG");
    println!("   Elements: circle, rect");
    println!();
}

fn example_list_context() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 4: List Context (ul/ol)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<li>Item 1</li>
    <li>Item 2</li>
    <li>Item 3</li>"#;

    // Parse as list items
    let context = FragmentContext::new("ul");
    let nodes = parse_fragment_with_context(html, Some(&context), &ParserOptions::default());
    
    println!("✅ Parsed list items in ul context");
    println!("   Nodes: {}", nodes.len());
    println!("   Context: ul (unordered list)");
    println!();
}

fn example_error_handling() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 5: Fragment with Errors");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<div>
        <p>Unclosed paragraph
        <span>Nested span</p>
    </div>"#;

    let context = FragmentContext::new("body");
    let output = parse_fragment_with_errors_and_context(
        html,
        Some(&context),
        &ParserOptions::default()
    );
    
    println!("✅ Parsed fragment with errors");
    println!("   Nodes: {}", output.document.children.len());
    println!("   Errors: {}", output.errors.len());
    
    if !output.errors.is_empty() {
        println!("\n   Parse errors:");
        for (i, error) in output.errors.iter().take(3).enumerate() {
            println!("   {}. {}", i + 1, error.message);
        }
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fragment_basic() {
        let html = "<div>test</div>";
        let nodes = parse_fragment(html, Some("body"));
        assert_eq!(nodes.len(), 1);
    }
    
    #[test]
    fn test_fragment_table_context() {
        let html = "<tr><td>cell</td></tr>";
        let context = FragmentContext::new("tbody");
        let nodes = parse_fragment_with_context(html, Some(&context), &ParserOptions::default());
        assert!(!nodes.is_empty());
    }
    
    #[test]
    fn test_fragment_svg_context() {
        let html = r#"<circle cx="50" cy="50" r="40"/>"#;
        let context = FragmentContext::new("svg").with_namespace(Namespace::Svg);
        let nodes = parse_fragment_with_context(html, Some(&context), &ParserOptions::default());
        assert!(!nodes.is_empty());
    }
}
