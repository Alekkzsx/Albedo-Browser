//! # Metadados de Compilação e Identificação do Motor (BuildInfo)
//!
//! Fornece informações sobre a versão do navegador, arquitetura de destino,
//! e geração do cabeçalho canônico `User-Agent` HTTP conforme os padrões da web.

/// Informações e metadados de compilação do Albedo Core Engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildInfo {
    /// Versão semântica completa (ex: `"0.1.0"`).
    pub version: &'static str,
    /// Versão principal (Major).
    pub major: u32,
    /// Versão secundária (Minor).
    pub minor: u32,
    /// Versão de correção (Patch).
    pub patch: u32,
    /// Sistema operacional alvo de compilação.
    pub target_os: &'static str,
    /// Arquitetura de CPU alvo de compilação.
    pub target_arch: &'static str,
    /// Perfil de compilação (`"debug"` ou `"release"`).
    pub build_profile: &'static str,
}

impl BuildInfo {
    /// Retorna os metadados oficiais da compilação atual.
    pub const fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            major: 0,
            minor: 1,
            patch: 0,
            target_os: std::env::consts::OS,
            target_arch: std::env::consts::ARCH,
            build_profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            },
        }
    }

    /// Gera uma string canônica de `User-Agent` compatível com a especificação HTTP (RFC 9110)
    /// e com o ecossistema moderno da web (compatibilidade com Chromium/Safari).
    pub fn default_user_agent(app_name: &str) -> String {
        let os_desc = match std::env::consts::OS {
            "windows" => "Windows NT 10.0; Win64; x64",
            "macos" => "Macintosh; Intel Mac OS X 10_15_7",
            "linux" => "X11; Linux x86_64",
            _ => "Unknown OS",
        };

        let engine_ver = env!("CARGO_PKG_VERSION");
        format!(
            "Mozilla/5.0 ({}) AppleWebKit/537.36 (KHTML, like Gecko) {}/{} Chrome/130.0.0.0 Safari/537.36",
            os_desc, app_name, engine_ver
        )
    }
}
