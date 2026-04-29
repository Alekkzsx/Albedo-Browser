use std::time::Instant;
use std::fs;

fn main() {
    println!("=== ACE-HTML Pipeline Breakdown ===\n");

    let html = fs::read_to_string("tests/html5lib/tree-construction/tests10.dat")
        .map(|_| fs::read_to_string("stress_test.html").unwrap_or_else(|_| {
            "<!DOCTYPE html><html><head><title>test</title></head><body><div class=\"item\"><span>content</span></div></body></html>".to_string()
        }))
        .unwrap_or_else(|_| {
            // Fallback: criar HTML grande para benchmark
            let mut html = String::from("<!DOCTYPE html><html><head><title>test</title></head><body>");
            for i in 0..10000 {
                html.push_str(&format!("<div class=\"item{}\"><span>content {}</span></div>", i, i));
            }
            html.push_str("</body></html>");
            html
        });

    let html = &html;
    println!("HTML size: {} bytes\n", html.len());

    println!("Running 20 iterations...\n");

    for i in 0..20 {
        let start = Instant::now();
        let _ = albedo::ace::html::parse_document(html);
        let elapsed = start.elapsed();
        
        if i < 3 {
            println!("Iteration {}: {:?}", i + 1, elapsed);
        }
    }
    
    println!("\nDone!");
}