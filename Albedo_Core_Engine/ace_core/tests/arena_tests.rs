use ace_core::arena::Arena;

#[test]
fn test_arena_alloc() {
    let arena = Arena::new();
    let a = arena.alloc(42i32);
    let b = arena.alloc(100f64);
    
    assert_eq!(*a, 42);
    assert_eq!(*b, 100.0);
    
    // Modificação mutável in-place
    *a = 50;
    assert_eq!(*a, 50);
}

#[test]
fn test_arena_mass_allocation() {
    let arena = Arena::new();
    // Disparando para forçar o limite dos 64KB e criar Múltiplas Páginas
    for i in 0..100_000 {
        let val = arena.alloc(i as u64);
        assert_eq!(*val, i as u64);
    }
    // Quando a arena dropar aqui, tudo (as dezenas de páginas) deve ser liberado perfeitamente
}
