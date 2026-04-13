use albedo::ace::html::HtmlTreeBuilder;
use albedo::ace::util::allocator::AceAllocator;
use std::time::Instant;
use std::fs;

fn main() {
    println!("--- ACE-HTML v2 Benchmark ---");
    
    let html = fs::read_to_string("stress_test.html").expect("Falha ao ler stress_test.html");
    println!("Tamanho do arquivo: {:.2} MB", html.len() as f64 / 1024.0 / 1024.0);

    let iterations = 20;
    let mut total_duration = std::time::Duration::default();

    // Aquecimento (Warm-up)
    {
        let allocator = AceAllocator::with_capacity(html.len() * 2);
        let mut builder = HtmlTreeBuilder::new(&html, &allocator);
        builder.run();
    }

    println!("Executando {} iterações...", iterations);

    for i in 0..iterations {
        let start = Instant::now();
        
        let allocator = AceAllocator::with_capacity(html.len() * 2);
        let mut builder = HtmlTreeBuilder::new(&html, &allocator);
        let nodes = builder.run();
        
        let duration = start.elapsed();
        total_duration += duration;
        
        if i == 0 {
            println!("Nós gerados: {}", nodes.len());
        }
    }

    let average = total_duration / iterations;
    println!("Tempo médio: {:?}", average);
    println!("Velocidade: {:.2} MB/s", (html.len() as f64 / 1024.0 / 1024.0) / average.as_secs_f64());
    println!("----------------------------");
}
