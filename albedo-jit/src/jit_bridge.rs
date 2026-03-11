//! # JIT Bridge — A ponte entre QuickJS e AlbedoJIT
//!
//! Orquestra o fluxo: Profiler → Decoder → Compiler → CodeCache.

use std::sync::Arc;
use parking_lot::RwLock;
use hashbrown::HashMap;

use crate::profiler::{FunctionId, JitProfiler};
use crate::jit_engine::{AlbedoJitEngine, JitError};
use crate::decoder::{QjsBytecodeFunction, StackToRegisterTranslator};
use crate::baseline_compiler::BaselineCompiler;
use crate::code_cache::CachedCode;

/// Registro central de bytecodes QuickJS disponíveis para o JIT.
#[derive(Default)]
pub struct BytecodeRegistry {
    entries: RwLock<HashMap<FunctionId, QjsBytecodeFunction>>,
}

impl BytecodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, id: FunctionId, func: QjsBytecodeFunction) {
        let mut entries = self.entries.write();
        entries.insert(id, func);
    }

    pub fn get(&self, id: &FunctionId) -> Option<QjsBytecodeFunction> {
        let entries = self.entries.read();
        entries.get(id).cloned()
    }
}

/// Estatísticas agregadas da Ponte JIT.
#[derive(Debug, Clone, Default)]
pub struct JitBridgeStats {
    pub total_hot_functions_processed: u64,
    pub active_code_count: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

/// Orquestrador do pipeline JIT.
pub struct JitBridge {
    engine: RwLock<AlbedoJitEngine>,
    profiler: Arc<JitProfiler>,
}

impl JitBridge {
    pub fn new(profiler: Arc<JitProfiler>) -> Result<Self, JitError> {
        Ok(Self {
            engine: RwLock::new(AlbedoJitEngine::new()?),
            profiler,
        })
    }

    /// Consulta a fila de hot functions do profiler e compila cada uma.
    pub fn compile_pending(&self, registry: &BytecodeRegistry) {
        let hot_ids = self.profiler.drain_hot_queue();
        
        for id in hot_ids {
            // Se já estiver compilado, ignora
            if self.engine.read().code_cache.lookup(&id).is_some() {
                continue;
            }

            // Busca o bytecode original
            if let Some(qjs_func) = registry.get(&id) {
                println!("[JIT] Compilando hot function: {:?}", id);
                
                // 1. Decodificar Bytecode → AIR
                let translator = StackToRegisterTranslator::new(&qjs_func);
                let (air_func, _map) = translator.translate(qjs_func);

                // 2. Compilar AIR → Native
                let mut engine = self.engine.write();
                let mut compiler = BaselineCompiler::new(&mut engine);
                match compiler.compile(&air_func) {
                    Ok(func_id) => {
                        // 3. Finalizar definições para materializar o código na memória RX
                        if let Err(e) = engine.module.finalize_definitions() {
                            eprintln!("[JIT] Falha ao finalizar definições para {:?}: {}", id, e);
                            continue;
                        }

                        // 4. Obter ponteiro nativo e registrar no cache
                        let native_ptr = engine.module.get_finalized_function(func_id);
                        let entry = CachedCode::new(
                            id.clone(),
                            native_ptr,
                            func_id,
                            128, // Placeholder size
                            crate::code_cache::JitTier::Baseline,
                        );
                        engine.code_cache.insert(entry);

                        println!("[JIT] Sucesso: {:?} compilada e registrada no cache.", id);
                    }
                    Err(e) => {
                        eprintln!("[JIT] Erro ao compilar {:?}: {}", id, e);
                    }
                }
            } else {
                eprintln!("[JIT] Bytecode não encontrado para {:?}, ignorando compilação.", id);
            }
        }
    }

    /// Tenta obter um ponteiro para a versão nativa de uma função.
    pub fn try_native(&self, id: &FunctionId) -> Option<*const u8> {
        self.engine.read().code_cache.lookup(id).map(|entry| {
            entry.increment_execution();
            entry.native_ptr
        })
    }

    /// Retorna snapshot de estatísticas.
    pub fn stats(&self) -> JitBridgeStats {
        let engine = self.engine.read();
        let cache_stats = engine.code_cache.stats();
        JitBridgeStats {
            total_hot_functions_processed: cache_stats.insertions,
            active_code_count: engine.code_cache.len(),
            cache_hits: cache_stats.hits,
            cache_misses: cache_stats.misses,
        }
    }
}

// SAFETY: Ponteiros para o CodeCache são gerenciados de forma thread-safe via Arc/RwLock.
unsafe impl Send for JitBridge {}
unsafe impl Sync for JitBridge {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiler::ProfilerConfig;
    use crate::js_value::JsValue;
    use crate::bytecode::{AirBuilder, AirOpcode, AirTerminator, AirReg};
    use crate::decoder::QjsOpcode;

    #[test]
    fn test_bridge_e2e_flow() {
        // 1. Setup
        let profiler = Arc::new(JitProfiler::new(ProfilerConfig { hot_threshold: 2 }));
        let bridge = JitBridge::new(Arc::clone(&profiler)).unwrap();
        let registry = BytecodeRegistry::new();
        
        let id = FunctionId("test_add".to_string());
        
        // 2. Registrar um bytecode (add 20)
        let qjs_func = QjsBytecodeFunction {
            name: "test_add".into(),
            num_args: 1,
            num_locals: 1,
            opcodes: vec![
                QjsOpcode::GetArg(0),
                QjsOpcode::PushI32(22),
                QjsOpcode::Add,
                QjsOpcode::Return,
            ],
            constant_pool_strings: vec![],
        };
        registry.register(id.clone(), qjs_func);

        // 3. Simular chamadas hot
        profiler.record_call(id.clone());
        assert!(bridge.try_native(&id).is_none()); // Ainda não compilou (faltou 1 call)
        
        profiler.record_call(id.clone()); // Bateu 2!
        
        // 4. Rodar o pipeline da ponte
        bridge.compile_pending(&registry);
        
        // 5. Verificar se agora temos código nativo
        let ptr = bridge.try_native(&id).expect("Deveria ter compilado");
        
        // 6. Executar o código nativo resultante
        // (i64: 20 -> JS(20) + JS(22) = JS(42))
        let func: fn(u64) -> u64 = unsafe { std::mem::transmute(ptr) };
        let result = JsValue(func(JsValue::int32(20).0));
        
        assert_eq!(result.as_int32(), 42);
        assert_eq!(bridge.stats().active_code_count, 1);
    }
}
