use ace_core::id::{NodeId, RequestId, TabId};
use std::mem::size_of;

#[test]
fn test_null_pointer_optimization() {
    // Prova Matemática Absoluta da Memória:
    // Graças ao NonZeroU64, o compilador Rust otimiza a Variante "None" de Option<Id>
    // para usar o próprio valor do 0 subjacente da memória.
    // Assim, Option<Id> DEVE pesar exatos 8 bytes, e não 16.
    
    assert_eq!(size_of::<NodeId>(), 8, "NodeId deve pesar 8 bytes");
    assert_eq!(size_of::<Option<NodeId>>(), 8, "Option<NodeId> sofreu falha de Null Pointer Optimization!");
    
    assert_eq!(size_of::<RequestId>(), 8);
    assert_eq!(size_of::<Option<RequestId>>(), 8);
    
    assert_eq!(size_of::<TabId>(), 8);
    assert_eq!(size_of::<Option<TabId>>(), 8);
}

#[test]
fn test_id_generation() {
    let id1 = NodeId::new();
    let id2 = NodeId::new();
    
    assert_ne!(id1, id2, "IDs gerados atômicamente devem ser únicos");
    assert!(id2.raw() > id1.raw(), "IDs devem ser monotonicamente crescentes");
}

#[test]
fn test_id_from_raw() {
    // raw id 0 deve retornar None, pois é NonZeroU64
    let invalid = NodeId::from_raw(0);
    assert!(invalid.is_none(), "ID zero deve ser rejeitado no parse raw");
    
    let valid = NodeId::from_raw(150).unwrap();
    assert_eq!(valid.raw(), 150);
}
