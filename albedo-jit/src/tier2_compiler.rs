//! # Tier 2 Compiler (Type Specialization)
//!
//! Usa type feedback coletado via Inline Caches para gerar código especializado
//! (IADD/FADD e acesso direto por offset de propriedade).

use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::types::{F64, I32, I64};
use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlags, StackSlot, StackSlotData, StackSlotKind,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::HashMap;
use std::sync::Arc;

use crate::bytecode::{AirBlockId, AirFunction, AirOpcode, AirTerminator};
use crate::deopt::{register_meta, DeoptMeta, DeoptPoint};
use crate::jit_engine::{AlbedoJitEngine, JitError};
use crate::js_value::{JsValue, FLOAT_NAN, PAYLOAD_MASK, TAG_INT32, TAG_MASK, TAG_MIN};
use crate::object_model::{JSOBJ_PROPS_OFFSET, JSOBJ_SHAPE_OFFSET};
use crate::type_feedback::{
    AddFeedbackSnapshot, GetPropFeedbackSnapshot, IcState, TypeFeedbackRegistry, TypePair,
    ValueType,
};

const MIN_FEEDBACK_SAMPLES: u64 = 1;

pub struct Tier2Compiler<'a> {
    engine: &'a mut AlbedoJitEngine,
    builder_context: FunctionBuilderContext,
}

impl<'a> Tier2Compiler<'a> {
    pub fn new(engine: &'a mut AlbedoJitEngine) -> Self {
        Self {
            engine,
            builder_context: FunctionBuilderContext::new(),
        }
    }

    pub fn compile(&mut self, air: &AirFunction) -> Result<FuncId, JitError> {
        let mut sig = self.engine.module.make_signature();
        for _ in 0..air.num_params {
            sig.params.push(AbiParam::new(I64));
        }
        sig.returns.push(AbiParam::new(I64));

        let func_name = format!("{}_tier2", air.name);
        let func_id = self
            .engine
            .module
            .declare_function(&func_name, Linkage::Export, &sig)?;

        // Pré-mapeamento de deopt points (1 por instrução AIR)
        let (deopt_points, deopt_map) = Self::build_deopt_points(air);
        let meta_id = register_meta(DeoptMeta {
            air: Arc::new(air.clone()),
            points: deopt_points,
            regs_count: air.registers_count,
            spill_stride: 8,
            spill_size: std::cmp::max(1, air.registers_count) * 8,
        });

        let mut ctx = self.engine.module.make_context();
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.builder_context);

        let mut vars = Vec::with_capacity(air.registers_count as usize);
        for _ in 0..air.registers_count {
            let var = builder.declare_var(I64);
            vars.push(var);
        }

        let spill_size = std::cmp::max(1, air.registers_count) * 8;
        let spill_slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            spill_size,
            8,
        ));

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
            "js_deopt_bailout",
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
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_div",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_mod",
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
            "js_gt",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_gte",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_lte",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_bit_and",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_bit_or",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_bit_xor",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_bit_shl",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_bit_shr",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_bit_ushr",
            2,
            &mut ext_funcs,
        )?;

        // Fast Builtins symbols
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_floor",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_ceil",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_abs",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_sqrt",
            1,
            &mut ext_funcs,
        )?;

        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_array_push",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_array_pop",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_string_char_at",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_json_parse",
            1,
            &mut ext_funcs,
        )?;

        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_has_prop",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_delete_prop",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_type_of",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_instance_of",
            2,
            &mut ext_funcs,
        )?;

        let mut block_map = HashMap::new();
        for air_block in &air.blocks {
            let cl_block = builder.create_block();
            block_map.insert(air_block.id, cl_block);
        }

        for (i, air_block) in air.blocks.iter().enumerate() {
            let cl_block = block_map[&air_block.id];
            builder.switch_to_block(cl_block);

            if i == 0 {
                builder.append_block_params_for_function_params(cl_block);
                for p in 0..air.num_params {
                    let val = builder.block_params(cl_block)[p as usize];
                    let var = vars[p as usize];
                    builder.def_var(var, val);
                }
                let undef = builder.ins().iconst(I64, JsValue::undefined().0 as i64);
                for r in air.num_params..air.registers_count {
                    let var = vars[r as usize];
                    builder.def_var(var, undef);
                }
            }

            for (inst_index, inst) in air_block.insts.iter().enumerate() {
                let deopt_id = *deopt_map.get(&(air_block.id.0, inst_index)).unwrap_or(&0);
                if Self::emit_instruction(
                    &mut builder,
                    inst,
                    &vars,
                    &ext_funcs,
                    meta_id,
                    deopt_id,
                    spill_slot,
                ) {
                    continue;
                }
            }

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
                        let cv = builder.use_var(vars[cond.0 as usize]);
                        let f_to_bool = *ext_funcs.get("js_to_bool").unwrap();
                        let call = builder.ins().call(f_to_bool, &[cv]);
                        let boxed_bool = builder.inst_results(call)[0];
                        let b_val = builder.ins().band_imm(boxed_bool, 1);
                        builder.ins().brif(b_val, cl_then, &[], cl_else, &[]);
                    }
                }
            }
        }

        builder.seal_all_blocks();
        builder.finalize();

        self.engine
            .module
            .define_function(func_id, &mut ctx)
            .map_err(|e| JitError::Compilation(e.to_string()))?;

        self.engine.module.clear_context(&mut ctx);
        Ok(func_id)
    }

    /// Compila um entry OSR que retoma a execuÃ§Ã£o a partir de um bloco/inst.
    /// Assinatura: (spill_ptr: i64) -> i64
    pub fn compile_osr(
        &mut self,
        air: &AirFunction,
        entry_block_id: u32,
        entry_inst: usize,
    ) -> Result<*const u8, JitError> {
        let mut sig = self.engine.module.make_signature();
        sig.params.push(AbiParam::new(I64)); // spill_ptr
        sig.returns.push(AbiParam::new(I64));

        let func_name = format!("{}_osr_b{}_i{}", air.name, entry_block_id, entry_inst);
        let func_id = self
            .engine
            .module
            .declare_function(&func_name, Linkage::Export, &sig)?;

        let (deopt_points, deopt_map) = Self::build_deopt_points(air);
        let meta_id = register_meta(DeoptMeta {
            air: Arc::new(air.clone()),
            points: deopt_points,
            regs_count: air.registers_count,
            spill_stride: 8,
            spill_size: std::cmp::max(1, air.registers_count) * 8,
        });

        let mut ctx = self.engine.module.make_context();
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.builder_context);

        let mut vars = Vec::with_capacity(air.registers_count as usize);
        for _ in 0..air.registers_count {
            let var = builder.declare_var(I64);
            vars.push(var);
        }

        let spill_size = std::cmp::max(1, air.registers_count) * 8;
        let spill_slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            spill_size,
            8,
        ));

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
            "js_gt",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_lte",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_gte",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_div",
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
            "js_deopt_bailout",
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

        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_has_prop",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_delete_prop",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "js_instance_of",
            2,
            &mut ext_funcs,
        )?;

        let mut block_map = HashMap::new();
        for air_block in &air.blocks {
            let cl_block = builder.create_block();
            block_map.insert(air_block.id, cl_block);
        }

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        let spill_ptr = builder.block_params(entry_block)[0];

        builder.switch_to_block(entry_block);
        for r in 0..air.registers_count {
            let offset = (r * 8) as i32;
            let val = builder.ins().load(I64, MemFlags::new(), spill_ptr, offset);
            builder.def_var(vars[r as usize], val);
        }
        let target = block_map
            .get(&AirBlockId(entry_block_id))
            .copied()
            .unwrap_or_else(|| block_map[&air.blocks[0].id]);
        builder.ins().jump(target, &[]);
        builder.seal_block(entry_block);

        for air_block in &air.blocks {
            let cl_block = block_map[&air_block.id];
            builder.switch_to_block(cl_block);

            let start = if air_block.id.0 == entry_block_id {
                entry_inst
            } else {
                0
            };
            for (inst_index, inst) in air_block.insts.iter().enumerate() {
                if inst_index < start {
                    continue;
                }
                let deopt_id = *deopt_map.get(&(air_block.id.0, inst_index)).unwrap_or(&0);
                if Self::emit_instruction(
                    &mut builder,
                    inst,
                    &vars,
                    &ext_funcs,
                    meta_id,
                    deopt_id,
                    spill_slot,
                ) {
                    continue;
                }
            }

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
                        let cv = builder.use_var(vars[cond.0 as usize]);
                        let f_to_bool = *ext_funcs.get("js_to_bool").unwrap();
                        let call = builder.ins().call(f_to_bool, &[cv]);
                        let boxed_bool = builder.inst_results(call)[0];
                        let b_val = builder.ins().band_imm(boxed_bool, 1);
                        builder.ins().brif(b_val, cl_then, &[], cl_else, &[]);
                    }
                }
            }
        }

        builder.seal_all_blocks();
        builder.finalize();

        self.engine
            .module
            .define_function(func_id, &mut ctx)
            .map_err(|e| JitError::Compilation(e.to_string()))?;

        self.engine.module.clear_context(&mut ctx);
        self.engine
            .module
            .finalize_definitions()
            .map_err(|e| JitError::Compilation(e.to_string()))?;

        Ok(self.engine.module.get_finalized_function(func_id))
    }

    fn emit_specialized_add(
        builder: &mut FunctionBuilder,
        ext_funcs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
        a: cranelift_codegen::ir::Value,
        b: cranelift_codegen::ir::Value,
        slot: u32,
        snap: &AddFeedbackSnapshot,
        meta_id: u32,
        deopt_id: u32,
        spill_slot: StackSlot,
        vars: &[cranelift_frontend::Variable],
    ) -> Option<cranelift_codegen::ir::Value> {
        if snap.state != IcState::Monomorphic || snap.total < MIN_FEEDBACK_SAMPLES {
            return None;
        }
        let pair = snap.monomorphic?;
        match pair {
            TypePair(ValueType::Int32, ValueType::Int32) => Some(Self::emit_add_int32(
                builder, ext_funcs, a, b, slot, meta_id, deopt_id, spill_slot, vars,
            )),
            TypePair(ValueType::Float64, ValueType::Float64) => Some(Self::emit_add_float64(
                builder, ext_funcs, a, b, slot, meta_id, deopt_id, spill_slot, vars,
            )),
            _ => None,
        }
    }

    fn emit_add_int32(
        builder: &mut FunctionBuilder,
        ext_funcs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
        a: cranelift_codegen::ir::Value,
        b: cranelift_codegen::ir::Value,
        _slot: u32,
        meta_id: u32,
        deopt_id: u32,
        spill_slot: StackSlot,
        vars: &[cranelift_frontend::Variable],
    ) -> cranelift_codegen::ir::Value {
        let fast_block = builder.create_block();
        let fast_ok_block = builder.create_block();
        let slow_block = builder.create_block();
        let cont_block = builder.create_block();
        builder.append_block_param(cont_block, I64);

        let tag_mask = builder.ins().iconst(I64, TAG_MASK as i64);
        let tag_int = builder.ins().iconst(I64, TAG_INT32 as i64);
        let a_tag = builder.ins().band(a, tag_mask);
        let b_tag = builder.ins().band(b, tag_mask);
        let a_ok = builder.ins().icmp(IntCC::Equal, a_tag, tag_int);
        let b_ok = builder.ins().icmp(IntCC::Equal, b_tag, tag_int);
        let both_ok = builder.ins().band(a_ok, b_ok);
        builder
            .ins()
            .brif(both_ok, fast_block, &[], slow_block, &[]);

        builder.switch_to_block(fast_block);
        let a_payload = builder.ins().band_imm(a, 0xFFFF_FFFF);
        let b_payload = builder.ins().band_imm(b, 0xFFFF_FFFF);
        let a_i32 = builder.ins().ireduce(I32, a_payload);
        let b_i32 = builder.ins().ireduce(I32, b_payload);
        let (sum, overflow) = builder.ins().sadd_overflow(a_i32, b_i32);
        let overflowed = builder.ins().icmp_imm(IntCC::NotEqual, overflow, 0);
        builder
            .ins()
            .brif(overflowed, slow_block, &[], fast_ok_block, &[]);

        builder.switch_to_block(fast_ok_block);
        let sum_i64 = builder.ins().uextend(I64, sum);
        let boxed = builder.ins().bor(sum_i64, tag_int);
        let cont_args = [boxed.into()];
        builder.ins().jump(cont_block, &cont_args);

        builder.switch_to_block(slow_block);
        let res = Self::emit_deopt_call(builder, ext_funcs, meta_id, deopt_id, spill_slot, vars);
        builder.ins().return_(&[res]);

        builder.switch_to_block(cont_block);
        builder.seal_block(fast_block);
        builder.seal_block(fast_ok_block);
        builder.seal_block(slow_block);
        let res = builder.block_params(cont_block)[0];
        res
    }

    fn emit_add_float64(
        builder: &mut FunctionBuilder,
        ext_funcs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
        a: cranelift_codegen::ir::Value,
        b: cranelift_codegen::ir::Value,
        _slot: u32,
        meta_id: u32,
        deopt_id: u32,
        spill_slot: StackSlot,
        vars: &[cranelift_frontend::Variable],
    ) -> cranelift_codegen::ir::Value {
        let fast_block = builder.create_block();
        let slow_block = builder.create_block();
        let cont_block = builder.create_block();
        builder.append_block_param(cont_block, I64);

        let tag_min = builder.ins().iconst(I64, TAG_MIN as i64);
        let a_ok = builder.ins().icmp(IntCC::UnsignedLessThan, a, tag_min);
        let b_ok = builder.ins().icmp(IntCC::UnsignedLessThan, b, tag_min);
        let both_ok = builder.ins().band(a_ok, b_ok);
        builder
            .ins()
            .brif(both_ok, fast_block, &[], slow_block, &[]);

        builder.switch_to_block(fast_block);
        let a_f = builder.ins().bitcast(F64, MemFlags::new(), a);
        let b_f = builder.ins().bitcast(F64, MemFlags::new(), b);
        let sum_f = builder.ins().fadd(a_f, b_f);
        let sum_bits = builder.ins().bitcast(I64, MemFlags::new(), sum_f);
        let is_nan = builder.ins().fcmp(FloatCC::Unordered, sum_f, sum_f);
        let nan_bits = builder.ins().iconst(I64, FLOAT_NAN as i64);
        let boxed = builder.ins().select(is_nan, nan_bits, sum_bits);
        let cont_args = [boxed.into()];
        builder.ins().jump(cont_block, &cont_args);

        builder.switch_to_block(slow_block);
        let res = Self::emit_deopt_call(builder, ext_funcs, meta_id, deopt_id, spill_slot, vars);
        builder.ins().return_(&[res]);

        builder.switch_to_block(cont_block);
        builder.seal_block(fast_block);
        builder.seal_block(slow_block);
        let res = builder.block_params(cont_block)[0];
        res
    }

    fn emit_specialized_get_prop(
        builder: &mut FunctionBuilder,
        ext_funcs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
        obj: cranelift_codegen::ir::Value,
        prop: cranelift_codegen::ir::Value,
        _slot: u32,
        snap: &GetPropFeedbackSnapshot,
        meta_id: u32,
        deopt_id: u32,
        spill_slot: StackSlot,
        vars: &[cranelift_frontend::Variable],
    ) -> Option<cranelift_codegen::ir::Value> {
        if snap.state != IcState::Monomorphic || snap.total < MIN_FEEDBACK_SAMPLES {
            return None;
        }
        let mono = snap.monomorphic?;

        let obj_block = builder.create_block();
        let prop_block = builder.create_block();
        let shape_block = builder.create_block();
        let load_block = builder.create_block();
        let slow_block = builder.create_block();
        let cont_block = builder.create_block();
        builder.append_block_param(cont_block, I64);

        let tag_mask = builder.ins().iconst(I64, TAG_MASK as i64);
        let tag_obj = builder
            .ins()
            .iconst(I64, crate::js_value::TAG_OBJECT as i64);
        let obj_tag = builder.ins().band(obj, tag_mask);
        let is_obj = builder.ins().icmp(IntCC::Equal, obj_tag, tag_obj);
        builder.ins().brif(is_obj, obj_block, &[], slow_block, &[]);

        builder.switch_to_block(obj_block);
        // Guard: prop id match (string id)
        let tag_str = builder
            .ins()
            .iconst(I64, crate::js_value::TAG_STRING as i64);
        let prop_tag = builder.ins().band(prop, tag_mask);
        let is_str = builder.ins().icmp(IntCC::Equal, prop_tag, tag_str);
        builder.ins().brif(is_str, prop_block, &[], slow_block, &[]);

        builder.switch_to_block(prop_block);
        let prop_expected = builder.ins().iconst(I64, mono.prop_id as i64);
        let prop_id = builder.ins().band_imm(prop, PAYLOAD_MASK as i64);
        let prop_ok = builder.ins().icmp(IntCC::Equal, prop_id, prop_expected);
        builder
            .ins()
            .brif(prop_ok, shape_block, &[], slow_block, &[]);

        builder.switch_to_block(shape_block);
        // Object ptr
        let payload_mask = builder.ins().iconst(I64, PAYLOAD_MASK as i64);
        let obj_ptr = builder.ins().band(obj, payload_mask);

        let shape_ptr = builder.ins().iadd_imm(obj_ptr, JSOBJ_SHAPE_OFFSET as i64);
        let shape = builder.ins().load(I64, MemFlags::new(), shape_ptr, 0);
        let expected_shape = builder.ins().iconst(I64, mono.shape_id as i64);
        let shape_ok = builder.ins().icmp(IntCC::Equal, shape, expected_shape);
        builder
            .ins()
            .brif(shape_ok, load_block, &[], slow_block, &[]);

        builder.switch_to_block(load_block);
        let props_ptr =
            builder
                .ins()
                .load(I64, MemFlags::new(), obj_ptr, JSOBJ_PROPS_OFFSET as i32);
        let offset_bytes = (mono.offset as i64) * 8;
        let value_addr = builder.ins().iadd_imm(props_ptr, offset_bytes);
        let value = builder.ins().load(I64, MemFlags::new(), value_addr, 0);
        let cont_args = [value.into()];
        builder.ins().jump(cont_block, &cont_args);

        builder.switch_to_block(slow_block);
        let res = Self::emit_deopt_call(builder, ext_funcs, meta_id, deopt_id, spill_slot, vars);
        builder.ins().return_(&[res]);

        builder.switch_to_block(cont_block);
        builder.seal_block(obj_block);
        builder.seal_block(prop_block);
        builder.seal_block(shape_block);
        builder.seal_block(load_block);
        builder.seal_block(slow_block);
        let res = builder.block_params(cont_block)[0];
        Some(res)
    }

    fn emit_specialized_call(
        builder: &mut FunctionBuilder,
        ext_funcs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
        f: cranelift_codegen::ir::Value,
        arg_start: crate::bytecode::AirReg,
        num_args: u32,
        snap: &crate::type_feedback::CallFeedbackSnapshot,
        meta_id: u32,
        deopt_id: u32,
        spill_slot: StackSlot,
        vars: &[cranelift_frontend::Variable],
    ) -> Option<cranelift_codegen::ir::Value> {
        if snap.state != IcState::Monomorphic || snap.total < MIN_FEEDBACK_SAMPLES {
            return None;
        }
        let callee = snap.monomorphic.expect("Devia ser monomórfico");
        println!(
            "[TIER2-DEBUG] Call especializado para callee={:?} builtin={}",
            callee,
            callee.is_builtin()
        );
        if !callee.is_builtin() {
            return None;
        }

        let bid = callee.as_builtin_id() as u32;
        use crate::builtins::BuiltinId;

        // Guard: callee must match monomorphic value exactly
        let slow_block = builder.create_block();
        let fast_block = builder.create_block();
        let expected_f = builder.ins().iconst(I64, callee.0 as i64);
        let f_ok = builder.ins().icmp(IntCC::Equal, f, expected_f);
        builder.ins().brif(f_ok, fast_block, &[], slow_block, &[]);

        builder.switch_to_block(fast_block);
        let res = match bid {
            x if x == BuiltinId::MathAbs as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                // Inlining puramente via Cranelift (FABS) para Float64
                // Para Int32 usamos a chamada rápida que lida com abs(MIN_INT)
                let func_ref = *ext_funcs.get("fast_math_abs").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathSqrt as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                // Inlining v2: fsqrt direto se for Float64
                let is_f64 = builder
                    .ins()
                    .icmp_imm(IntCC::UnsignedLessThan, arg, TAG_MIN as i64);
                let f_sqrt_block = builder.create_block();
                let f_slow_block = builder.create_block();
                let f_cont_block = builder.create_block();
                builder.append_block_param(f_cont_block, I64);

                builder
                    .ins()
                    .brif(is_f64, f_sqrt_block, &[], f_slow_block, &[]);

                builder.switch_to_block(f_sqrt_block);
                let f_val = builder.ins().bitcast(F64, MemFlags::new(), arg);
                let f_res = builder.ins().sqrt(f_val);
                let res_bits = builder.ins().bitcast(I64, MemFlags::new(), f_res);
                let jump_args = [res_bits.into()];
                builder.ins().jump(f_cont_block, &jump_args);

                builder.switch_to_block(f_slow_block);
                let func_ref = *ext_funcs.get("fast_math_sqrt").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                let res_call = builder.inst_results(call)[0];
                let jump_args = [res_call.into()];
                builder.ins().jump(f_cont_block, &jump_args);

                builder.switch_to_block(f_cont_block);
                builder.seal_block(f_sqrt_block);
                builder.seal_block(f_slow_block);
                Some(builder.block_params(f_cont_block)[0])
            }
            x if x == BuiltinId::MathFloor as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_floor").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathCeil as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_ceil").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::ArrayPush as u32 && num_args >= 1 => {
                let arr = builder.use_var(vars[arg_start.0 as usize]);
                let val = if num_args >= 2 {
                    builder.use_var(vars[arg_start.0 as usize + 1])
                } else {
                    builder.ins().iconst(I64, JsValue::undefined().0 as i64)
                };
                let func_ref = *ext_funcs.get("fast_array_push").unwrap();
                let call = builder.ins().call(func_ref, &[arr, val]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::ArrayPop as u32 && num_args >= 1 => {
                let arr = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_array_pop").unwrap();
                let call = builder.ins().call(func_ref, &[arr]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::StringCharAt as u32 && num_args >= 1 => {
                let s = builder.use_var(vars[arg_start.0 as usize]);
                let idx = if num_args >= 2 {
                    builder.use_var(vars[arg_start.0 as usize + 1])
                } else {
                    builder.ins().iconst(I64, JsValue::int32(0).0 as i64)
                };
                let func_ref = *ext_funcs.get("fast_string_char_at").unwrap();
                let call = builder.ins().call(func_ref, &[s, idx]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::JsonParse as u32 && num_args >= 1 => {
                let s = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_json_parse").unwrap();
                let call = builder.ins().call(func_ref, &[s]);
                Some(builder.inst_results(call)[0])
            }
            _ => None,
        };

        if let Some(result) = res {
            let next_block = builder.create_block();
            builder.ins().jump(next_block, &[]);

            builder.switch_to_block(slow_block);
            let deopt_res =
                Self::emit_deopt_call(builder, ext_funcs, meta_id, deopt_id, spill_slot, vars);
            builder.ins().return_(&[deopt_res]);

            builder.switch_to_block(next_block);
            builder.seal_block(fast_block);
            builder.seal_block(slow_block);
            builder.seal_block(next_block);
            Some(result)
        } else {
            // Se não conseguimos especializar esse builtin específico, voltamos ao normal
            // Mas precisamos fechar o bloco aberto pelo guard
            builder.ins().jump(slow_block, &[]);
            builder.switch_to_block(slow_block);
            builder.seal_block(fast_block);
            builder.seal_block(slow_block);
            None
        }
    }

    fn spill_all(
        builder: &mut FunctionBuilder,
        spill_slot: StackSlot,
        vars: &[cranelift_frontend::Variable],
    ) {
        for (i, var) in vars.iter().enumerate() {
            let val = builder.use_var(*var);
            let offset = (i * 8) as i32;
            builder.ins().stack_store(val, spill_slot, offset);
        }
    }

    fn emit_deopt_call(
        builder: &mut FunctionBuilder,
        ext_funcs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
        meta_id: u32,
        deopt_id: u32,
        spill_slot: StackSlot,
        vars: &[cranelift_frontend::Variable],
    ) -> cranelift_codegen::ir::Value {
        Self::spill_all(builder, spill_slot, vars);
        let meta_val = builder.ins().iconst(I64, meta_id as i64);
        let deopt_val = builder.ins().iconst(I64, deopt_id as i64);
        let spill_ptr = builder.ins().stack_addr(I64, spill_slot, 0);
        let func_ref = *ext_funcs.get("js_deopt_bailout").unwrap();
        let call = builder
            .ins()
            .call(func_ref, &[meta_val, deopt_val, spill_ptr]);
        builder.inst_results(call)[0]
    }

    /// ISUB (int32) com guard de tipo e overflow.
    fn emit_sub_int32(
        builder: &mut FunctionBuilder,
        ext_funcs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
        a: cranelift_codegen::ir::Value,
        b: cranelift_codegen::ir::Value,
        meta_id: u32,
        deopt_id: u32,
        spill_slot: StackSlot,
        vars: &[cranelift_frontend::Variable],
    ) -> Option<cranelift_codegen::ir::Value> {
        let fast_block = builder.create_block();
        let fast_ok_block = builder.create_block();
        let slow_block = builder.create_block();
        let cont_block = builder.create_block();
        builder.append_block_param(cont_block, I64);

        let tag_mask = builder.ins().iconst(I64, TAG_MASK as i64);
        let tag_int = builder.ins().iconst(I64, TAG_INT32 as i64);
        let a_tag = builder.ins().band(a, tag_mask);
        let b_tag = builder.ins().band(b, tag_mask);
        let a_ok = builder.ins().icmp(IntCC::Equal, a_tag, tag_int);
        let b_ok = builder.ins().icmp(IntCC::Equal, b_tag, tag_int);
        let both_ok = builder.ins().band(a_ok, b_ok);
        builder
            .ins()
            .brif(both_ok, fast_block, &[], slow_block, &[]);

        builder.switch_to_block(fast_block);
        let a_i32 = builder.ins().ireduce(I32, a);
        let b_i32 = builder.ins().ireduce(I32, b);
        let (diff, overflow) = builder.ins().ssub_overflow(a_i32, b_i32);
        let overflowed = builder.ins().icmp_imm(IntCC::NotEqual, overflow, 0);
        builder
            .ins()
            .brif(overflowed, slow_block, &[], fast_ok_block, &[]);

        builder.switch_to_block(fast_ok_block);
        let diff_i64 = builder.ins().uextend(I64, diff);
        let boxed = builder.ins().bor(diff_i64, tag_int);
        let cont_args = [boxed.into()];
        builder.ins().jump(cont_block, &cont_args);

        builder.switch_to_block(slow_block);
        let res = Self::emit_deopt_call(builder, ext_funcs, meta_id, deopt_id, spill_slot, vars);
        builder.ins().return_(&[res]);

        builder.switch_to_block(cont_block);
        builder.seal_block(fast_block);
        builder.seal_block(fast_ok_block);
        builder.seal_block(slow_block);
        builder.seal_block(cont_block);
        Some(builder.block_params(cont_block)[0])
    }

    /// IMUL (int32) com guard de tipo e overflow.
    fn emit_mul_int32(
        builder: &mut FunctionBuilder,
        ext_funcs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
        a: cranelift_codegen::ir::Value,
        b: cranelift_codegen::ir::Value,
        meta_id: u32,
        deopt_id: u32,
        spill_slot: StackSlot,
        vars: &[cranelift_frontend::Variable],
    ) -> Option<cranelift_codegen::ir::Value> {
        let fast_block = builder.create_block();
        let fast_ok_block = builder.create_block();
        let slow_block = builder.create_block();
        let cont_block = builder.create_block();
        builder.append_block_param(cont_block, I64);

        let tag_mask = builder.ins().iconst(I64, TAG_MASK as i64);
        let tag_int = builder.ins().iconst(I64, TAG_INT32 as i64);
        let a_tag = builder.ins().band(a, tag_mask);
        let b_tag = builder.ins().band(b, tag_mask);
        let a_ok = builder.ins().icmp(IntCC::Equal, a_tag, tag_int);
        let b_ok = builder.ins().icmp(IntCC::Equal, b_tag, tag_int);
        let both_ok = builder.ins().band(a_ok, b_ok);
        builder
            .ins()
            .brif(both_ok, fast_block, &[], slow_block, &[]);

        builder.switch_to_block(fast_block);
        let a_i32 = builder.ins().ireduce(I32, a);
        let b_i32 = builder.ins().ireduce(I32, b);
        let (prod, overflow) = builder.ins().smul_overflow(a_i32, b_i32);
        let overflowed = builder.ins().icmp_imm(IntCC::NotEqual, overflow, 0);
        builder
            .ins()
            .brif(overflowed, slow_block, &[], fast_ok_block, &[]);

        builder.switch_to_block(fast_ok_block);
        let prod_i64 = builder.ins().uextend(I64, prod);
        let boxed = builder.ins().bor(prod_i64, tag_int);
        let cont_args = [boxed.into()];
        builder.ins().jump(cont_block, &cont_args);

        builder.switch_to_block(slow_block);
        let res = Self::emit_deopt_call(builder, ext_funcs, meta_id, deopt_id, spill_slot, vars);
        builder.ins().return_(&[res]);

        builder.switch_to_block(cont_block);
        builder.seal_block(fast_block);
        builder.seal_block(fast_ok_block);
        builder.seal_block(slow_block);
        builder.seal_block(cont_block);
        Some(builder.block_params(cont_block)[0])
    }

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

    fn emit_instruction(
        builder: &mut FunctionBuilder,
        inst: &AirOpcode,
        vars: &[Variable],
        ext_funcs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
        meta_id: u32,
        deopt_id: u32,
        spill_slot: StackSlot,
    ) -> bool {
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
            AirOpcode::LoadInt64 { dst, value } => {
                let val = builder.ins().iconst(I64, *value);
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
                let slot = *ic_slot;
                if let Some(snap) = TypeFeedbackRegistry::add_snapshot(slot) {
                    if let Some(res) = Self::emit_specialized_add(
                        builder,
                        ext_funcs,
                        a,
                        b,
                        slot,
                        &snap,
                        meta_id,
                        deopt_id,
                        spill_slot,
                        vars,
                    ) {
                        builder.def_var(vars[dst.0 as usize], res);
                        return true;
                    }
                }
                let slot_val = builder.ins().iconst(I64, slot as i64);
                let func_ref = *ext_funcs.get("js_add_ic").unwrap();
                let call = builder.ins().call(func_ref, &[a, b, slot_val]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::Sub { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                if let Some(res) = Self::emit_sub_int32(
                    builder,
                    ext_funcs,
                    a,
                    b,
                    meta_id,
                    deopt_id,
                    spill_slot,
                    vars,
                ) {
                    builder.def_var(vars[dst.0 as usize], res);
                } else {
                    let func_ref = *ext_funcs.get("js_sub").unwrap();
                    let call = builder.ins().call(func_ref, &[a, b]);
                    let res = builder.inst_results(call)[0];
                    builder.def_var(vars[dst.0 as usize], res);
                }
            }
            AirOpcode::Mul { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                if let Some(res) = Self::emit_mul_int32(
                    builder,
                    ext_funcs,
                    a,
                    b,
                    meta_id,
                    deopt_id,
                    spill_slot,
                    vars,
                ) {
                    builder.def_var(vars[dst.0 as usize], res);
                } else {
                    let func_ref = *ext_funcs.get("js_mul").unwrap();
                    let call = builder.ins().call(func_ref, &[a, b]);
                    let res = builder.inst_results(call)[0];
                    builder.def_var(vars[dst.0 as usize], res);
                }
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
            AirOpcode::StrictEq { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_strict_eq").unwrap();
                let call = builder.ins().call(func_ref, &[a, b]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::Div { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_div").unwrap();
                let call = builder.ins().call(func_ref, &[a, b]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::Mod { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_mod").unwrap();
                let call = builder.ins().call(func_ref, &[a, b]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::Gt { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_gt").unwrap();
                let call = builder.ins().call(func_ref, &[a, b]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::Gte { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_gte").unwrap();
                let call = builder.ins().call(func_ref, &[a, b]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::Lte { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_lte").unwrap();
                let call = builder.ins().call(func_ref, &[a, b]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::BitAnd { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_bit_and").unwrap();
                let call = builder.ins().call(func_ref, &[a, b]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::BitOr { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_bit_or").unwrap();
                let call = builder.ins().call(func_ref, &[a, b]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::BitXor { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_bit_xor").unwrap();
                let call = builder.ins().call(func_ref, &[a, b]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::Shl { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_bit_shl").unwrap();
                let call = builder.ins().call(func_ref, &[a, b]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::Shr { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_bit_shr").unwrap();
                let call = builder.ins().call(func_ref, &[a, b]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::UShr { dst, lhs, rhs } => {
                let a = builder.use_var(vars[lhs.0 as usize]);
                let b = builder.use_var(vars[rhs.0 as usize]);
                let func_ref = *ext_funcs.get("js_bit_ushr").unwrap();
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
                let slot = *ic_slot;
                if let Some(snap) = TypeFeedbackRegistry::get_prop_snapshot(slot) {
                    if let Some(res) = Self::emit_specialized_get_prop(
                        builder,
                        ext_funcs,
                        o,
                        p,
                        slot,
                        &snap,
                        meta_id,
                        deopt_id,
                        spill_slot,
                        vars,
                    ) {
                        builder.def_var(vars[dst.0 as usize], res);
                        return true;
                    }
                }
                let slot_val = builder.ins().iconst(I64, slot as i64);
                let func_ref = *ext_funcs.get("js_get_prop_ic").unwrap();
                let call = builder.ins().call(func_ref, &[o, p, slot_val]);
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
                let slot = *ic_slot;
                if let Some(snap) = TypeFeedbackRegistry::call_snapshot(slot) {
                    if let Some(res) = Self::emit_specialized_call(
                        builder,
                        ext_funcs,
                        f,
                        *arg_start,
                        *num_args,
                        &snap,
                        meta_id,
                        deopt_id,
                        spill_slot,
                        vars,
                    ) {
                        builder.def_var(vars[dst.0 as usize], res);
                        return true;
                    }
                }
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
            AirOpcode::HasProp { dst, obj, prop } => {
                let o = builder.use_var(vars[obj.0 as usize]);
                let p = builder.use_var(vars[prop.0 as usize]);
                let func_ref = *ext_funcs.get("js_has_prop").unwrap();
                let call = builder.ins().call(func_ref, &[o, p]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::DeleteProp { dst, obj, prop } => {
                let o = builder.use_var(vars[obj.0 as usize]);
                let p = builder.use_var(vars[prop.0 as usize]);
                let func_ref = *ext_funcs.get("js_delete_prop").unwrap();
                let call = builder.ins().call(func_ref, &[o, p]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::TypeOf { dst, src } => {
                let s = builder.use_var(vars[src.0 as usize]);
                let func_ref = *ext_funcs.get("js_type_of").unwrap();
                let call = builder.ins().call(func_ref, &[s]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::InstanceOf { dst, obj, ctor } => {
                let o = builder.use_var(vars[obj.0 as usize]);
                let c = builder.use_var(vars[ctor.0 as usize]);
                let func_ref = *ext_funcs.get("js_instance_of").unwrap();
                let call = builder.ins().call(func_ref, &[o, c]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            _ => {
                let func_ref = *ext_funcs.get("js_unimplemented").unwrap();
                let call = builder.ins().call(func_ref, &[]);
                let _res = builder.inst_results(call)[0];
                println!(
                    "[Tier2] Warning: Unsupported Opcode {:?} - fallback to undefined",
                    inst
                );
            }
        }
        false
    }

    fn build_deopt_points(air: &AirFunction) -> (Vec<DeoptPoint>, HashMap<(u32, usize), u32>) {
        let mut points = Vec::new();
        let mut map = HashMap::new();

        for block in &air.blocks {
            for (idx, _inst) in block.insts.iter().enumerate() {
                let id = points.len() as u32;
                points.push(DeoptPoint {
                    block_id: block.id.0,
                    inst_index: idx as u32,
                });
                map.insert((block.id.0, idx), id);
            }
        }

        (points, map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::opcodes::AirBlockId;
    use crate::bytecode::{AirBlock, AirConstantPool, AirOpcode, AirReg, AirTerminator};
    use crate::jit_engine::AlbedoJitEngine;
    use crate::js_value::JsValue;

    /// Helper: compila com Tier2 e executa com 2 argumentos.
    fn tier2_run_2(air: &AirFunction, a: JsValue, b: JsValue) -> JsValue {
        let mut engine = AlbedoJitEngine::new().unwrap();
        let mut compiler = Tier2Compiler::new(&mut engine);
        let id = compiler.compile(air).expect("Tier2 compile falhou");
        engine.module.finalize_definitions().unwrap();
        let ptr = engine.module.get_finalized_function(id);
        let f: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };
        JsValue(f(a.0, b.0))
    }

    fn make_binary_air(name: &str, op: AirOpcode) -> AirFunction {
        AirFunction {
            name: name.to_string(),
            num_params: 2,
            registers_count: 3,
            blocks: vec![AirBlock {
                id: AirBlockId(0),
                insts: vec![op],
                terminator: Some(AirTerminator::Return(AirReg(2))),
            }],
            const_pool: AirConstantPool::default(),
        }
    }

    #[test]
    fn test_tier2_sub_int32_specialization() {
        let air = make_binary_air(
            "sub_t2",
            AirOpcode::Sub {
                dst: AirReg(2),
                lhs: AirReg(0),
                rhs: AirReg(1),
            },
        );
        let result = tier2_run_2(&air, JsValue::int32(10), JsValue::int32(3));
        assert_eq!(result.as_int32(), 7);
    }

    #[test]
    fn test_tier2_mul_int32_specialization() {
        let air = make_binary_air(
            "mul_t2",
            AirOpcode::Mul {
                dst: AirReg(2),
                lhs: AirReg(0),
                rhs: AirReg(1),
            },
        );
        let result = tier2_run_2(&air, JsValue::int32(6), JsValue::int32(7));
        assert_eq!(result.as_int32(), 42);
    }

    #[test]
    fn test_tier2_sub_deopt_bailout() {
        // Float64 tipos → guard int32 falha → slow path via js_sub
        let air = make_binary_air(
            "sub_deopt",
            AirOpcode::Sub {
                dst: AirReg(2),
                lhs: AirReg(0),
                rhs: AirReg(1),
            },
        );
        let result = tier2_run_2(&air, JsValue::float64(10.5), JsValue::float64(3.5));
        assert_eq!(result.as_float64(), 7.0);
    }

    #[test]
    fn test_tier2_mul_deopt_bailout() {
        // Float64 tipos → guard int32 falha → slow path via js_mul
        let air = make_binary_air(
            "mul_deopt",
            AirOpcode::Mul {
                dst: AirReg(2),
                lhs: AirReg(0),
                rhs: AirReg(1),
            },
        );
        let result = tier2_run_2(&air, JsValue::float64(2.5), JsValue::float64(4.0));
        assert_eq!(result.as_float64(), 10.0);
    }

    #[test]
    fn test_tier2_add_int32_without_feedback() {
        // Sem type feedback → slow path via js_add_ic genérico
        let air = AirFunction {
            name: "add_nofb".to_string(),
            num_params: 2,
            registers_count: 3,
            blocks: vec![AirBlock {
                id: AirBlockId(0),
                insts: vec![AirOpcode::Add {
                    dst: AirReg(2),
                    lhs: AirReg(0),
                    rhs: AirReg(1),
                    ic_slot: 999,
                }],
                terminator: Some(AirTerminator::Return(AirReg(2))),
            }],
            const_pool: AirConstantPool::default(),
        };
        let result = tier2_run_2(&air, JsValue::int32(20), JsValue::int32(22));
        assert_eq!(result.as_int32(), 42);
    }

    #[test]
    fn test_tier2_add_deopt_real() {
        // Testa o bailout real para o Tier 0 (interpretador)
        // 1. Criar AIR que faz Add(0, 1) -> 2 e depois Multiply(2, 2) -> 3
        let air = AirFunction {
            name: "add_deopt_real".to_string(),
            num_params: 2,
            registers_count: 4,
            blocks: vec![AirBlock {
                id: AirBlockId(0),
                insts: vec![
                    AirOpcode::Add {
                        dst: AirReg(2),
                        lhs: AirReg(0),
                        rhs: AirReg(1),
                        ic_slot: 0,
                    },
                    AirOpcode::Mul {
                        dst: AirReg(3),
                        lhs: AirReg(2),
                        rhs: AirReg(2),
                    },
                ],
                terminator: Some(AirTerminator::Return(AirReg(3))),
            }],
            const_pool: AirConstantPool::default(),
        };

        // 2. Gravar feedback Monomorphic(Int32, Int32) para o slot 0
        crate::type_feedback::TypeFeedbackRegistry::record_add(
            0,
            JsValue::int32(1),
            JsValue::int32(1),
        );

        // 3. Compilar no Tier 2 (vai especializar Add para Int32)
        let mut engine = AlbedoJitEngine::new().unwrap();
        let mut compiler = Tier2Compiler::new(&mut engine);
        let id = compiler.compile(&air).expect("Tier2 compile falhou");
        engine.module.finalize_definitions().unwrap();
        let ptr = engine.module.get_finalized_function(id);

        // 4. Executar com Float64 (vai falhar o guard e dar bailout)
        // No interpretador:
        //   Add(1.5, 2.5) -> 4.0
        //   Mul(4.0, 4.0) -> 16.0
        let native_func: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };
        let result = JsValue(native_func(
            JsValue::float64(1.5).0,
            JsValue::float64(2.5).0,
        ));

        assert_eq!(result.as_float64(), 16.0);
    }
}
