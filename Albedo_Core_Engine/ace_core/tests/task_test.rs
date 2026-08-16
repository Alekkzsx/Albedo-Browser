use ace_core::task::spawn_safe;
use ace_core::error::AceError;

#[test]
fn test_spawn_safe_catches_panics() {
    // Tarefa segura
    let success = spawn_safe(|| 42);
    assert_eq!(success.unwrap(), 42);

    // Tarefa com Panic
    let failed = spawn_safe(|| {
        panic!("Divisão por zero no layout CSS simulada!");
    });
    
    assert!(failed.is_err());
    let err = failed.unwrap_err();
    match err {
        AceError::InvalidOperation(msg) => {
            assert!(msg.contains("Divisão por zero"));
        }
        _ => panic!("Esperava InvalidOperation"),
    }
}
