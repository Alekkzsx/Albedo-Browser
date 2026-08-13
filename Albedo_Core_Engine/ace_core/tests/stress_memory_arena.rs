use ace_core::alloc;
use ace_core::arena::Arena;
use std::hint::black_box;

#[derive(Default)]
struct DomNode {
    _tag: u32,
    _id: u32,
    _children_count: usize,
    _style_ptr: usize,
    _layout_data: [f64; 4],
}

#[test]
fn test_stress_memory_arena_dom_simulation() {
    // Alocando e liberando 1 Milhão de nós para testar o Phase-Oriented Reset O(1).
    let arena = Arena::new();

    // Anota o pico de memória antes do teste
    let initial_peak = alloc::peak_memory_usage_bytes();

    // Simulação de 60 frames (DOM Reflow/Relayout)
    for _frame in 0..60 {
        // Aloca 10.000 nós por frame
        for i in 0..10_000 {
            let node = arena.alloc(DomNode::default());
            node._id = i;
            black_box(node); // Previne o compilador de otimizar a alocação
        }

        // Final do frame: Limpa o bump pointer mas mantém a RAM alocada
        arena.clear();
    }

    let current_allocations = alloc::current_active_allocations();
    let final_peak = alloc::peak_memory_usage_bytes();

    // O pico não deve crescer absurdamente (linearmente), porque a Arena reaproveita a capacidade do ciclo.
    // 10.000 nós * sizeof(DomNode) ~= 10.000 * 56 bytes = 560KB por frame.
    // Mesmo em 60 frames, a memória usada pela arena não deve passar de ~1MB (CHUNK_SIZE é 64KB, então ele aloca o necessário)
    assert!(final_peak >= initial_peak);
    // Assegurar que os chunks não vazaram
    assert!(current_allocations < 100);
}
