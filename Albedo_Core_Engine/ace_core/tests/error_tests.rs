use ace_core::error::AceError;
use std::io::{Error, ErrorKind};

// Um mock de função que retorna um I/O Error e é convertido para AceError
fn read_mock_file() -> Result<(), AceError> {
    let io_err = Error::new(ErrorKind::NotFound, "file not found");
    // O operador ? invoca o `From<std::io::Error>` nativamente
    Err(io_err)?
}

#[test]
fn test_error_propagation_and_context() {
    let result = read_mock_file();

    assert!(result.is_err());

    let err = result.unwrap_err();

    // Adiciona contexto
    let err_with_context = err.with_context("Failed to initialize engine");

    // Converte para string para checar o display
    let err_str = format!("{}", err_with_context);

    assert!(err_str.contains("Failed to initialize engine"));
    assert!(err_str.contains("System I/O failure"));
}
