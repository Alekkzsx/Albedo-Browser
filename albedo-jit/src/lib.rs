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
pub mod compiler;
pub mod contracts;
pub mod decoder;
pub mod engine;
pub mod infra;
pub mod parking_lot;
pub mod runtime;

// Re-exports públicos para manter compatibilidade
pub use bytecode::{AirBuilder, AirFunction, AirOpcode};
#[cfg(test)]
mod jit_equivalence_tests;

pub use compiler::baseline_compiler::BaselineCompiler;
pub use compiler::code_cache::{CacheStatsSnapshot, CachedCode, CodeCache, JitTier};
pub use compiler::deopt::{DeoptMeta, DeoptPoint};
pub use compiler::escape_analysis::{
    run_field_sensitive, AllocationKind, EscapeAnalysisResult, EscapeReason, EscapeStatus,
    ScalarCandidate, ScalarProperty, StackCandidate,
};
pub use compiler::scalar_replacement::ScalarReplacer;
pub use compiler::stack_allocator::{StackAllocation, StackAllocator};
pub use compiler::tier2_compiler::Tier2Compiler;

pub use runtime::builtins::BuiltinId;
pub use runtime::js_value::JsValue;

pub use engine::jit_bridge::{BytecodeRegistry, JitBridge, JitBridgeStats};
pub use engine::jit_engine::AlbedoJitEngine;
pub use engine::profiler::{ExecutionStats, FunctionId, JitProfiler, ProfilerConfig};

pub use infra::executable_memory::{CodePool, CodePoolStats, CodeRegion};

pub use decoder::{QjsBytecodeFunction, StackToRegisterTranslator};
