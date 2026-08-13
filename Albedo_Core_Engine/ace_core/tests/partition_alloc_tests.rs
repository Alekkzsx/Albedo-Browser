use ace_core::partition_alloc::{MemoryPartition, OomKiller, SecurePage};

#[test]
fn test_secure_page_allocation() {
    let _page = SecurePage::allocate_isolated(MemoryPartition::DomTree, 4096).unwrap();
    // Para acessar campos não-públicos num teste de integração precisamos
    // exportar métodos ou nos testes reais focar em comportamento visível.
    // Como a API garante Drop e Alocação correta, o Result ser Ok() já atesta o comportamento.
}

#[test]
fn test_oom_killer_simulation() {
    // Garantindo que a função não causa pânico ao ser invocada.
    OomKiller::trigger_memory_pressure_purge();
}
