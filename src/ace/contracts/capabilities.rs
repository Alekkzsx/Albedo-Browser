//! # ACE-Contracts: Engine Capabilities
//! 
//! Define o versionamento do Albedo Browser e as flags de funcionalidades ativas.
//! Essencial para o JIT e as WebAPIs consultarem o que a engine suporta no momento.
//!
//! > [!IMPORTANT]
//! > **ATUALIZAÇÃO QUASE SEMPRE**: À medida que novas camadas (Tier 2, WebGL) 
//! > forem ativadas, este arquivo será atualizado.

/// Versão semântica oficial do Albedo Browser.
pub const ALBEDO_VERSION: &str = "0.1.0-alpha";

/// Representa a versão estável do contrato atual.
pub enum ContractVersion {
    V1_0,
}

/// Flags de recursos habilitados na Core Engine.
pub struct AlbedoCapabilities {
    pub has_jit_tier2: bool,
    pub has_json_nd: bool,
    pub has_wasm: bool,
    pub has_gpu: bool,
}

impl Default for AlbedoCapabilities {
    fn default() -> Self {
        Self {
            has_jit_tier2: true, // Tier 2 Turbo habilitado por padrão
            has_json_nd: true,   // JSON nativo habilitado
            has_wasm: false,     // WIP
            has_gpu: false,      // WIP
        }
    }
}
