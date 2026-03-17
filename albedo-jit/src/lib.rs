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

pub mod bytecode;
pub mod decoder;
pub mod jit_engine;
pub mod profiler;
pub mod js_value;
pub mod runtime_helpers;
pub mod baseline_compiler;
pub mod executable_memory;
pub mod code_cache;
pub mod type_feedback;
pub mod object_model;
pub mod tier2_compiler;
pub mod fast_builtins;
pub mod jit_bridge;
pub mod air_interpreter;
pub mod deopt;
pub mod builtins;

// Re-exports públicos
pub use bytecode::{AirBuilder, AirFunction, AirOpcode};#[cfg(test)]
mod jit_equivalence_tests;
pub use decoder::{QjsBytecodeFunction, StackToRegisterTranslator};
pub use jit_engine::AlbedoJitEngine;
pub use profiler::{FunctionId, JitProfiler, ProfilerConfig, ExecutionStats};
pub use js_value::JsValue;
pub use baseline_compiler::BaselineCompiler;
pub use tier2_compiler::Tier2Compiler;
pub use executable_memory::{CodePool, CodeRegion, CodePoolStats};
pub use code_cache::{CodeCache, CachedCode, CacheStatsSnapshot, JitTier};
pub use jit_bridge::{JitBridge, BytecodeRegistry, JitBridgeStats};
pub use deopt::{DeoptMeta, DeoptPoint};
pub use builtins::{BuiltinId};
