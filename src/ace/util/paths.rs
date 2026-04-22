pub fn cache_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("albedo-cache")
}

pub fn config_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("albedo-config")
}
