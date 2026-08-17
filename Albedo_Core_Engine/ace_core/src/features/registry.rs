//! # Flags de Recursos em Tempo de Execução (Runtime Features)
//!
//! Registro de flags de features lock-free baseado em `AtomicBitSet`, permitindo habilitar/desabilitar
//! tecnologias experimentais (CSS Subgrid, WebGPU, DevTools) sem necessidade de recompilação.

use crate::collections::AtomicBitSet;
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

/// Gerenciador de flags de funcionalidades do navegador com acesso atômico $O(1)$ sem locks.
pub struct RuntimeFeatures {
    flags: AtomicBitSet<4>, // 256 flags
}

impl RuntimeFeatures {
    /// Cria uma nova instância de features zerada.
    pub fn new() -> Self {
        Self {
            flags: AtomicBitSet::new(),
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

    /// Restaura a configuração padrão das features do navegador.
    pub fn reset_defaults(&self) {
        self.flags.clear(Ordering::SeqCst);
        self.enable(Feature::CssFlexbox);
        self.enable(Feature::CssGrid);
        self.enable(Feature::WebAssembly);
        self.enable(Feature::FetchApi);
        self.enable(Feature::LocalStorage);
        self.enable(Feature::WebSocket);
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
