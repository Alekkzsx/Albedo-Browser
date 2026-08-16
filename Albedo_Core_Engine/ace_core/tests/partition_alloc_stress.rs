// ============================================================================
// Albedo Core Engine (ACE)
// File: partition_alloc_stress.rs
// Description: Testes de stress e segurança para o PartitionAlloc (Guard Pages,
//              Ofuscação de FreeList e Quarentena de liberação anti-UAF).
// Author: Albedo Browser Engineering Team
// ============================================================================

use ace_core::partition_alloc::{
    decode_freelist_ptr, encode_freelist_ptr, MemoryPartition, QuarantineRing, SecurePage,
};

#[test]
fn test_secure_page_guard_page_boundaries() {
    let page_size = 4096;
    let page = SecurePage::allocate_isolated(MemoryPartition::DomTree, page_size)
        .expect("Falha ao alocar SecurePage");

    assert_eq!(page.partition(), MemoryPartition::DomTree);
    assert_eq!(page.data_size(), page_size);

    let data_ptr = page.data_ptr();
    assert!(!data_ptr.is_null());
    assert!(page.contains_ptr(data_ptr));

    // Escrita segura na região útil
    unsafe {
        for i in 0..page_size {
            *data_ptr.add(i) = (i % 256) as u8;
        }
        for i in 0..page_size {
            assert_eq!(*data_ptr.add(i), (i % 256) as u8);
        }
    }
}

#[test]
fn test_freelist_pointer_obfuscation() {
    let dummy_ptr = std::ptr::null_mut::<u8>().wrapping_add(0x7FFF12345000);
    let encoded = encode_freelist_ptr(dummy_ptr);
    assert_ne!(encoded, 0);
    assert_ne!(encoded, dummy_ptr as usize);

    let decoded = decode_freelist_ptr(encoded);
    assert_eq!(decoded, dummy_ptr);

    // Null pointer encoding
    assert_eq!(encode_freelist_ptr(std::ptr::null_mut()), 0);
    assert_eq!(decode_freelist_ptr(0), std::ptr::null_mut());
}

#[test]
fn test_quarantine_ring_eviction_and_drain() {
    let mut ring = QuarantineRing::new();

    // Inserindo 64 elementos (capacidade total)
    for i in 1..=64 {
        let dummy = std::ptr::null_mut::<u8>().wrapping_add(i);
        let evicted = ring.push(dummy);
        assert_eq!(evicted, None);
    }

    // O 65º elemento deve expulsar o 1º da quarentena
    let dummy_65 = std::ptr::null_mut::<u8>().wrapping_add(65);
    let evicted = ring.push(dummy_65);
    assert_eq!(evicted, Some(std::ptr::null_mut::<u8>().wrapping_add(1)));

    // O 66º elemento deve expulsar o 2º
    let dummy_66 = std::ptr::null_mut::<u8>().wrapping_add(66);
    let evicted_2 = ring.push(dummy_66);
    assert_eq!(evicted_2, Some(std::ptr::null_mut::<u8>().wrapping_add(2)));

    // Drain all remaining
    let mut collected = Vec::new();
    ring.drain_all(|ptr| {
        collected.push(ptr as usize);
    });

    assert_eq!(collected.len(), 64);
    assert_eq!(collected[0], 3);
    assert_eq!(collected[62], 65);
    assert_eq!(collected[63], 66);
}
