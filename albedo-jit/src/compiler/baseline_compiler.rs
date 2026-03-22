//! # Baseline Compiler (Tier 1 JIT)
//!
//! Traduz funções `AirFunction` (Albedo IR) para código de máquina nativo
//! via Cranelift. Neste tier, não há otimizações (além das nativas do LLVM/Cran).
//! O foco é velocidade de compilação.
//!
//! Todas as operações (Add, Sub, Eq) são tratadas usando ponteiros NaN-boxed
//! de 64 bits implícitos através de chamadas C nativas (runtime helpers).

use cranelift_codegen::ir::types::I64;
use cranelift_codegen::ir::{AbiParam, InstBuilder, StackSlotData, StackSlotKind};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::HashMap;

use crate::bytecode::{AirFunction, AirOpcode, AirTerminator};
use crate::engine::jit_engine::{AlbedoJitEngine, JitError};
use crate::runtime::js_value::JsValue;

// Função dummy de fallback enquanto todos helpers não estão implementados
extern "C" fn _js_unimplemented() -> u64 {
    JsValue::undefined().0
}

/// BaselineCompiler do AlbedoJIT (Air -> Native Machine Code usando Cranelift)
pub struct BaselineCompiler<'a> {
    engine: &'a mut AlbedoJitEngine,
    builder_context: FunctionBuilderContext,
}

impl<'a> BaselineCompiler<'a> {
    pub fn new(engine: &'a mut AlbedoJitEngine) -> Self {
        Self {
            engine,
            builder_context: FunctionBuilderContext::new(),
        }
    }

    /// Compila uma AirFunction para código JIT, inserindo-a no módulo atual
    /// Retorna o FuncId da função registrada
    pub fn compile(&mut self, air: &AirFunction) -> Result<FuncId, JitError> {
        // 1. Configurar Assinatura da Função Cranelift
        // No Tier 1, TODOS os parâmetros do JS entram como I64 (NaN-boxed) e retorna I64
        let mut sig = self.engine.module.make_signature();
        for _ in 0..air.num_params {
            sig.params.push(AbiParam::new(I64));
        }
        sig.returns.push(AbiParam::new(I64));

        let func_name = air.name.clone();

        // Declarar função no módulo
        let func_id = self
            .engine
            .module
            .declare_function(&func_name, Linkage::Export, &sig)?;

        // 2. Criar contexto de função do Cranelift
        let mut ctx = self.engine.module.make_context();
        ctx.func.signature = sig;

        // 3. Builder
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.builder_context);

        // Declarar todas as variáveis Cranelift (1:1 com os AirRegs)
        let mut vars = Vec::with_capacity(air.registers_count as usize);
        for _ in 0..air.registers_count {
            let var = builder.declare_var(I64);
            vars.push(var);
        }

        let mut ext_funcs = HashMap::new();
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_add_ic",
            3,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_sub",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_mul",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_strict_eq",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_eq",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_lt",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_to_bool",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_get_prop_ic",
            3,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_call_ic",
            4,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_create_obj",
            0,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_create_array",
            0,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_set_prop",
            3,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_unimplemented",
            0,
            &mut ext_funcs,
        )?;

        // Declarar todos os blocos antecipadamente para satisfazer Jumps forward
        let mut block_map = HashMap::new();
        for air_block in &air.blocks {
            let cl_block = builder.create_block();
            block_map.insert(air_block.id, cl_block);
        }

        // 4. Emitir instruções (Basic Blocks do AIR)
        for (i, air_block) in air.blocks.iter().enumerate() {
            let cl_block = block_map[&air_block.id];

            // Appends para este bloco
            builder.switch_to_block(cl_block);

            // Apenas no bloco 0 (Entry): mapear params do JS pros Regs Cranelift
            if i == 0 {
                builder.append_block_params_for_function_params(cl_block);
                for p in 0..air.num_params {
                    let val = builder.block_params(cl_block)[p as usize];
                    let var = vars[p as usize];
                    builder.def_var(var, val);
                }
            }

            // Converter instruções
            for inst in &air_block.insts {
                match inst {
                    AirOpcode::LoadInt32 { dst, value } => {
                        let packed = JsValue::int32(*value).0 as i64;
                        let val = builder.ins().iconst(I64, packed);
                        builder.def_var(vars[dst.0 as usize], val);
                    }
                    AirOpcode::LoadFloat64 { dst, value } => {
                        let packed = JsValue::float64(*value).0 as i64;
                        let val = builder.ins().iconst(I64, packed);
                        builder.def_var(vars[dst.0 as usize], val);
                    }
                    AirOpcode::LoadNull { dst } => {
                        let packed = JsValue::null().0 as i64;
                        let val = builder.ins().iconst(I64, packed);
                        builder.def_var(vars[dst.0 as usize], val);
                    }
                    AirOpcode::LoadString { dst, str_id } => {
                        let packed = JsValue::string(*str_id as u64).0 as i64;
                        let val = builder.ins().iconst(I64, packed);
                        builder.def_var(vars[dst.0 as usize], val);
                    }
                    AirOpcode::LoadBool { dst, value } => {
                        let packed = JsValue::bool(*value).0 as i64;
                        let val = builder.ins().iconst(I64, packed);
                        builder.def_var(vars[dst.0 as usize], val);
                    }
                    AirOpcode::LoadUndefined { dst } => {
                        let packed = JsValue::undefined().0 as i64;
                        let val = builder.ins().iconst(I64, packed);
                        builder.def_var(vars[dst.0 as usize], val);
                    }
                    AirOpcode::Move { dst, src } => {
                        let val = builder.use_var(vars[src.0 as usize]);
                        builder.def_var(vars[dst.0 as usize], val);
                    }
                    AirOpcode::Add {
                        dst,
                        lhs,
                        rhs,
                        ic_slot,
                    } => {
                        let a = builder.use_var(vars[lhs.0 as usize]);
                        let b = builder.use_var(vars[rhs.0 as usize]);
                        let slot_val = builder.ins().iconst(I64, *ic_slot as i64);
                        let func_ref = *ext_funcs.get("js_add_ic").unwrap();
                        let call = builder.ins().call(func_ref, &[a, b, slot_val]);
                        let res = builder.inst_results(call)[0];
                        builder.def_var(vars[dst.0 as usize], res);
                    }
                    AirOpcode::Sub { dst, lhs, rhs } => {
                        let a = builder.use_var(vars[lhs.0 as usize]);
                        let b = builder.use_var(vars[rhs.0 as usize]);
                        let func_ref = *ext_funcs.get("js_sub").unwrap();
                        let call = builder.ins().call(func_ref, &[a, b]);
                        let res = builder.inst_results(call)[0];
                        builder.def_var(vars[dst.0 as usize], res);
                    }
                    AirOpcode::Mul { dst, lhs, rhs } => {
                        let a = builder.use_var(vars[lhs.0 as usize]);
                        let b = builder.use_var(vars[rhs.0 as usize]);
                        let func_ref = *ext_funcs.get("js_mul").unwrap();
                        let call = builder.ins().call(func_ref, &[a, b]);
                        let res = builder.inst_results(call)[0];
                        builder.def_var(vars[dst.0 as usize], res);
                    }
                    AirOpcode::StrictEq { dst, lhs, rhs } => {
                        let a = builder.use_var(vars[lhs.0 as usize]);
                        let b = builder.use_var(vars[rhs.0 as usize]);
                        let func_ref = *ext_funcs.get("js_strict_eq").unwrap();
                        let call = builder.ins().call(func_ref, &[a, b]);
                        let res = builder.inst_results(call)[0];
                        builder.def_var(vars[dst.0 as usize], res);
                    }
                    AirOpcode::Eq { dst, lhs, rhs } => {
                        let a = builder.use_var(vars[lhs.0 as usize]);
                        let b = builder.use_var(vars[rhs.0 as usize]);
                        let func_ref = *ext_funcs.get("js_eq").unwrap();
                        let call = builder.ins().call(func_ref, &[a, b]);
                        let res = builder.inst_results(call)[0];
                        builder.def_var(vars[dst.0 as usize], res);
                    }
                    AirOpcode::Lt { dst, lhs, rhs } => {
                        let a = builder.use_var(vars[lhs.0 as usize]);
                        let b = builder.use_var(vars[rhs.0 as usize]);
                        let func_ref = *ext_funcs.get("js_lt").unwrap();
                        let call = builder.ins().call(func_ref, &[a, b]);
                        let res = builder.inst_results(call)[0];
                        builder.def_var(vars[dst.0 as usize], res);
                    }
                    AirOpcode::GetProp {
                        dst,
                        obj,
                        prop,
                        ic_slot,
                    } => {
                        let o = builder.use_var(vars[obj.0 as usize]);
                        let p = builder.use_var(vars[prop.0 as usize]);
                        let slot = builder.ins().iconst(I64, *ic_slot as i64);
                        let func_ref = *ext_funcs.get("js_get_prop_ic").unwrap();
                        let call = builder.ins().call(func_ref, &[o, p, slot]);
                        let res = builder.inst_results(call)[0];
                        builder.def_var(vars[dst.0 as usize], res);
                    }
                    AirOpcode::SetProp { obj, prop, value } => {
                        let o = builder.use_var(vars[obj.0 as usize]);
                        let p = builder.use_var(vars[prop.0 as usize]);
                        let v = builder.use_var(vars[value.0 as usize]);
                        let func_ref = *ext_funcs.get("js_set_prop").unwrap();
                        let _call = builder.ins().call(func_ref, &[o, p, v]);
                    }
                    AirOpcode::Call {
                        dst,
                        func,
                        arg_start,
                        num_args,
                        ic_slot,
                    } => {
                        let f = builder.use_var(vars[func.0 as usize]);
                        let args_ptr = if *num_args == 0 {
                            builder.ins().iconst(I64, 0)
                        } else {
                            let size = (*num_args as u32) * 8;
                            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                                StackSlotKind::ExplicitSlot,
                                size,
                                8,
                            ));
                            for i in 0..*num_args {
                                let reg = crate::bytecode::AirReg(arg_start.0 + i);
                                let val = builder.use_var(vars[reg.0 as usize]);
                                let offset = (i * 8) as i32;
                                builder.ins().stack_store(val, slot, offset);
                            }
                            builder.ins().stack_addr(I64, slot, 0)
                        };
                        let num_args_val = builder.ins().iconst(I64, *num_args as i64);
                        let slot_val = builder.ins().iconst(I64, *ic_slot as i64);
                        let func_ref = *ext_funcs.get("js_call_ic").unwrap();
                        let call = builder
                            .ins()
                            .call(func_ref, &[f, args_ptr, num_args_val, slot_val]);
                        let res = builder.inst_results(call)[0];
                        builder.def_var(vars[dst.0 as usize], res);
                    }
                    AirOpcode::CreateObj { dst } => {
                        let func_ref = *ext_funcs.get("js_create_obj").unwrap();
                        let call = builder.ins().call(func_ref, &[]);
                        let res = builder.inst_results(call)[0];
                        builder.def_var(vars[dst.0 as usize], res);
                    }
                    AirOpcode::CreateArray { dst } => {
                        let func_ref = *ext_funcs.get("js_create_array").unwrap();
                        let call = builder.ins().call(func_ref, &[]);
                        let res = builder.inst_results(call)[0];
                        builder.def_var(vars[dst.0 as usize], res);
                    }
                    _ => {
                        // Opcodes complexos (propriedades, closures) pendentes da spec C
                        let func_ref = *ext_funcs.get("js_unimplemented").unwrap();
                        let call = builder.ins().call(func_ref, &[]);
                        let _res = builder.inst_results(call)[0];

                        // Fallback maroto (usa reg destination padrao se der falha match, senao da compile error)
                        // Descobrir qual eh a "dst" property no Enum requer matching, entao usaremos log warning.
                        println!("[JIT Compiler] Warning: Unsupported Opcode {:?} - fallback to undefined", inst);
                    }
                }
            }

            // Converter Control-Flow (Terminator)
            if let Some(term) = &air_block.terminator {
                match term {
                    AirTerminator::Return(reg) => {
                        let rval = builder.use_var(vars[reg.0 as usize]);
                        builder.ins().return_(&[rval]);
                    }
                    AirTerminator::Jump(target) => {
                        let cl_target = block_map[target];
                        builder.ins().jump(cl_target, &[]);
                    }
                    AirTerminator::JumpIf {
                        cond,
                        then_blk,
                        else_blk,
                    } => {
                        let cl_then = block_map[then_blk];
                        let cl_else = block_map[else_blk];

                        // Convertermos o valor arbitrario p/ bool puro via C runtime
                        let cv = builder.use_var(vars[cond.0 as usize]);
                        let f_to_bool = *ext_funcs.get("js_to_bool").unwrap();
                        let call = builder.ins().call(f_to_bool, &[cv]);
                        let boxed_bool = builder.inst_results(call)[0];

                        // Mask pra pegar o bit isolado
                        let b_val = builder.ins().band_imm(boxed_bool, 1);
                        // Jump if Not Zero
                        builder.ins().brif(b_val, cl_then, &[], cl_else, &[]);
                    }
                }
            }
        }

        // Selar todos os blocos após emitir todos os saltos
        for cl_block in block_map.values() {
            builder.seal_block(*cl_block);
        }

        builder.finalize();

        // 5. Compilar código para o Module Memory
        self.engine
            .module
            .define_function(func_id, &mut ctx)
            .map_err(|e| JitError::Compilation(e.to_string()))?;

        self.engine.module.clear_context(&mut ctx);

        // O flush pra .text (Executable RX Memory) é feito no commit() do jit_engine raiz

        Ok(func_id)
    }

    /// Importa uma função "extern C" para uso dentro dos módulos I64.
    fn declare_runtime_helper(
        module: &mut cranelift_jit::JITModule,
        builder: &mut FunctionBuilder,
        name: &str,
        arity: usize,
        ext_funcs: &mut HashMap<String, cranelift_codegen::ir::FuncRef>,
    ) -> Result<(), JitError> {
        let mut sig = module.make_signature();
        for _ in 0..arity {
            sig.params.push(AbiParam::new(I64));
        }
        sig.returns.push(AbiParam::new(I64));

        let func_id = module.declare_function(name, Linkage::Import, &sig)?;

        let local_ref = module.declare_func_in_func(func_id, builder.func);
        ext_funcs.insert(name.to_string(), local_ref);

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Testes de Integração End-to-End
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{AirBuilder, AirOpcode, AirReg, AirTerminator};

    #[test]
    fn test_compile_simple_add() {
        let mut engine = AlbedoJitEngine::new().unwrap();

        // Fake builder parameters
        let mut b = AirBuilder::new("test_add".to_string(), 2, 0);

        // Bloco main que extrai P0, P1, Adiciona e Retorna
        let pr0 = b.param(0);
        let pr1 = b.param(1);

        let sum_reg = b.emit_add(pr0, pr1);
        b.emit_return(sum_reg);

        let air = b.build();

        // Compilação
        let mut compiler = BaselineCompiler::new(&mut engine);
        let id = compiler.compile(&air).expect("Compilação falhou");

        // Materializa Memória RX
        engine.module.finalize_definitions().unwrap();
        let ptr = engine.module.get_finalized_function(id);

        let func: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };

        // Teste de Execução Nativa
        let v1 = JsValue::int32(100).0;
        let v2 = JsValue::int32(200).0;
        let p_res = func(v1, v2);

        assert!(JsValue(p_res).is_int32());
        assert_eq!(JsValue(p_res).as_int32(), 300);
    }

    #[test]
    fn test_compile_branch() {
        let mut engine = AlbedoJitEngine::new().unwrap();

        let mut b = AirBuilder::new("test_branch".to_string(), 1, 0);
        let pr0 = b.param(0); // condicao

        let blk_then = b.create_block();
        let blk_else = b.create_block();

        // Entry block
        b.emit_jump_if(pr0, blk_then, blk_else);

        // Then (retorna int 10)
        b.switch_block(blk_then);
        let dst_10 = b.emit_load_int32(10);
        b.emit_return(dst_10);

        // Else (retorna int 20)
        b.switch_block(blk_else);
        let dst_20 = b.emit_load_int32(20);
        b.emit_return(dst_20);

        let air = b.build();

        let mut compiler = BaselineCompiler::new(&mut engine);
        let id = compiler.compile(&air).expect("Falhou no compile do branch");

        engine.module.finalize_definitions().unwrap();
        let ptr = engine.module.get_finalized_function(id);

        let func: extern "C" fn(u64) -> u64 = unsafe { std::mem::transmute(ptr) };

        // Valores JS na C-ABI
        let js_true = JsValue::bool(true).0;
        let js_false = JsValue::bool(false).0;
        // Float 0 no JS é false no JumpIf
        let js_zero = JsValue::float64(0.0).0;

        // Validando fluxo lógico Nativo
        assert_eq!(JsValue(func(js_true)).as_int32(), 10);
        assert_eq!(JsValue(func(js_false)).as_int32(), 20);
        assert_eq!(JsValue(func(js_zero)).as_int32(), 20); // Pula pro else
    }
}
