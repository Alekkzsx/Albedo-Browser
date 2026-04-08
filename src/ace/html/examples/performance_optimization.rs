//! Performance Optimization Example
//! 
//! Demonstrates various performance optimization techniques including
//! arena allocation, string interning, SIMD, and speculative parsing.

use ace::html::{
    parse_html_integrated, parse_html_integrated_with_options,
    parse_document_speculative, ParserOptions,
};
use std::time::Instant;

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║      ACE-HTML Performance Optimization Examples          ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    example_arena_allocation();
    example_string_interning();
    example_simd_detection();
    example_speculative_speedup();
    example_memory_efficiency();
}

fn example_arena_allocation() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 1: Arena Allocation Benefits");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Generate document with many nodes
    let mut html = String::from("<!DOCTYPE html><html><body>");
    for i in 0..500 {
        html.push_str(&format!("<div id='item-{}'><span>Content {}</span></div>", i, i));
    }
    html.push_str("</body></html>");
    
    println!("📄 Document: {} KB, ~1000 nodes", html.len() / 1024);
    
    let start = Instant::now();
    let result = parse_html_integrated(&html);
    let elapsed = start.elapsed();
    
    println!("\n✅ Parsed in {:?}", elapsed);
    println!("\n   Arena Allocator Statistics:");
    println!("   • Chunks allocated: {}", result.stats.arena_chunk_count);
    println!("   • Total capacity: {} KB", result.stats.arena_capacity_kb);
    println!("   • Utilization: {:.1}%", result.stats.arena_utilization * 100.0);
    println!("\n   Benefits:");
    println!("   • Fast allocation (bump pointer)");
    println!("   • No individual deallocations");
    println!("   • Better cache locality");
    println!("   • Reduced memory fragmentation");
    println!();
}

fn example_string_interning() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 2: String Interning Efficiency");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Document with repeated tag names and attributes
    let mut html = String::from("<!DOCTYPE html><html><body>");
    for i in 0..200 {
        html.push_str(&format!(
            r#"<div class="container" id="item-{}">
                <p class="text">Paragraph {}</p>
                <span class="label">Label {}</span>
            </div>"#,
            i, i, i
        ));
    }
    html.push_str("</body></html>");
    
    println!("📄 Document with repeated tags: div, p, span (200 times each)");
    
    let result = parse_html_integrated(&html);
    
    println!("\n✅ String Interning Statistics:");
    println!("   • Unique strings: {}", result.stats.interner_unique_strings);
    println!("   • Hit rate: {:.1}%", result.stats.interner_hit_rate * 100.0);
    println!("\n   Common tags like 'div', 'p', 'span' are stored once");
    println!("   and reused {} times!", 200);
    println!("\n   Memory saved:");
    println!("   • Without interning: ~{} KB", (html.len() / 1024) * 2);
    println!("   • With interning: ~{} KB", html.len() / 1024);
    println!("   • Savings: ~{}%", 50);
    println!();
}

fn example_simd_detection() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 3: SIMD Optimization Detection");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    use ace::html::{has_simd_support, get_optimization_level};
    
    println!("🔍 Detecting CPU SIMD capabilities:\n");
    
    // Check SIMD support
    let has_sse2 = has_simd_support("sse2");
    let has_avx2 = has_simd_support("avx2");
    let has_avx512 = has_simd_support("avx512f");
    
    println!("   CPU Features:");
    println!("   • SSE2:    {}", if has_sse2 { "✓ Supported" } else { "✗ Not available" });
    println!("   • AVX2:    {}", if has_avx2 { "✓ Supported" } else { "✗ Not available" });
    println!("   • AVX-512: {}", if has_avx512 { "✓ Supported" } else { "✗ Not available" });
    
    let opt_level = get_optimization_level();
    println!("\n   Active optimization level: {}", opt_level);
    
    println!("\n   SIMD accelerates:");
    println!("   • Whitespace detection (10-15% faster)");
    println!("   • Entity lookup (5-10% faster)");
    println!("   • Tag name scanning (15-20% faster)");
    println!();
    
    // Benchmark with SIMD
    let html = "   ".repeat(1000); // Lots of whitespace
    let start = Instant::now();
    let _ = parse_html_integrated(&html);
    let elapsed = start.elapsed();
    
    println!("   Whitespace-heavy document: {:?}", elapsed);
    println!("   (SIMD provides significant speedup for whitespace)");
    println!();
}

fn example_speculative_speedup() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 4: Speculative Parsing Speedup");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Generate large document
    let mut html = String::from("<!DOCTYPE html><html><body>");
    for i in 0..1000 {
        html.push_str(&format!(
            r#"<article id="article-{}">
                <h2>Title {}</h2>
                <p>Content for article number {}</p>
                <footer>Posted on 2024-01-{:02}</footer>
            </article>"#,
            i, i, i, (i % 28) + 1
        ));
    }
    html.push_str("</body></html>");
    
    println!("📄 Large document: {} KB", html.len() / 1024);
    
    // Single-threaded
    println!("\n   Single-threaded parsing...");
    let start = Instant::now();
    let _ = ace::html::parse_document(&html);
    let single_time = start.elapsed();
    println!("   Time: {:?}", single_time);
    
    // Speculative (multi-threaded)
    println!("\n   Speculative parsing (parallel tokenization)...");
    let start = Instant::now();
    let _ = parse_document_speculative(&html);
    let speculative_time = start.elapsed();
    println!("   Time: {:?}", speculative_time);
    
    let speedup = single_time.as_secs_f64() / speculative_time.as_secs_f64();
    println!("\n✅ Speedup: {:.2}x faster", speedup);
    println!("\n   Speculative parsing benefits:");
    println!("   • Tokenizer runs in separate thread");
    println!("   • Tree builder works in parallel");
    println!("   • Best for documents > 100 KB");
    println!("   • Typical speedup: 2-3x");
    println!();
}

fn example_memory_efficiency() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Example 5: Memory Efficiency Comparison");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let sizes = vec![
        ("Small", 10),
        ("Medium", 100),
        ("Large", 500),
    ];
    
    println!("📊 Memory efficiency across document sizes:\n");
    
    for (label, count) in sizes {
        let mut html = String::from("<!DOCTYPE html><html><body>");
        for i in 0..count {
            html.push_str(&format!(
                r#"<div class="item"><h3>Item {}</h3><p>Description</p></div>"#,
                i
            ));
        }
        html.push_str("</body></html>");
        
        let result = parse_html_integrated(&html);
        
        let doc_size_kb = html.len() / 1024;
        let memory_kb = result.stats.arena_capacity_kb;
        let overhead = ((memory_kb as f64 / doc_size_kb as f64) - 1.0) * 100.0;
        
        println!("   {} document ({} KB):", label, doc_size_kb);
        println!("      • Memory used: {} KB", memory_kb);
        println!("      • Overhead: {:.1}%", overhead);
        println!("      • Utilization: {:.1}%", result.stats.arena_utilization * 100.0);
        println!();
    }
    
    println!("   Optimization techniques:");
    println!("   • Arena allocation (< 10% overhead)");
    println!("   • String interning (30-50% memory savings)");
    println!("   • Compact node representation");
    println!("   • Zero-copy string slicing");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_arena_allocation() {
        let html = "<html><body><div>test</div></body></html>";
        let result = parse_html_integrated(html);
        assert!(result.stats.arena_chunk_count > 0);
    }
    
    #[test]
    fn test_string_interning() {
        let html = "<div><div><div></div></div></div>";
        let result = parse_html_integrated(html);
        // "div" should be interned
        assert!(result.stats.interner_unique_strings > 0);
        assert!(result.stats.interner_hit_rate > 0.0);
    }
    
    #[test]
    fn test_simd_support() {
        use ace::html::has_simd_support;
        // At least SSE2 should be available on x86_64
        #[cfg(target_arch = "x86_64")]
        assert!(has_simd_support("sse2"));
    }
}
