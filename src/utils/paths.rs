use std::path::PathBuf;

/// Retorna o diretório de cache apropriado para o Albedo.
/// Tenta usar o diretório home do usuário (~/.cache/albedo), caindo para o diretório temporário se indisponível.
pub fn cache_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        std::path::Path::new(&home).join(".cache").join("albedo")
    } else {
        std::env::temp_dir().join("albedo-cache")
    }
}

/// Retorna o diretório de configuração/dados do Albedo.
/// Tenta usar o diretório home do usuário (~/.config/albedo), caindo para o diretório temporário se indisponível.
pub fn config_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        std::path::Path::new(&home).join(".config").join("albedo")
    } else {
        std::env::temp_dir().join("albedo-config")
    }
}
