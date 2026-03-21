//! # AlbedoJIT — Motor JavaScript JIT em Rust puro
//!
//! Usa **Cranelift** como backend de geração de código de máquina nativo.
//!
//! ## Arquitetura Híbrida
//!
//! - **Tier 0** — QuickJS Interpreter (cold code, startup instantâneo)
//! - **Tier 1** — Baseline JIT via Cranelift (compilação rápida, sem otimizações)
//! - **Tier 2** — AlbedoTurbo (type specialization, inlining, escape analysis)
//!
//! O QuickJS continua como interpretador base. O AlbedoJIT compila hot paths
//! para código de máquina nativo, acelerando funções executadas repetidamente.

pub mod air_interpreter;
pub mod baseline_compiler;
pub mod builtins;
pub mod bytecode;
pub mod code_cache;
pub mod decoder;
pub mod deopt;
pub mod executable_memory;
pub mod fast_builtins;
pub mod jit_bridge;
pub mod jit_engine;
pub mod js_value;
pub mod object_model;
pub mod profiler;
pub mod runtime_helpers;
pub mod tier2_compiler;
pub mod type_feedback;

// Re-exports públicos
pub use bytecode::{AirBuilder, AirFunction, AirOpcode};
#[cfg(test)]
mod jit_equivalence_tests;
pub use baseline_compiler::BaselineCompiler;
pub use builtins::BuiltinId;
pub use code_cache::{CacheStatsSnapshot, CachedCode, CodeCache, JitTier};
pub use decoder::{QjsBytecodeFunction, StackToRegisterTranslator};
pub use deopt::{DeoptMeta, DeoptPoint};
pub use executable_memory::{CodePool, CodePoolStats, CodeRegion};
pub use jit_bridge::{BytecodeRegistry, JitBridge, JitBridgeStats};
pub use jit_engine::AlbedoJitEngine;
pub use js_value::JsValue;
pub use profiler::{ExecutionStats, FunctionId, JitProfiler, ProfilerConfig};
pub use tier2_compiler::Tier2Compiler;
