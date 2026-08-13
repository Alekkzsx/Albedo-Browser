use ace_core::intern;
use ace_core::bloom::BloomFilter;
use ace_core::hash::fxhash32;
use std::sync::Arc;
use std::thread;

#[test]
fn test_stress_interner_multithread() {
    let num_threads = 8;
    let items_per_thread = 10_000;
    
    let mut handles = vec![];
    
    for t in 0..num_threads {
        handles.push(thread::spawn(move || {
            let mut symbols = Vec::with_capacity(items_per_thread);
            for i in 0..items_per_thread {
                // Simula extração de classes CSS ou IDs no DOM
                let s = format!("albedo-class-{}-{}", t, i);
                symbols.push(intern::intern(&s));
            }
            symbols
        }));
    }
    
    for h in handles {
        let symbols = h.join().unwrap();
        assert_eq!(symbols.len(), items_per_thread);
        // Garante que o interner resolve de volta e está acessível e válido
        let resolved = intern::resolve(symbols[0]);
        assert!(resolved.is_some());
    }
}

#[test]
fn test_stress_bloom_filter_heavy_load() {
    let mut bloom = BloomFilter::new();
    
    let dataset_size = 1500; // Máximo recomendado para 2048 bits mantendo FPR baixo
    
    // Insere 1500 pseudo-hashes (Simulando uma árvore DOM profunda)
    for i in 0..dataset_size {
        let hash = fxhash32(format!("node-{}", i).as_bytes());
        bloom.insert(hash);
    }
    
    let query_size = 5_000_000;
    let mut hits = 0;
    
    // Checa 5 milhões de hashes aleatórios (Simulando a cascata CSS)
    for i in 0..query_size {
        // Usamos wrapping add e mult para manter a velocidade alta no gerador pseudo-aleatório do teste
        let hash = (i as u32).wrapping_mul(1103515245).wrapping_add(12345);
        if bloom.might_contain(hash) {
            hits += 1;
        }
    }
    
    let false_positive_rate = hits as f64 / query_size as f64;
    // O filtro com 2 func hashes, 2048 bits e n=1500, deve ter teóricos FPR < 30%.
    assert!(false_positive_rate < 0.40, "FPR disparou: {}%", false_positive_rate * 100.0);
}
