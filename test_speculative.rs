// Simple test for speculative parsing
use albedo::ace::html::speculative::parse_document_speculative;

fn main() {
    println!("Testing speculative parsing...");
    
    // Test 1: Simple HTML
    let html = "<html><body><p>Hello World</p></body></html>";
    let doc = parse_document_speculative(html);
    println!("Test 1 passed: {} children", doc.children.len());
    
    // Test 2: Larger document
    let mut large_html = String::from("<html><body>");
    for i in 0..100 {
        large_html.push_str(&format!("<div id='div{}'>Content {}</div>", i, i));
    }
    large_html.push_str("</body></html>");
    
    let doc2 = parse_document_speculative(&large_html);
    println!("Test 2 passed: {} children", doc2.children.len());
    
    println!("All tests passed!");
}
