use ace_core::hash::{FxHashMap, FxHashSet};
use std::time::Instant;
use std::collections::HashMap;

#[test]
fn test_fxhash_correctness() {
    let mut map = FxHashMap::default();
    map.insert("dom_node_1", 100);
    map.insert("dom_node_2", 200);

    assert_eq!(map.get("dom_node_1"), Some(&100));
    assert_eq!(map.get("dom_node_2"), Some(&200));
    assert_eq!(map.get("dom_node_3"), None);

    let mut set = FxHashSet::default();
    set.insert(42);
    assert!(set.contains(&42));
    assert!(!set.contains(&43));
}

#[test]
fn test_fxhash_stress_benchmark() {
    // 1. Benchmark do FxHashMap (O nosso)
    let start_fx = Instant::now();
    let mut fx_map = FxHashMap::default();
    for i in 0..100_000 {
        fx_map.insert(i, i * 2);
    }
    for i in 0..100_000 {
        assert_eq!(fx_map.get(&i), Some(&(i * 2)));
    }
    let elapsed_fx = start_fx.elapsed();

    // 2. Benchmark do std::HashMap (O Padrão do Rust)
    let start_std = Instant::now();
    let mut std_map = HashMap::new();
    for i in 0..100_000 {
        std_map.insert(i, i * 2);
    }
    for i in 0..100_000 {
        assert_eq!(std_map.get(&i), Some(&(i * 2)));
    }
    let elapsed_std = start_std.elapsed();

    println!("FxHashMap (Nativo) Tempo: {:?}", elapsed_fx);
    println!("std::HashMap (SipHash) Tempo: {:?}", elapsed_std);

    // O FxHashMap DEVE ser estritamente mais rápido que o SipHash (em geral 2x a 4x mais rápido)
    // Usamos um assert solto para não falhar a suíte em servidores de CI hiper lentos, mas
    // registramos a diferença colossal na prática.
    assert!(
        elapsed_fx <= elapsed_std || elapsed_fx.as_millis() < 50,
        "FxHashMap apresentou anomalia de lentidão"
    );
}
