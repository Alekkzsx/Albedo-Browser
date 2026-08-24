//! # Flags de Recursos e Parâmetros em Tempo de Execução (Runtime Features & FeatureParams)
//!
//! Registro de flags de features lock-free baseado em `AtomicBitSet` e suporte a parâmetros tipados
//! dinâmicos (`FeatureParam<T>`, padrão Chromium `base::FeatureParam`), permitindo testes A/B,
//! parametrização de buffers e ativação gradual de novas tecnologias sem necessidade de recompilação.

use crate::collections::AtomicBitSet;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::sync::atomic::Ordering;
use std::sync::LazyLock;

/// Identificadores das funcionalidades do motor do navegador controláveis em tempo de execução.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Feature {
    /// Suporte a CSS Flexbox Layout.
    CssFlexbox = 0,
    /// Suporte a CSS Grid Layout.
    CssGrid = 1,
    /// Suporte a CSS Subgrid.
    CssSubgrid = 2,
    /// Compilação e execução de WebAssembly (Wasm).
    WebAssembly = 3,
    /// Contexto gráfico WebGL / GPU.
    WebGl = 4,
    /// Renderização em threads secundárias via OffscreenCanvas.
    OffscreenCanvas = 5,
    /// API de requisições de rede `fetch()`.
    FetchApi = 6,
    /// Persistência em disco via LocalStorage.
    LocalStorage = 7,
    /// Banco de dados estruturado IndexedDB.
    IndexedDb = 8,
    /// Protocolo de depuração e inspeção DevTools.
    DevTools = 9,
    /// Comunicação bidirecional via WebSockets.
    WebSocket = 10,
    /// Service Workers e Cache Storage.
    ServiceWorker = 11,
}

static GLOBAL_FEATURES: LazyLock<RuntimeFeatures> = LazyLock::new(RuntimeFeatures::new_default);

/// Valor de um parâmetro dinâmico de feature.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

/// Parâmetro tipado associado a uma `Feature` (Chromium `base::FeatureParam<T>`).
#[derive(Debug, Clone)]
pub struct FeatureParam<T> {
    pub feature: Feature,
    pub name: &'static str,
    pub default_value: T,
}

impl<T: Clone> FeatureParam<T> {
    /// Cria uma nova definição de parâmetro com valor padrão.
    pub const fn new(feature: Feature, name: &'static str, default_value: T) -> Self {
        Self {
            feature,
            name,
            default_value,
        }
    }
}

impl FeatureParam<bool> {
    /// Retorna o valor booleano atual do parâmetro considerando ativação da feature e overrides.
    pub fn get(&self) -> bool {
        if !RuntimeFeatures::global().is_enabled(self.feature) {
            return self.default_value;
        }
        let overrides = RuntimeFeatures::global().param_overrides.read();
        if let Some(ParamValue::Bool(v)) = overrides.get(&(self.feature, self.name)) {
            *v
        } else {
            self.default_value
        }
    }
}

impl FeatureParam<i64> {
    /// Retorna o valor inteiro atual do parâmetro.
    pub fn get(&self) -> i64 {
        if !RuntimeFeatures::global().is_enabled(self.feature) {
            return self.default_value;
        }
        let overrides = RuntimeFeatures::global().param_overrides.read();
        if let Some(ParamValue::Int(v)) = overrides.get(&(self.feature, self.name)) {
            *v
        } else {
            self.default_value
        }
    }
}

impl FeatureParam<f64> {
    /// Retorna o valor de ponto flutuante atual do parâmetro.
    pub fn get(&self) -> f64 {
        if !RuntimeFeatures::global().is_enabled(self.feature) {
            return self.default_value;
        }
        let overrides = RuntimeFeatures::global().param_overrides.read();
        if let Some(ParamValue::Float(v)) = overrides.get(&(self.feature, self.name)) {
            *v
        } else {
            self.default_value
        }
    }
}

impl FeatureParam<String> {
    /// Retorna o valor textual atual do parâmetro.
    pub fn get(&self) -> String {
        if !RuntimeFeatures::global().is_enabled(self.feature) {
            return self.default_value.clone();
        }
        let overrides = RuntimeFeatures::global().param_overrides.read();
        if let Some(ParamValue::Str(v)) = overrides.get(&(self.feature, self.name)) {
            v.clone()
        } else {
            self.default_value.clone()
        }
    }
}

/// Gerenciador de flags de funcionalidades do navegador com acesso atômico $O(1)$ sem locks.
pub struct RuntimeFeatures {
    flags: AtomicBitSet<4>, // 256 flags
    param_overrides: RwLock<FxHashMap<(Feature, &'static str), ParamValue>>,
}

impl RuntimeFeatures {
    /// Cria uma nova instância de features zerada.
    pub fn new() -> Self {
        Self {
            flags: AtomicBitSet::new(),
            param_overrides: RwLock::new(FxHashMap::default()),
        }
    }

    /// Cria uma instância com os recursos padrão de produção habilitados.
    pub fn new_default() -> Self {
        let registry = Self::new();
        registry.reset_defaults();
        registry
    }

    /// Retorna a instância global padrão compartilhada por todo o motor.
    #[inline]
    pub fn global() -> &'static Self {
        &GLOBAL_FEATURES
    }

    /// Retorna `true` se o recurso especificado estiver ativo.
    #[inline]
    pub fn is_enabled(&self, feature: Feature) -> bool {
        self.flags.get(feature as usize, Ordering::Relaxed)
    }

    /// Habilita um recurso do motor.
    #[inline]
    pub fn enable(&self, feature: Feature) {
        self.flags.set(feature as usize, true, Ordering::SeqCst);
    }

    /// Desabilita um recurso do motor.
    #[inline]
    pub fn disable(&self, feature: Feature) {
        self.flags.set(feature as usize, false, Ordering::SeqCst);
    }

    /// Define o estado de ativação de um recurso.
    #[inline]
    pub fn set(&self, feature: Feature, enabled: bool) {
        self.flags.set(feature as usize, enabled, Ordering::SeqCst);
    }

    /// Define um override para um parâmetro de feature (para testes A/B ou flags de CLI).
    pub fn set_param_override(&self, feature: Feature, name: &'static str, value: ParamValue) {
        self.param_overrides.write().insert((feature, name), value);
    }

    /// Limpa todos os overrides de parâmetros.
    pub fn clear_param_overrides(&self) {
        self.param_overrides.write().clear();
    }

    /// Restaura a configuração padrão das features do navegador.
    pub fn reset_defaults(&self) {
        self.flags.clear(Ordering::SeqCst);
        self.enable(Feature::CssFlexbox);
        self.enable(Feature::CssGrid);
        self.enable(Feature::WebAssembly);
        self.enable(Feature::FetchApi);
        self.enable(Feature::LocalStorage);
        self.enable(Feature::WebSocket);
        self.clear_param_overrides();
    }

    /// Atalho global estático: verifica se um recurso está habilitado no motor.
    #[inline]
    pub fn is_feature_enabled(feature: Feature) -> bool {
        Self::global().is_enabled(feature)
    }
}

impl Default for RuntimeFeatures {
    fn default() -> Self {
        Self::new_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_flags_and_params() {
        let registry = RuntimeFeatures::global();
        registry.reset_defaults();

        assert!(registry.is_enabled(Feature::CssGrid));
        assert!(!registry.is_enabled(Feature::CssSubgrid));

        let subgrid_depth = FeatureParam::new(Feature::CssSubgrid, "max_depth", 8i64);
        assert_eq!(subgrid_depth.get(), 8); // Feature desabilitada retorna padrão

        registry.enable(Feature::CssSubgrid);
        assert_eq!(subgrid_depth.get(), 8);

        registry.set_param_override(Feature::CssSubgrid, "max_depth", ParamValue::Int(16));
        assert_eq!(subgrid_depth.get(), 16); // Override aplicado com sucesso

        registry.reset_defaults();
    }
}
