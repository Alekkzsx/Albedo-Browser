use ace_core::slab::Slab;

#[test]
fn test_slab_basic_operations() {
    let mut slab = Slab::new();

    let id0 = slab.insert(100);
    let id1 = slab.insert(200);
    let id2 = slab.insert(300);

    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(slab.len(), 3);

    assert_eq!(slab.get(1), Some(&200));

    // Remove um elemento do meio, mandando-o pra free-list
    let val = slab.remove(1);
    assert_eq!(val, 200);
    assert_eq!(slab.len(), 2);
    assert_eq!(slab.get(1), None);

    // Na próxima inserção, ele DEVE reutilizar o ID 1
    let id3 = slab.insert(400);
    assert_eq!(id3, 1);
    assert_eq!(slab.len(), 3);
    assert_eq!(slab.get(1), Some(&400));
}

#[test]
fn test_slab_memory_leak_stress() {
    // 1 Milhão de ciclos atestando a robustez (Alocação + Desalocação)
    let mut slab = Slab::with_capacity(100);

    // Preenche com 100 itens (ID 0 a 99)
    for i in 0..100 {
        slab.insert(i);
    }

    assert_eq!(slab.len(), 100);

    // Fazemos um milhão de remoções e re-inserções aleatórias.
    // Se a free-list estourar ou vazar memória (allocs no vector base sem controle),
    // a RAM explodiria em um ambiente de produção.
    for i in 0..1_000_000 {
        let slot = i % 100;

        let removed = slab.remove(slot as usize);
        assert_eq!(removed, slot);

        // Re-insere. Pela matemática do Slab, o ID retornado DEVE ser o mesmo `slot`.
        let new_id = slab.insert(slot);
        assert_eq!(new_id, slot as usize);
    }

    // O len deve continuar sendo 100. Nenhuma expansão descontrolada aconteceu.
    assert_eq!(slab.len(), 100);
}
