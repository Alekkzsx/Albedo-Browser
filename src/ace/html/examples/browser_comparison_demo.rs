//! Browser Comparison Demo
//! 
//! This example demonstrates how to use the side-by-side browser comparison
//! functionality to benchmark ACE HTML parser against Chrome and Firefox.
//! 
//! Usage:
//!   cargo run --example browser_comparison_demo

use std::fs;

fn main() {
    println!("🔬 ACE HTML Parser - Browser Comparison Demo\n");
    
    // Import the comparison runner
    use albedo::ace::html::tests::browser_comparison::{
        BrowserComparisonRunner,
        generate_comprehensive_report,
    };
    
    let runner = BrowserComparisonRunner::new();
    
    // Check browser availability
    let availability = runner.check_availability();
    println!("📋 Browser Availability:");
    println!("  Chrome:  {}", if availability.chrome { "✓" } else { "✗" });
    println!("  Firefox: {}", if availability.firefox { "✓" } else { "✗" });
    
    if !availability.any_available() {
        println!("\n⚠️  No browsers available for comparison!");
        println!("To enable browser comparisons:");
        println!("  1. Install Node.js");
        println!("  2. Run setup script in benchmarks/ directory");
        println!("     - For Chrome: npm install puppeteer");
        println!("     - For Firefox: npm install playwright");
        return;
    }
    
    println!("\n" + &"=".repeat(80));
    println!("Running Benchmark Suite");
    println!("{}\n", "=".repeat(80));
    
    let mut comparisons = Vec::new();
    
    // Benchmark 1: Simple document
    println!("\n1️⃣  Simple Document");
    let simple = "<html><body><div>Hello World</div></body></html>";
    comparisons.push(runner.compare_all_parsers(simple, "Simple Document"));
    
    // Benchmark 2: Nested structure
    println!("\n2️⃣  Nested Structure");
    let nested = r#"
        <html>
            <body>
                <div class="container">
                    <header>
                        <h1>Title</h1>
                        <nav>
                            <ul>
                                <li><a href="#">Link 1</a></li>
                                <li><a href="#">Link 2</a></li>
                                <li><a href="#">Link 3</a></li>
                            </ul>
                        </nav>
                    </header>
                    <main>
                        <article>
                            <p>Paragraph 1</p>
                            <p>Paragraph 2</p>
                            <p>Paragraph 3</p>
                        </article>
                    </main>
                    <footer>
                        <p>Footer content</p>
                    </footer>
                </div>
            </body>
        </html>
    "#;
    comparisons.push(runner.compare_all_parsers(nested, "Nested Structure"));
    
    // Benchmark 3: Attribute-heavy
    println!("\n3️⃣  Attribute-Heavy Document");
    let attrs = r#"
        <div id="main" class="container wrapper" data-value="123" data-name="test" style="color: red;">
            <span id="s1" class="text highlight" data-index="1" title="Span 1">Text 1</span>
            <span id="s2" class="text highlight" data-index="2" title="Span 2">Text 2</span>
            <span id="s3" class="text highlight" data-index="3" title="Span 3">Text 3</span>
        </div>
    "#;
    comparisons.push(runner.compare_all_parsers(attrs, "Attribute-Heavy"));
    
    // Benchmark 4: Table structure
    println!("\n4️⃣  Table Structure");
    let table = r#"
        <table>
            <thead>
                <tr>
                    <th>Name</th>
                    <th>Age</th>
                    <th>City</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Alice</td>
                    <td>30</td>
                    <td>New York</td>
                </tr>
                <tr>
                    <td>Bob</td>
                    <td>25</td>
                    <td>London</td>
                </tr>
                <tr>
                    <td>Charlie</td>
                    <td>35</td>
                    <td>Tokyo</td>
                </tr>
            </tbody>
        </table>
    "#;
    comparisons.push(runner.compare_all_parsers(table, "Table Structure"));
    
    // Benchmark 5: Large document
    println!("\n5️⃣  Large Document");
    let mut large = String::from("<html><body>");
    for i in 0..200 {
        large.push_str(&format!(
            r#"<div class="item-{}" id="item-{}"><p>Content {}</p></div>"#,
            i, i, i
        ));
    }
    large.push_str("</body></html>");
    comparisons.push(runner.compare_all_parsers(&large, "Large Document (200 divs)"));
    
    // Print individual comparisons
    println!("\n" + &"=".repeat(80));
    println!("Individual Results");
    println!("{}\n", "=".repeat(80));
    
    for comparison in &comparisons {
        comparison.print_comparison();
    }
    
    // Generate comprehensive report
    println!("\n" + &"=".repeat(80));
    println!("Generating Comprehensive Report");
    println!("{}\n", "=".repeat(80));
    
    let report = generate_comprehensive_report(&comparisons);
    
    // Save report to file
    let report_path = "target/browser_comparison_report.md";
    match fs::write(report_path, &report) {
        Ok(_) => {
            println!("✓ Report saved to: {}", report_path);
        }
        Err(e) => {
            println!("⚠ Failed to save report: {}", e);
        }
    }
    
    // Print report to console
    println!("\n{}", report);
    
    println!("\n" + &"=".repeat(80));
    println!("Demo Complete!");
    println!("{}", "=".repeat(80));
}
