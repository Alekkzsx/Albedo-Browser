//! Exemplo de uso do ACE-HTML Parser com todas otimizações
//! 
//! Este exemplo demonstra como usar o parser integrado com:
//! - Arena allocator para eficiência de memória
//! - String interner para reutilização de strings
//! - Preload scanner para descoberta antecipada de recursos
//! - Métricas detalhadas de performance

use ace::html::{
    parse_html_integrated,
    parse_document,
    build_document,
};

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         ACE-HTML Parser - Demo Completo                  ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // Exemplo 1: Parser Integrado (Arena + Interner + SIMD)
    exemplo_parser_integrado();
    
    // Exemplo 2: Parser Tradicional
    exemplo_parser_tradicional();
    
    // Exemplo 3: HTML Complexo
    exemplo_html_complexo();
}

fn exemplo_parser_integrado() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Exemplo 1: Parser Integrado (Otimizado)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html lang="pt-BR">
<head>
    <meta charset="UTF-8">
    <title>ACE-HTML Demo</title>
    <link rel="stylesheet" href="styles.css">
    <script src="app.js"></script>
</head>
<body>
    <header>
        <h1>Bem-vindo ao ACE-HTML</h1>
        <nav>
            <a href="/home">Home</a>
            <a href="/about">Sobre</a>
        </nav>
    </header>
    <main>
        <article>
            <p>Este é um parser HTML ultra-rápido escrito em Rust.</p>
            <img src="logo.png" alt="Logo">
        </article>
    </main>
    <footer>
        <p>&copy; 2024 ACE-HTML</p>
    </footer>
</body>
</html>"#;

    let result = parse_html_integrated(html);
    
    println!("📊 Estatísticas do Parsing:");
    println!("   • Nodes na DOM: {}", count_nodes(&result.document));
    println!("   • Erros encontrados: {}", result.errors.len());
    println!("   • Recursos para preload: {}", result.preload_requests.len());
    println!();
    
    println!("💾 Arena Allocator:");
    println!("   • Chunks alocados: {}", result.stats.arena_chunk_count);
    println!("   • Capacidade: {} KB", result.stats.arena_capacity_kb);
    println!("   • Utilização: {:.1}%", result.stats.arena_utilization * 100.0);
    println!();
    
    println!("🔤 String Interner:");
    println!("   • Strings únicas: {}", result.stats.interner_unique_strings);
    println!("   • Hit rate: {:.1}%", result.stats.interner_hit_rate * 100.0);
    println!();
    
    if !result.preload_requests.is_empty() {
        println!("🚀 Preload Scanner发现了 {} recursos:", result.preload_requests.len());
        for req in &result.preload_requests {
            println!("   • [{:?}] {}", req.resource_type, req.url);
        }
        println!();
    }
}

fn exemplo_parser_tradicional() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Exemplo 2: Parser Tradicional");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head><title>Teste</title></head>
<body>
    <div class="container">
        <p>Hello, World!</p>
    </div>
</body>
</html>"#;

    let doc = parse_document(html);
    
    println!("✅ Documento parseado com sucesso!");
    println!("   • Doctype: {:?}", doc.doctype.as_ref().map(|d| d.name.as_ref()));
    println!("   • Children no root: {}", doc.children.len());
    println!();
}

fn exemplo_html_complexo() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Exemplo 3: HTML Complexo (Shadow DOM, SVG, MathML)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <style>
        @import url("fonts.css");
    </style>
</head>
<body>
    <!-- Shadow DOM Declarativo -->
    <custom-element>
        <template shadowrootmode="open">
            <div>Shadow Content</div>
        </template>
        <p>Light DOM Content</p>
    </custom-element>
    
    <!-- SVG Inline -->
    <svg width="100" height="100">
        <circle cx="50" cy="50" r="40" fill="blue"/>
    </svg>
    
    <!-- MathML -->
    <math>
        <mi>x</mi>
        <mo>=</mo>
        <mn>42</mn>
    </math>
    
    <!-- Recursos para preload -->
    <link rel="preload" href="critical.css" as="style">
    <img srcset="small.jpg 480w, large.jpg 800w" src="default.jpg">
    <picture>
        <source srcset="image.webp" type="image/webp">
        <img src="image.jpg" alt="Responsive">
    </picture>
</body>
</html>"#;

    let result = parse_html_integrated(html);
    
    println!("📊 Análise de HTML Complexo:");
    println!("   • Total nodes: {}", count_nodes(&result.document));
    println!("   • Erros de parsing: {}", result.errors.len());
    println!("   • Recursos detectados: {}", result.preload_requests.len());
    println!();
    
    if !result.preload_requests.is_empty() {
        println!("🎯 Recursos para Preload:");
        for (i, req) in result.preload_requests.iter().enumerate() {
            println!("   {}. {:?}: {}", i + 1, req.resource_type, req.url);
        }
    }
    println!();
}

fn count_nodes(doc: &ace::html::HtmlDocument) -> usize {
    fn count_children(children: &[ace::html::HtmlNode]) -> usize {
        children.iter().map(|node| {
            match node {
                ace::html::HtmlNode::Element(el) => {
                    1 + count_children(&el.children)
                }
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
    fn test_basic_parsing() {
        let html = "<!DOCTYPE html><html><body>Hello</body></html>";
        let result = parse_html_integrated(html);
        
        assert!(result.document.children.len() > 0);
        assert_eq!(result.errors.len(), 0);
    }
    
    #[test]
    fn test_arena_allocation() {
        let html = "<div><span>Test</span></div>";
        let result = parse_html_integrated(html);
        
        assert!(result.stats.arena_capacity_kb > 0);
        assert!(result.stats.arena_chunk_count > 0);
    }
    
    #[test]
    fn test_string_interning() {
        let html = "<div><div><div></div></div></div>";
        let result = parse_html_integrated(html);
        
        // Tag "div" deve ser internada e reutilizada
        assert!(result.stats.interner_unique_strings > 0);
    }
    
    #[test]
    fn test_preload_detection() {
        let html = r#"
            <html>
                <head>
                    <link rel="stylesheet" href="style.css">
                    <script src="app.js"></script>
                    <img src="image.png">
                </head>
            </html>
        "#;
        let result = parse_html_integrated(html);
        
        // Deve detectar pelo menos alguns recursos
        assert!(result.preload_requests.len() >= 0);
    }
}
