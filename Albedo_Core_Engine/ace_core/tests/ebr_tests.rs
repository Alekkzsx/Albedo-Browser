use ace_core::ebr::{AtomicEbr, Guard};

#[test]
fn test_ebr_basic_pinning() {
    let data = AtomicEbr::new(42);

    // 1. Thread "A" faz o Pin (inicia leitura lock-free)
    let guard = Guard::pin();
    let val_ref = data.load(&guard);

    assert_eq!(val_ref, Some(&42));

    // 2. Simulamos a deleção
    data.defer_destroy(&guard);

    // Na implementação EBR completa, val_ref AINDA estaria vivo aqui!
}
