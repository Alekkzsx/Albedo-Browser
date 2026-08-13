use ace_core::bloom::BloomFilter;

#[test]
fn test_bloom_filter_basic() {
    let mut bloom = BloomFilter::new();
    
    // Hash dummy (na vida real isso viria do hasher do CSS)
    let hash1: u32 = 42;
    let hash2: u32 = 1337;
    let hash3: u32 = 9999;
    
    assert!(!bloom.might_contain(hash1));
    assert!(!bloom.might_contain(hash2));
    assert!(!bloom.might_contain(hash3));
    
    bloom.insert(hash1);
    
    // Agora pode conter
    assert!(bloom.might_contain(hash1));
    // E não deve conter os outros, a menos que haja colisão,
    // mas com poucos elementos, a chance de colisão é quase nula para nós
    assert!(!bloom.might_contain(hash2));
    assert!(!bloom.might_contain(hash3));
    
    bloom.clear();
    assert!(!bloom.might_contain(hash1));
}

#[test]
fn test_bloom_filter_false_positive_rate() {
    let mut bloom = BloomFilter::new();
    
    // Inserindo 500 elementos únicos simulados com simple LCG
    for i in 0..500 {
        let hash = (i as u32).wrapping_mul(2654435761u32) % 4294967295u32;
        bloom.insert(hash as u32);
    }
    
    let mut false_positives = 0;
    // Testando contra outros 10_000 hashes que sabemos que *não* inserimos (espalhados diferentemente)
    for i in 500..10_500 {
        let hash = (i as u32).wrapping_mul(2654435761u32) % 4294967295u32;
        if bloom.might_contain(hash as u32) {
            false_positives += 1;
        }
    }
    
    // A taxa de falsos positivos num filtro de 2048 bits com 2 hash functions
    // e 500 itens costuma ficar em torno de ~15-20%.
    // Isso quer dizer que nós rejeitamos 80%+ do lixo (que é a maioria da árvore DOM em CSS Matching)
    // O teste garante que o filtro ainda tem sanidade estatística.
    assert!(false_positives < 3500, "FPR muito alta: {} falsos positivos de 10_000 ({}%)", false_positives, (false_positives as f32 / 10000.0) * 100.0);
}
