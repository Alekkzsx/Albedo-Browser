//! # Sistema de Flags e Features em Tempo de Execução
//!
//! Controle central de funcionalidades experimentais e APIs do motor de navegação.

pub mod registry;
pub mod utils;

pub use registry::{Feature, RuntimeFeatures};
pub use utils::parse_feature_overrides;
