//! Browser Comparison Tests
//! 
//! Tests for the side-by-side browser comparison functionality.

#[cfg(test)]
use crate::ace::html::tests::browser_comparison::{BrowserComparisonRunner, generate_comprehensive_report};

#[test]
fn test_simple_document_comparison() {
    let runner = BrowserComparisonRunner::new();
    let html = "<html><body><div>Hello World</div></body></html>";
    
    let comparison = runner.compare_all_parsers(html, "Simple Document");
    comparison.print_comparison();
    
    assert_eq!(comparison.name, "Simple Document");
    assert!(comparison.ace_result.stats.mean.as_secs_f64() > 0.0);
}

#[test]
fn test_nested_elements_comparison() {
    let runner = BrowserComparisonRunner::new();
    let html = r##"
        <html>
            <body>
                <div class="container">
                    <header>
                        <h1>Title</h1>
                        <nav>
                            <ul>
                                <li><a href="#">Link 1</a></li>
                                <li><a href="#">Link 2</a></li>
                            </ul>
                        </nav>
                    </header>
                    <main>
                        <article>
                            <p>Content paragraph 1</p>
                            <p>Content paragraph 2</p>
                        </article>
                    </main>
                </div>
            </body>
        </html>
    "##;
    
    let comparison = runner.compare_all_parsers(html, "Nested Elements");
    comparison.print_comparison();
    
    assert_eq!(comparison.name, "Nested Elements");
}

#[test]
fn test_attributes_heavy_comparison() {
    let runner = BrowserComparisonRunner::new();
    let html = r##"
        <div id="main" class="container wrapper" data-value="123" data-name="test" style="color: red;">
            <span id="s1" class="text" data-index="1">Text 1</span>
            <span id="s2" class="text" data-index="2">Text 2</span>
            <span id="s3" class="text" data-index="3">Text 3</span>
        </div>
    "##;
    
    let comparison = runner.compare_all_parsers(html, "Attributes Heavy");
    comparison.print_comparison();
    
    assert_eq!(comparison.name, "Attributes Heavy");
}

#[test]
fn test_table_comparison() {
    let runner = BrowserComparisonRunner::new();
    let html = r##"
        <table>
            <thead>
                <tr>
                    <th>Header 1</th>
                    <th>Header 2</th>
                    <th>Header 3</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Cell 1-1</td>
                    <td>Cell 1-2</td>
                    <td>Cell 1-3</td>
                </tr>
                <tr>
                    <td>Cell 2-1</td>
                    <td>Cell 2-2</td>
                    <td>Cell 2-3</td>
                </tr>
            </tbody>
        </table>
    "##;
    
    let comparison = runner.compare_all_parsers(html, "Table Structure");
    comparison.print_comparison();
    
    assert_eq!(comparison.name, "Table Structure");
}

#[test]
fn test_entities_comparison() {
    let runner = BrowserComparisonRunner::new();
    let html = r##"
        <div>
            &lt;div&gt; &amp; &quot;quotes&quot; &apos;apostrophe&apos;
            &nbsp; &copy; &reg; &trade;
            &#65; &#x41; &#x1F600;
        </div>
    "##;
    
    let comparison = runner.compare_all_parsers(html, "Entity References");
    comparison.print_comparison();
    
    assert_eq!(comparison.name, "Entity References");
}

#[test]
fn test_script_and_style_comparison() {
    let runner = BrowserComparisonRunner::new();
    let html = r##"
        <html>
            <head>
                <style>
                    body { margin: 0; }
                    .container { width: 100%; }
                </style>
                <script>
                    function test() {
                        console.log("test");
                    }
                </script>
            </head>
            <body>
                <div class="container">Content</div>
            </body>
        </html>
    "##;
    
    let comparison = runner.compare_all_parsers(html, "Script and Style");
    comparison.print_comparison();
    
    assert_eq!(comparison.name, "Script and Style");
}

#[test]
fn test_comprehensive_report_generation() {
    let runner = BrowserComparisonRunner::new();
    
    let mut comparisons = Vec::new();
    
    // Run multiple benchmarks
    let test_cases = vec![
        ("<div>Simple</div>", "Simple"),
        ("<div><p>Nested</p></div>", "Nested"),
        (r##"<div class="test" id="main">Attributes</div>"##, "Attributes"),
    ];
    
    for (html, name) in test_cases {
        let comparison = runner.compare_all_parsers(html, name);
        comparisons.push(comparison);
    }
    
    // Generate comprehensive report
    let report = generate_comprehensive_report(&comparisons);
    
    println!("\n{}", report);
    
    // Verify report structure
    assert!(report.contains("# ACE HTML Parser - Comprehensive Browser Comparison"));
    assert!(report.contains("## Performance Comparison Summary"));
    assert!(report.contains("## Detailed Statistics"));
    assert!(report.contains("## Performance Insights"));
    assert!(report.contains("Simple"));
    assert!(report.contains("Nested"));
    assert!(report.contains("Attributes"));
}

#[test]
fn test_large_document_comparison() {
    let runner = BrowserComparisonRunner::new();
    
    // Generate a larger document
    let mut html = String::from("<html><body>");
    for i in 0..100 {
        html.push_str(&format!(
            r##"<div class="item-{}" id="item-{}"><p>Content {}</p></div>"##,
            i, i, i
        ));
    }
    html.push_str("</body></html>");
    
    let comparison = runner.compare_all_parsers(&html, "Large Document (100 divs)");
    comparison.print_comparison();
    
    assert_eq!(comparison.name, "Large Document (100 divs)");
}

#[test]
fn test_browser_availability_check() {
    let runner = BrowserComparisonRunner::new();
    let availability = runner.check_availability();
    
    println!("\n📋 Browser Availability:");
    println!("  Chrome:  {}", if availability.chrome { "✓ Available" } else { "✗ Not available" });
    println!("  Firefox: {}", if availability.firefox { "✓ Available" } else { "✗ Not available" });
    
    if availability.any_available() {
        println!("\n✓ At least one browser is available for comparison");
    } else {
        println!("\n⚠ No browsers available for comparison");
        println!("  To enable browser comparisons:");
        println!("  1. Install Node.js");
        println!("  2. Run setup script in benchmarks/ directory");
    }
}

#[test]
fn test_multiple_benchmarks_with_report() {
    let runner = BrowserComparisonRunner::new();
    let availability = runner.check_availability();
    
    if !availability.any_available() {
        println!("⚠ Skipping test: No browsers available");
        return;
    }
    
    let mut comparisons = Vec::new();
    
    // Micro benchmarks
    comparisons.push(runner.compare_all_parsers("<div>test</div>", "Micro: Simple Tag"));
    comparisons.push(runner.compare_all_parsers(
        r##"<div class="x" id="y">test</div>"##,
        "Micro: Attributes"
    ));
    
    // Small document
    let small_doc = r##"
        <html>
            <head><title>Test</title></head>
            <body>
                <div class="container">
                    <h1>Title</h1>
                    <p>Paragraph</p>
                </div>
            </body>
        </html>
    "##;
    comparisons.push(runner.compare_all_parsers(small_doc, "Small Document"));
    
    // Generate and save report
    let report = generate_comprehensive_report(&comparisons);
    
    // Save to file
    if let Err(e) = std::fs::write("target/browser_comparison_report.md", &report) {
        println!("⚠ Failed to write report: {}", e);
    } else {
        println!("\n✓ Report saved to: target/browser_comparison_report.md");
    }
    
    println!("\n{}", report);
}
