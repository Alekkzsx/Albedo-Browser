// Standalone test for many attributes benchmark
// This avoids the rquickjs dependency issue

use std::time::{Duration, Instant};

// Simplified version to test if the code compiles and runs
fn main() {
    println!("Testing many attributes stress test...\n");
    
    // Generate HTML with 100 elements, each with 100 attributes
    let html = generate_many_attributes();
    
    println!("=== Stress Test: Many Attributes (100 per element) ===");
    println!("Document size: {} bytes ({:.2} MB)", html.len(), html.len() as f64 / 1_000_000.0);
    println!("Elements: 100, Attributes per element: 100");
    println!("\nNote: This is a simplified test to verify the HTML generation.");
    println!("The actual benchmark requires the full ACE-HTML parser.\n");
    
    // Verify the HTML structure
    let elem_count = html.matches("<div id='elem-").count();
    let attr_count = html.matches("data-attr-").count();
    
    println!("Generated elements: {}", elem_count);
    println!("Generated attributes: {}", attr_count);
    println!("Expected attributes: {}", 100 * 100);
    
    if elem_count == 100 && attr_count == 10000 {
        println!("\n✓ HTML generation is correct!");
        println!("✓ Test structure matches specification (100 elements × 100 attributes)");
    } else {
        println!("\n✗ HTML generation mismatch!");
        println!("  Expected: 100 elements, 10000 attributes");
        println!("  Got: {} elements, {} attributes", elem_count, attr_count);
    }
}

fn generate_many_attributes() -> String {
    let mut html = String::from("<!DOCTYPE html><html><body>");
    
    // Generate 100 elements, each with 100 attributes
    for i in 0..100 {
        html.push_str(&format!("<div id='elem-{}'", i));
        
        // Add 100 attributes
        for j in 0..100 {
            html.push_str(&format!(" data-attr-{}='value-{}'", j, j));
        }
        
        html.push_str(&format!(">Element {}</div>", i));
    }
    
    html.push_str("</body></html>");
    html
}
