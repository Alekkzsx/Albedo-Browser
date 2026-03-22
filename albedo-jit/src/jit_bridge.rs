//! # JIT Bridge — A ponte entre QuickJS e AlbedoJIT
//!
//! Orquestra o fluxo: Profiler → Decoder → Compiler → CodeCache.

use hashbrown::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::baseline_compiler::BaselineCompiler;
use crate::code_cache::CachedCode;
use crate::decoder::{QjsBytecodeFunction, StackToRegisterTranslator};
use crate::jit_engine::{AlbedoJitEngine, JitError};
use crate::profiler::{FunctionId, JitProfiler};
use cranelift_codegen::ir::{types::I64, AbiParam};
use cranelift_module::Module;

#[derive(Default)]
pub struct BytecodeRegistry {
    entries: RwLock<HashMap<FunctionId, QjsBytecodeFunction>>,
    air_entries: RwLock<HashMap<FunctionId, crate::bytecode::AirFunction>>,
}

impl BytecodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, id: FunctionId, func: QjsBytecodeFunction) {
        let mut entries = self.entries.write();
        entries.insert(id, func);
    }

    pub fn register_air(&self, id: FunctionId, air: crate::bytecode::AirFunction) {
        let mut entries = self.air_entries.write();
        entries.insert(id, air);
    }

    pub fn get(&self, id: &FunctionId) -> Option<QjsBytecodeFunction> {
        let entries = self.entries.read();
        entries.get(id).cloned()
    }

    pub fn get_air(&self, id: &FunctionId) -> Option<crate::bytecode::AirFunction> {
        // Tenta primeiro o cache de AIR (pulo de decodificação)
        if let Some(air) = self.air_entries.read().get(id) {
            return Some(air.clone());
        }

        if let Some(qjs) = self.get(id) {
            let translator = StackToRegisterTranslator::new(&qjs);
            let (air, _) = translator.translate(qjs);
            return Some(air);
        }
        None
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
                eprintln!(
                    "[JIT] Bytecode não encontrado para {:?}, ignorando compilação.",
                    id
                );
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

    /// Tenta obter ou compilar um entry point OSR para um loop.
    pub fn try_osr(
        &self,
        id: &FunctionId,
        pc_offset: u32,
        registry: &BytecodeRegistry,
    ) -> Option<*const u8> {
        // 1. Verificar se já existe no cache (usando uma chave composta p/ OSR)
        let osr_key = FunctionId(format!("{}_osr_{}", id.0, pc_offset));
        if let Some(ptr) = self.try_native(&osr_key) {
            return Some(ptr);
        }

        // 2. Se não existir, tentar compilar via Tier 2 (especializado)
        if let Some(qjs_func) = registry.get(id) {
            println!(
                "[JIT] Compilando OSR Entry p/ {:?} @ PC offset {}",
                id, pc_offset
            );

            // Decodificar Bytecode → AIR (inclui o SourceMap)
            let translator = StackToRegisterTranslator::new(&qjs_func);
            let (air_func, map) = translator.translate(qjs_func);

            // Mapear PC Offset do QuickJS para Bloco/Instrução no AIR
            // Para o protótipo V1, assumimos que loops começam em blocos específicos.
            // Em uma impl real, usaríamos o `map.block_to_qjs_offset`.
            // Mapeamento Robusto: Encontrar o bloco cujo offset inicial é o mais próximo (atrás) do PC atual.
            let target_block_info = map
                .block_to_qjs_offset
                .iter()
                .filter(|(_, &offset)| offset <= pc_offset as usize)
                .max_by_key(|(_, &offset)| offset);

            let (target_block, _block_start_offset) = if let Some((&id, &off)) = target_block_info {
                (id, off)
            } else {
                (0, 0)
            };

            // Dentro do bloco, encontrar a instrução correspondente ao offset exato, se possível.
            let mut entry_inst = 0;
            if let Some(blk) = air_func.blocks.iter().find(|b| b.id.0 == target_block) {
                for (idx, inst) in blk.insts.iter().enumerate() {
                    // Verificamos qual registrador essa instrução define e se ele mapeia pro PC atual.
                    let reg_dst = match inst {
                        crate::bytecode::AirOpcode::LoadInt32 { dst, .. } => Some(dst.0),
                        crate::bytecode::AirOpcode::Add { dst, .. } => Some(dst.0),
                        crate::bytecode::AirOpcode::Sub { dst, .. } => Some(dst.0),
                        crate::bytecode::AirOpcode::Mul { dst, .. } => Some(dst.0),
                        crate::bytecode::AirOpcode::Div { dst, .. } => Some(dst.0),
                        crate::bytecode::AirOpcode::GetProp { dst, .. } => Some(dst.0),
                        crate::bytecode::AirOpcode::LoadFloat64 { dst, ..} => Some(dst.0),
                        crate::bytecode::AirOpcode::LoadBool { dst, ..} => Some(dst.0),
                        crate::bytecode::AirOpcode::LoadString { dst, ..} => Some(dst.0),
                        crate::bytecode::AirOpcode::Move { dst, ..} => Some(dst.0),
                        crate::bytecode::AirOpcode::Eq { dst, ..} => Some(dst.0),
                        crate::bytecode::AirOpcode::StrictEq { dst, ..} => Some(dst.0),
                        crate::bytecode::AirOpcode::Lt { dst, ..} => Some(dst.0),
                        crate::bytecode::AirOpcode::Gt { dst, ..} => Some(dst.0),
                        crate::bytecode::AirOpcode::Lte { dst, ..} => Some(dst.0),
                        crate::bytecode::AirOpcode::Gte { dst, ..} => Some(dst.0),
                        _ => None,
                    };

                    if let Some(r) = reg_dst {
                        if map.reg_to_qjs_offset.get(&r) == Some(&(pc_offset as usize)) {
                            entry_inst = idx;
                            break;
                        }
                    }
                }
            }

            let mut engine = self.engine.write();

            // Construir assinatura: (spill_ptr: i64) -> JsValue(i64)
            let mut sig = engine.module.make_signature();
            sig.params.push(AbiParam::new(I64));
            sig.returns.push(AbiParam::new(I64));

            let mut compiler = crate::tier2_compiler::Tier2Compiler::new(&mut engine);

            // Compilação OSR (Tier 2)
            match compiler.compile_osr(&air_func, target_block, entry_inst) {
                Ok(ptr) => {
                    // Registrar no cache com a chave OSR
                    let func_id = engine.module.declare_anonymous_function(&sig).unwrap();
                    let entry = CachedCode::new(
                        osr_key.clone(),
                        ptr,
                        func_id,
                        256,
                        crate::code_cache::JitTier::AlbedoTurbo, // Tier 2
                    );
                    engine.code_cache.insert(entry);
                    return Some(ptr);
                }
                Err(e) => {
                    eprintln!("[JIT] Falha na compilação OSR (Tier 2): {}", e);
                }
            }
        } else if let Some(air_func) = registry.get_air(id) {
            println!(
                "[JIT] Compilando OSR Entry p/ {:?} a partir do AIR direto (Testing)",
                id
            );
            let target_block = 0;
            let entry_inst = 0;
            
            let mut engine = self.engine.write();
            let mut sig = engine.module.make_signature();
            sig.params.push(AbiParam::new(I64));
            sig.returns.push(AbiParam::new(I64));

            let mut compiler = crate::tier2_compiler::Tier2Compiler::new(&mut engine);
            match compiler.compile_osr(&air_func, target_block, entry_inst) {
                Ok(ptr) => {
                    let func_id = engine.module.declare_anonymous_function(&sig).unwrap();
                    let entry = CachedCode::new(
                        osr_key.clone(),
                        ptr,
                        func_id,
                        256,
                        crate::code_cache::JitTier::AlbedoTurbo,
                    );
                    engine.code_cache.insert(entry);
                    return Some(ptr);
                }
                Err(e) => {
                    eprintln!("[JIT] Falha na compilação OSR a partir do AIR puro: {}", e);
                }
            }
        }

        None
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
    use crate::bytecode::{AirBuilder, AirOpcode, AirReg, AirTerminator};
    use crate::decoder::QjsOpcode;
    use crate::js_value::JsValue;
    use crate::profiler::ProfilerConfig;

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
