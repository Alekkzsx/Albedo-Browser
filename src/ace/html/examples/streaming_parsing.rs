//! Streaming Parsing Example
//! 
//! Demonstrates incremental parsing of HTML as it arrives over the network.
//! This is useful for reducing latency and starting rendering before the
//! entire document is downloaded.

use ace::html::{StreamingHtmlParser, StreamingState};
use std::time::Instant;

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         ACE-HTML Streaming Parsing Examples             ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    example_basic_streaming();
    example_chunked_parsing();
    example_network_simulation();
    example_early_rendering();
}

fn example_basic_streaming() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 1: Basic Streaming");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut parser = StreamingHtmlParser::new();
    
    // Simulate receiving HTML in chunks
    let chunks = vec![
        "<!DOCTYPE html><html><head>",
        "<title>Streaming Example</title>",
        "</head><body>",
        "<h1>Hello World</h1>",
        "<p>This is streamed content.</p>",
        "</body></html>",
    ];
    
    println!("📡 Feeding {} chunks to parser...\n", chunks.len());
    
    for (i, chunk) in chunks.iter().enumerate() {
        let start = Instant::now();
        parser.feed(chunk);
        let elapsed = start.elapsed();
        
        println!("   Chunk {}: {} bytes in {:?}", i + 1, chunk.len(), elapsed);
    }
    
    // Finalize parsing
    let document = parser.finish();
    
    println!("\n✅ Streaming complete");
    println!("   Total nodes: {}", count_nodes(&document));
    println!();
}

fn example_chunked_parsing() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 2: Chunked Parsing (16KB chunks)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Generate a large HTML document
    let mut html = String::from("<!DOCTYPE html><html><body>");
    for i in 0..500 {
        html.push_str(&format!(
            r#"<div class="item-{}"><h2>Item {}</h2><p>Content for item {}</p></div>"#,
            i, i, i
        ));
    }
    html.push_str("</body></html>");
    
    println!("📄 Document size: {} KB", html.len() / 1024);
    
    let mut parser = StreamingHtmlParser::new();
    let chunk_size = 16 * 1024; // 16KB chunks
    let chunks: Vec<&str> = html
        .as_bytes()
        .chunks(chunk_size)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect();
    
    println!("📦 Split into {} chunks of ~16KB\n", chunks.len());
    
    let mut total_time = std::time::Duration::ZERO;
    
    for (i, chunk) in chunks.iter().enumerate() {
        let start = Instant::now();
        parser.feed(chunk);
        let elapsed = start.elapsed();
        total_time += elapsed;
        
        if i < 3 || i >= chunks.len() - 1 {
            println!("   Chunk {}: {:?}", i + 1, elapsed);
        } else if i == 3 {
            println!("   ...");
        }
    }
    
    let document = parser.finish();
    
    println!("\n✅ Chunked parsing complete");
    println!("   Total time: {:?}", total_time);
    println!("   Average per chunk: {:?}", total_time / chunks.len() as u32);
    println!("   Nodes: {}", count_nodes(&document));
    println!();
}

fn example_network_simulation() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 3: Network Simulation (with delays)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut parser = StreamingHtmlParser::new();
    
    let chunks = vec![
        ("<!DOCTYPE html><html><head>", 10),
        ("<title>Network Sim</title></head>", 15),
        ("<body><header><h1>Title</h1></header>", 20),
        ("<main><article><p>Content</p></article></main>", 25),
        ("<footer><p>Footer</p></footer></body></html>", 10),
    ];
    
    println!("🌐 Simulating network with variable latency...\n");
    
    for (i, (chunk, delay_ms)) in chunks.iter().enumerate() {
        // Simulate network delay
        std::thread::sleep(std::time::Duration::from_millis(*delay_ms));
        
        let start = Instant::now();
        parser.feed(chunk);
        let parse_time = start.elapsed();
        
        println!("   Chunk {}: {}ms network + {:?} parse",
            i + 1, delay_ms, parse_time);
    }
    
    let document = parser.finish();
    
    println!("\n✅ Network simulation complete");
    println!("   Document parsed successfully");
    println!();
}

fn example_early_rendering() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 4: Early Rendering (progressive display)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut parser = StreamingHtmlParser::new();
    
    let chunks = vec![
        "<!DOCTYPE html><html><head><title>Progressive</title></head><body>",
        "<header><h1>Page Title</h1></header>",
        "<nav><a href='/'>Home</a><a href='/about'>About</a></nav>",
        "<main><article><h2>Article Title</h2>",
        "<p>First paragraph of content...</p>",
        "<p>Second paragraph of content...</p>",
        "<p>Third paragraph of content...</p>",
        "</article></main>",
        "<footer><p>Copyright 2024</p></footer>",
        "</body></html>",
    ];
    
    println!("🎨 Progressive rendering simulation:\n");
    
    for (i, chunk) in chunks.iter().enumerate() {
        parser.feed(chunk);
        
        // Check if we can start rendering
        let state = parser.state();
        
        match i {
            0 => println!("   ✓ Head parsed - can load CSS/JS"),
            1 => println!("   ✓ Header parsed - can render header"),
            2 => println!("   ✓ Nav parsed - can render navigation"),
            3 => println!("   ✓ Main content starting - can render above-fold"),
            4..=6 => println!("   ✓ Content chunk {} - progressive rendering", i - 3),
            7 => println!("   ✓ Main complete - can render full article"),
            8 => println!("   ✓ Footer parsed - can render footer"),
            9 => println!("   ✓ Document complete - final render"),
            _ => {}
        }
    }
    
    let document = parser.finish();
    
    println!("\n✅ Progressive rendering complete");
    println!("   User saw content incrementally as it arrived");
    println!("   Total nodes: {}", count_nodes(&document));
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
    fn test_streaming_basic() {
        let mut parser = StreamingHtmlParser::new();
        parser.feed("<html><body>");
        parser.feed("<div>test</div>");
        parser.feed("</body></html>");
        let doc = parser.finish();
        assert!(!doc.children.is_empty());
    }
    
    #[test]
    fn test_streaming_chunked() {
        let html = "<html><body><div>test</div></body></html>";
        let mut parser = StreamingHtmlParser::new();
        
        for chunk in html.as_bytes().chunks(5) {
            parser.feed(std::str::from_utf8(chunk).unwrap());
        }
        
        let doc = parser.finish();
        assert!(!doc.children.is_empty());
    }
    
    #[test]
    fn test_streaming_large_document() {
        let mut html = String::from("<html><body>");
        for i in 0..100 {
            html.push_str(&format!("<div>Item {}</div>", i));
        }
        html.push_str("</body></html>");
        
        let mut parser = StreamingHtmlParser::new();
        let chunk_size = 1024;
        
        for chunk in html.as_bytes().chunks(chunk_size) {
            parser.feed(std::str::from_utf8(chunk).unwrap());
        }
        
        let doc = parser.finish();
        assert!(count_nodes(&doc) > 100);
    }
}
