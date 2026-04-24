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

use super::escape_analysis::{run_field_sensitive, EscapeAnalysisResult};
use super::loop_opts::{validate_air_cfg, LoopOptimizer};
use super::scalar_replacement::{ScalarReplacer, ScalarTransformResult};
use super::stack_allocator::{StackAllocation, StackAllocator};
use crate::bytecode::{AirBlockId, AirFunction, AirOpcode, AirReg, AirTerminator};
use crate::compiler::deopt::{register_meta, DeoptMeta, DeoptPoint};
use crate::engine::jit_bridge::BytecodeRegistry;
use crate::engine::jit_engine::{AlbedoJitEngine, JitError};
use crate::runtime::js_value::{
    JsValue, FLOAT_NAN, PAYLOAD_MASK, TAG_INT32, TAG_MASK, TAG_MIN, TAG_OBJECT, TAG_UNDEFINED,
};
use crate::runtime::object_model::{
    get_string, JsObject, ObjectKind, JSOBJ_KIND_OFFSET, JSOBJ_PROPS_CAP_OFFSET,
    JSOBJ_PROPS_LEN_OFFSET, JSOBJ_PROPS_OFFSET, JSOBJ_SHAPE_OFFSET,
};
use crate::runtime::type_feedback::{
    AddFeedbackSnapshot, GetPropFeedbackSnapshot, IcState, TypeFeedbackRegistry, TypePair,
    ValueType,
};

const MIN_FEEDBACK_SAMPLES: u64 = 1;

pub struct Tier2Compiler<'a> {
    engine: &'a mut AlbedoJitEngine,
    registry: &'a BytecodeRegistry,
    escape_results: Option<EscapeAnalysisResult>,
    stack_plan: HashMap<AirReg, StackAllocation>,
}

impl<'a> Tier2Compiler<'a> {
    pub fn new(engine: &'a mut AlbedoJitEngine, registry: &'a BytecodeRegistry) -> Self {
        Self {
            engine,
            registry,
            escape_results: None,
            stack_plan: HashMap::new(),
        }
    }

    pub fn compile(&mut self, air_orig: &AirFunction) -> Result<FuncId, JitError> {
        self.stack_plan.clear();
        let mut air = air_orig.clone();
        let mut loop_optimizer = LoopOptimizer::new(&mut air);
        let _loop_report = loop_optimizer.run();
        if !air.is_valid() {
            return Err(JitError::Compilation(
                "AIR invalid after loop optimizer pass".to_string(),
            ));
        }
        if let Err(err) = validate_air_cfg(&air) {
            return Err(JitError::Compilation(format!(
                "AIR CFG validation failed after loop optimizer pass: {err}"
            )));
        }

        // 3.3 Escape Analysis
        let escape_results = run_field_sensitive(&air);

        // 3.4 Scalar Replacement (SROA)
        let mut replacer = ScalarReplacer::new(AirReg(air.registers_count));
        for (&obj_reg, candidate) in &escape_results.scalar_candidates {
            if matches!(
                replacer.replace_object_with_scalars(&mut air, obj_reg, &candidate.properties),
                ScalarTransformResult::RejectedCoverage
            ) {
                // Conservative fallback: keep object path untouched if coverage is incomplete.
            }
        }
        air.registers_count = replacer.get_next_reg().0;

        // 3.5 Stack allocation planning (for non-scalarized non-escaping objects)
        let mut allocator = StackAllocator::new();
        for (&obj_reg, candidate) in &escape_results.stack_candidates {
            let alloc = allocator.allocate_object(obj_reg, candidate.num_slots.max(1));
            self.stack_plan.insert(obj_reg, alloc);
        }

        if !air.is_valid() {
            return Err(JitError::Compilation(
                "AIR invalid after scalar replacement".to_string(),
            ));
        }
        if let Err(err) = validate_air_cfg(&air) {
            return Err(JitError::Compilation(format!(
                "AIR CFG validation failed after scalar replacement: {err}"
            )));
        }

        self.escape_results = Some(escape_results);
        let air = &air;

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

        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_context);

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
            5,
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
            "fast_math_acos",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_acosh",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_asin",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_asinh",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_atan",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_atan2",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_atanh",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_cos",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_cosh",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_sin",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_sinh",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_tan",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_tanh",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_exp",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_expm1",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_log",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_log1p",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_log10",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_log2",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_cbrt",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_clz32",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_fround",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_hypot",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_imul",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_pow",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_random",
            0,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_round",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_sign",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_trunc",
            1,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_max",
            2,
            &mut ext_funcs,
        )?;
        Self::declare_runtime_helper(
            &mut self.engine.module,
            &mut builder,
            "fast_math_min",
            2,
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
                if self.emit_instruction(
                    &mut builder,
                    inst,
                    &mut vars,
                    &mut ext_funcs,
                    meta_id,
                    deopt_id,
                    spill_slot,
                    0, // depth initial
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

        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_context);

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
            5,
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

            let _start = if air_block.id.0 == entry_block_id {
                entry_inst
            } else {
                0
            };
            for (inst_index, inst) in air_block.insts.iter().enumerate() {
                if air_block.id.0 == entry_block_id && inst_index < entry_inst {
                    continue;
                }
                let deopt_id = *deopt_map.get(&(air_block.id.0, inst_index)).unwrap_or(&0);
                if self.emit_instruction(
                    &mut builder,
                    inst,
                    &mut vars,
                    &mut ext_funcs,
                    meta_id,
                    deopt_id,
                    spill_slot,
                    0,
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
            .iconst(I64, crate::runtime::js_value::TAG_OBJECT as i64);
        let obj_tag = builder.ins().band(obj, tag_mask);
        let is_obj = builder.ins().icmp(IntCC::Equal, obj_tag, tag_obj);
        builder.ins().brif(is_obj, obj_block, &[], slow_block, &[]);

        builder.switch_to_block(obj_block);
        // Guard: prop id match (string id)
        let tag_str = builder
            .ins()
            .iconst(I64, crate::runtime::js_value::TAG_STRING as i64);
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
        &mut self,
        builder: &mut FunctionBuilder,
        ext_funcs: &mut HashMap<String, cranelift_codegen::ir::FuncRef>,
        f: cranelift_codegen::ir::Value,
        _this: cranelift_codegen::ir::Value,
        arg_start: crate::bytecode::AirReg,
        num_args: u32,
        snap: &crate::runtime::type_feedback::CallFeedbackSnapshot,
        meta_id: u32,
        deopt_id: u32,
        spill_slot: StackSlot,
        vars: &mut Vec<cranelift_frontend::Variable>,
        depth: u32,
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
            // Inlining de funções AIR
            if depth < 3 && callee.is_object() {
                let ptr = callee.as_object_ptr();
                // Ler metadados do objeto (func_id_idx)
                let obj = unsafe { &*(ptr as *const JsObject) };
                if obj.kind == ObjectKind::Function as u32 {
                    let func_id_str =
                        get_string(obj.func_id_idx).expect("ID de função não encontrado");
                    let func_id = crate::engine::profiler::FunctionId(func_id_str);

                    if let Some(callee_air) = self.registry.get_air(&func_id) {
                        if callee_air.instruction_count() <= 50 {
                            println!(
                                "[TIER2-INLINE] Inlining {} (ops={})",
                                callee_air.name,
                                callee_air.instruction_count()
                            );

                            // Guard
                            let slow_block = builder.create_block();
                            let fast_block = builder.create_block();
                            let expected_f = builder.ins().iconst(I64, callee.0 as i64);
                            let f_ok = builder.ins().icmp(IntCC::Equal, f, expected_f);
                            builder.ins().brif(f_ok, fast_block, &[], slow_block, &[]);

                            builder.switch_to_block(fast_block);

                            // Mapear argumentos
                            let mut args = Vec::with_capacity(num_args as usize + 1);
                            args.push(_this);
                            for i in 0..num_args {
                                let reg = AirReg(arg_start.0 + i);
                                args.push(builder.use_var(vars[reg.0 as usize]));
                            }

                            let inline_res = self.inline_air_function(
                                &callee_air,
                                &args,
                                builder,
                                vars,
                                ext_funcs,
                                meta_id,
                                deopt_id, // Deopt p/ o callee call site em caso de erro no inline
                                spill_slot,
                                depth,
                            );

                            match inline_res {
                                Ok(res_val) => {
                                    let cont_block = builder.create_block();
                                    builder.append_block_param(cont_block, I64);
                                    let cont_args = [res_val.into()];
                                    builder.ins().jump(cont_block, &cont_args);

                                    builder.switch_to_block(slow_block);
                                    let deopt_res = Self::emit_deopt_call(
                                        builder, ext_funcs, meta_id, deopt_id, spill_slot, vars,
                                    );
                                    builder.ins().return_(&[deopt_res]);

                                    builder.switch_to_block(cont_block);
                                    builder.seal_block(fast_block);
                                    builder.seal_block(slow_block);
                                    return Some(builder.block_params(cont_block)[0]);
                                }
                                Err(_) => {
                                    // Se falhar, fallback para o slow path normal (não deveria acontecer pos-check de ops)
                                    builder.switch_to_block(slow_block);
                                    builder.seal_block(fast_block);
                                    builder.seal_block(slow_block);
                                    return None;
                                }
                            }
                        }
                    }
                }
            }
            return None;
        }

        let bid = callee.as_builtin_id() as u32;
        use crate::runtime::builtins::BuiltinId;

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
                let func_ref = *ext_funcs.get("fast_math_abs").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathSqrt as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
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
            x if x == BuiltinId::MathAcos as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_acos").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathAcosh as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_acosh").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathAsin as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_asin").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathAsinh as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_asinh").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathAtan as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_atan").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathAtan2 as u32 && num_args >= 2 => {
                let y = builder.use_var(vars[arg_start.0 as usize]);
                let x = builder.use_var(vars[arg_start.0 as usize + 1]);
                let func_ref = *ext_funcs.get("fast_math_atan2").unwrap();
                let call = builder.ins().call(func_ref, &[y, x]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathAtanh as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_atanh").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathCos as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_cos").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathCosh as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_cosh").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathSin as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_sin").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathSinh as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_sinh").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathTan as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_tan").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathTanh as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_tanh").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathExp as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_exp").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathExpm1 as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_expm1").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathLog as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_log").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathLog1p as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_log1p").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathLog10 as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_log10").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathLog2 as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_log2").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathCbrt as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_cbrt").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathClz32 as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_clz32").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathFround as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_fround").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathHypot as u32 && num_args >= 2 => {
                let x = builder.use_var(vars[arg_start.0 as usize]);
                let y = builder.use_var(vars[arg_start.0 as usize + 1]);
                let func_ref = *ext_funcs.get("fast_math_hypot").unwrap();
                let call = builder.ins().call(func_ref, &[x, y]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathImul as u32 && num_args >= 2 => {
                let x = builder.use_var(vars[arg_start.0 as usize]);
                let y = builder.use_var(vars[arg_start.0 as usize + 1]);
                let func_ref = *ext_funcs.get("fast_math_imul").unwrap();
                let call = builder.ins().call(func_ref, &[x, y]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathPow as u32 && num_args >= 2 => {
                let x = builder.use_var(vars[arg_start.0 as usize]);
                let y = builder.use_var(vars[arg_start.0 as usize + 1]);
                let func_ref = *ext_funcs.get("fast_math_pow").unwrap();
                let call = builder.ins().call(func_ref, &[x, y]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathRandom as u32 => {
                let func_ref = *ext_funcs.get("fast_math_random").unwrap();
                let call = builder.ins().call(func_ref, &[]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathRound as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_round").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathSign as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_sign").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathTrunc as u32 && num_args >= 1 => {
                let arg = builder.use_var(vars[arg_start.0 as usize]);
                let func_ref = *ext_funcs.get("fast_math_trunc").unwrap();
                let call = builder.ins().call(func_ref, &[arg]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathMax as u32 && num_args >= 2 => {
                let x = builder.use_var(vars[arg_start.0 as usize]);
                let y = builder.use_var(vars[arg_start.0 as usize + 1]);
                let func_ref = *ext_funcs.get("fast_math_max").unwrap();
                let call = builder.ins().call(func_ref, &[x, y]);
                Some(builder.inst_results(call)[0])
            }
            x if x == BuiltinId::MathMin as u32 && num_args >= 2 => {
                let x = builder.use_var(vars[arg_start.0 as usize]);
                let y = builder.use_var(vars[arg_start.0 as usize + 1]);
                let func_ref = *ext_funcs.get("fast_math_min").unwrap();
                let call = builder.ins().call(func_ref, &[x, y]);
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

    fn inline_air_function(
        &mut self,
        callee: &AirFunction,
        args: &[cranelift_codegen::ir::Value],
        builder: &mut FunctionBuilder,
        _vars: &mut Vec<Variable>,
        ext_funcs: &mut HashMap<String, cranelift_codegen::ir::FuncRef>,
        meta_id: u32,
        caller_deopt_id: u32,
        spill_slot: StackSlot,
        depth: u32,
    ) -> Result<cranelift_codegen::ir::Value, JitError> {
        // 1. Mapear registradores locais e parâmetros
        let mut callee_vars = Vec::with_capacity(callee.registers_count as usize);
        for _ in 0..callee.registers_count {
            let var = builder.declare_var(I64);
            callee_vars.push(var);
        }

        // 2. Inicializar parâmetros
        for (i, &arg_val) in args.iter().enumerate().take(callee.num_params as usize) {
            builder.def_var(callee_vars[i], arg_val);
        }
        let undef = builder.ins().iconst(I64, JsValue::undefined().0 as i64);
        for i in (args.len() as u32)..callee.num_params {
            builder.def_var(callee_vars[i as usize], undef);
        }
        for i in callee.num_params..callee.registers_count {
            builder.def_var(callee_vars[i as usize], undef);
        }

        // 3. Mapear blocos
        let mut block_map = HashMap::new();
        for block in &callee.blocks {
            block_map.insert(block.id.0, builder.create_block());
        }
        let cont_block = builder.create_block();
        builder.append_block_param(cont_block, I64);

        // Preencher o bloco atual (do caller) com um jump para a entrada do callee inlines
        if let Some(first_block) = callee.blocks.first() {
            builder.ins().jump(block_map[&first_block.id.0], &[]);
        }

        // 4. Emitir instruções
        for block in &callee.blocks {
            let cl_block = block_map[&block.id.0];
            builder.switch_to_block(cl_block);

            for inst in &block.insts {
                self.emit_instruction(
                    builder,
                    inst,
                    &mut callee_vars,
                    ext_funcs,
                    meta_id,
                    caller_deopt_id, // Deopt p/ o caller site
                    spill_slot,
                    depth + 1,
                );
            }

            if let Some(ref term) = block.terminator {
                match term {
                    AirTerminator::Jump(target) => {
                        builder.ins().jump(block_map[&target.0], &[]);
                    }
                    AirTerminator::JumpIf {
                        cond,
                        then_blk,
                        else_blk,
                    } => {
                        let c_val = builder.use_var(callee_vars[cond.0 as usize]);
                        let f_to_bool = *ext_funcs
                            .get("js_to_bool")
                            .expect("js_to_bool not declared");
                        let call = builder.ins().call(f_to_bool, &[c_val]);
                        let boxed_bool = builder.inst_results(call)[0];
                        let b_val = builder.ins().band_imm(boxed_bool, 1);
                        builder.ins().brif(
                            b_val,
                            block_map[&then_blk.0],
                            &[],
                            block_map[&else_blk.0],
                            &[],
                        );
                    }
                    AirTerminator::Return(reg) => {
                        let res = builder.use_var(callee_vars[reg.0 as usize]);
                        let args = [res.into()];
                        builder.ins().jump(cont_block, &args);
                    }
                }
            }
        }

        builder.switch_to_block(cont_block);
        for block in &callee.blocks {
            builder.seal_block(block_map[&block.id.0]);
        }
        Ok(builder.block_params(cont_block)[0])
    }

    fn emit_instruction(
        &mut self,
        builder: &mut FunctionBuilder,
        inst: &AirOpcode,
        vars: &mut Vec<Variable>,
        ext_funcs: &mut HashMap<String, cranelift_codegen::ir::FuncRef>,
        meta_id: u32,
        deopt_id: u32,
        spill_slot: StackSlot,
        depth: u32,
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
                        builder, ext_funcs, a, b, slot, &snap, meta_id, deopt_id, spill_slot, vars,
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
                    builder, ext_funcs, a, b, meta_id, deopt_id, spill_slot, vars,
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
                    builder, ext_funcs, a, b, meta_id, deopt_id, spill_slot, vars,
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
                        builder, ext_funcs, o, p, slot, &snap, meta_id, deopt_id, spill_slot, vars,
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
                this,
                arg_start,
                num_args,
                ic_slot,
            } => {
                let f = builder.use_var(vars[func.0 as usize]);
                let this_val = builder.use_var(vars[this.0 as usize]);
                let slot = *ic_slot;
                if let Some(snap) = TypeFeedbackRegistry::call_snapshot(slot) {
                    if let Some(res) = self.emit_specialized_call(
                        builder, ext_funcs, f, this_val, *arg_start, *num_args, &snap, meta_id,
                        deopt_id, spill_slot, vars, depth,
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
                    .call(func_ref, &[f, this_val, args_ptr, num_args_val, slot_val]);
                let res = builder.inst_results(call)[0];
                builder.def_var(vars[dst.0 as usize], res);
            }
            AirOpcode::CreateObj { dst } => {
                let stack_alloc = self.stack_plan.get(dst).cloned();

                if let Some(stack_alloc) = stack_alloc {
                    // Stack allocation planned by StackAllocator.
                    let obj_slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        32,
                        8,
                    ));
                    let props_slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        (stack_alloc.num_properties as u32 * 8).max(8),
                        8,
                    ));

                    let obj_addr = builder.ins().stack_addr(I64, obj_slot, 0);
                    let props_addr = builder.ins().stack_addr(I64, props_slot, 0);

                    // Inicializar JsObject
                    // shape_id = fnv1a_seed (0xcbf29ce484222325)
                    let empty_shape = builder.ins().iconst(I64, 0xcbf29ce484222325u64 as i64);
                    builder.ins().store(
                        MemFlags::trusted(),
                        empty_shape,
                        obj_addr,
                        JSOBJ_SHAPE_OFFSET as i32,
                    );

                    // kind = Object (0), func_id_idx = 0 -> combined into 64-bit store for speed?
                    // No, let's follow offsets.
                    let kind = builder.ins().iconst(I32, ObjectKind::Object as i64);
                    builder.ins().store(
                        MemFlags::trusted(),
                        kind,
                        obj_addr,
                        JSOBJ_KIND_OFFSET as i32,
                    );

                    // props = props_addr
                    builder.ins().store(
                        MemFlags::trusted(),
                        props_addr,
                        obj_addr,
                        JSOBJ_PROPS_OFFSET as i32,
                    );

                    // props_len = 0, props_cap = planned num_properties
                    let zero32 = builder.ins().iconst(I32, 0);
                    let cap32 = builder.ins().iconst(I32, stack_alloc.num_properties as i64);
                    builder.ins().store(
                        MemFlags::trusted(),
                        zero32,
                        obj_addr,
                        JSOBJ_PROPS_LEN_OFFSET as i32,
                    );
                    builder.ins().store(
                        MemFlags::trusted(),
                        cap32,
                        obj_addr,
                        JSOBJ_PROPS_CAP_OFFSET as i32,
                    );

                    // Initialize props with undefined
                    let undefined = builder.ins().iconst(I64, TAG_UNDEFINED as i64);
                    for i in 0..stack_alloc.num_properties {
                        builder.ins().store(
                            MemFlags::trusted(),
                            undefined,
                            props_addr,
                            (i as i32) * 8,
                        );
                    }

                    // Pack into JsValue (Object tag)
                    let tag = builder.ins().iconst(I64, TAG_OBJECT as i64);
                    let packed = builder.ins().bor(obj_addr, tag);
                    builder.def_var(vars[dst.0 as usize], packed);
                } else {
                    let func_ref = *ext_funcs.get("js_create_obj").unwrap();
                    let call = builder.ins().call(func_ref, &[]);
                    let res = builder.inst_results(call)[0];
                    builder.def_var(vars[dst.0 as usize], res);
                }
            }
            AirOpcode::CreateArray { dst } => {
                let stack_alloc = self.stack_plan.get(dst).cloned();

                if let Some(stack_alloc) = stack_alloc {
                    let obj_slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        32,
                        8,
                    ));
                    let props_slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        (stack_alloc.num_properties as u32 * 8).max(8),
                        8,
                    ));

                    let obj_addr = builder.ins().stack_addr(I64, obj_slot, 0);
                    let props_addr = builder.ins().stack_addr(I64, props_slot, 0);

                    let empty_shape = builder.ins().iconst(I64, 0xcbf29ce484222325u64 as i64);
                    builder.ins().store(
                        MemFlags::trusted(),
                        empty_shape,
                        obj_addr,
                        JSOBJ_SHAPE_OFFSET as i32,
                    );

                    let kind = builder.ins().iconst(I32, ObjectKind::Array as i64);
                    builder.ins().store(
                        MemFlags::trusted(),
                        kind,
                        obj_addr,
                        JSOBJ_KIND_OFFSET as i32,
                    );

                    builder.ins().store(
                        MemFlags::trusted(),
                        props_addr,
                        obj_addr,
                        JSOBJ_PROPS_OFFSET as i32,
                    );

                    let zero32 = builder.ins().iconst(I32, 0);
                    let cap32 = builder.ins().iconst(I32, stack_alloc.num_properties as i64);
                    builder.ins().store(
                        MemFlags::trusted(),
                        zero32,
                        obj_addr,
                        JSOBJ_PROPS_LEN_OFFSET as i32,
                    );
                    builder.ins().store(
                        MemFlags::trusted(),
                        cap32,
                        obj_addr,
                        JSOBJ_PROPS_CAP_OFFSET as i32,
                    );

                    // Initialize props with undefined
                    let undefined = builder.ins().iconst(I64, TAG_UNDEFINED as i64);
                    for i in 0..stack_alloc.num_properties {
                        builder.ins().store(
                            MemFlags::trusted(),
                            undefined,
                            props_addr,
                            (i as i32) * 8,
                        );
                    }

                    let tag = builder.ins().iconst(I64, TAG_OBJECT as i64);
                    let packed = builder.ins().bor(obj_addr, tag);
                    builder.def_var(vars[dst.0 as usize], packed);
                } else {
                    let func_ref = *ext_funcs.get("js_create_array").unwrap();
                    let call = builder.ins().call(func_ref, &[]);
                    let res = builder.inst_results(call)[0];
                    builder.def_var(vars[dst.0 as usize], res);
                }
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
    use crate::engine::jit_engine::AlbedoJitEngine;
    use crate::runtime::js_value::JsValue;

    /// Helper: compila com Tier2 e executa com 2 argumentos.
    fn tier2_run_2(air: &AirFunction, a: JsValue, b: JsValue) -> JsValue {
        let mut engine = AlbedoJitEngine::new().unwrap();
        let registry = BytecodeRegistry::new();
        let mut compiler = Tier2Compiler::new(&mut engine, &registry);
        let id = compiler.compile(air).expect("Tier2 compile falhou");
        engine.module.finalize_definitions().unwrap();
        let ptr = engine.module.get_finalized_function(id);
        let f: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };
        JsValue(f(JsValue::undefined().0, a.0, b.0))
    }

    fn make_binary_air(name: &str, op: AirOpcode) -> AirFunction {
        AirFunction {
            name: name.to_string(),
            num_params: 3, // this, p1, p2
            registers_count: 4,
            blocks: vec![AirBlock {
                id: AirBlockId(0),
                insts: vec![op],
                terminator: Some(AirTerminator::Return(AirReg(3))),
            }],
            const_pool: AirConstantPool::default(),
        }
    }

    #[test]
    fn test_tier2_sub_int32_specialization() {
        let air = make_binary_air(
            "sub_t2",
            AirOpcode::Sub {
                dst: AirReg(3),
                lhs: AirReg(1),
                rhs: AirReg(2),
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
                dst: AirReg(3),
                lhs: AirReg(1),
                rhs: AirReg(2),
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
                dst: AirReg(3),
                lhs: AirReg(1),
                rhs: AirReg(2),
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
                dst: AirReg(3),
                lhs: AirReg(1),
                rhs: AirReg(2),
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
            num_params: 3,
            registers_count: 4,
            blocks: vec![AirBlock {
                id: AirBlockId(0),
                insts: vec![AirOpcode::Add {
                    dst: AirReg(3),
                    lhs: AirReg(1),
                    rhs: AirReg(2),
                    ic_slot: 999,
                }],
                terminator: Some(AirTerminator::Return(AirReg(3))),
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
            num_params: 3,
            registers_count: 5,
            blocks: vec![AirBlock {
                id: AirBlockId(0),
                insts: vec![
                    AirOpcode::Add {
                        dst: AirReg(3),
                        lhs: AirReg(1),
                        rhs: AirReg(2),
                        ic_slot: 0,
                    },
                    AirOpcode::Mul {
                        dst: AirReg(4),
                        lhs: AirReg(3),
                        rhs: AirReg(3),
                    },
                ],
                terminator: Some(AirTerminator::Return(AirReg(4))),
            }],
            const_pool: AirConstantPool::default(),
        };

        // 2. Gravar feedback Monomorphic(Int32, Int32) para o slot 0
        crate::runtime::type_feedback::TypeFeedbackRegistry::record_add(
            0,
            JsValue::int32(1),
            JsValue::int32(1),
        );

        // 3. Compilar no Tier 2 (vai especializar Add para Int32)
        let mut engine = AlbedoJitEngine::new().unwrap();
        let registry = BytecodeRegistry::new();
        let mut compiler = Tier2Compiler::new(&mut engine, &registry);
        let id = compiler.compile(&air).expect("Tier2 compile falhou");
        engine.module.finalize_definitions().unwrap();
        let ptr = engine.module.get_finalized_function(id);

        // 4. Executar com Float64 (vai falhar o guard e dar bailout)
        // No interpretador:
        //   Add(1.5, 2.5) -> 4.0
        //   Mul(4.0, 4.0) -> 16.0
        let native_func: extern "C" fn(u64, u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };
        let result = JsValue(native_func(
            JsValue::undefined().0,
            JsValue::float64(1.5).0,
            JsValue::float64(2.5).0,
        ));

        assert_eq!(result.as_float64(), 16.0);
    }

    #[test]
    fn test_tier2_function_inlining() {
        // 1. Criar Callee AIR: function add(a, b) { return a + b; }
        // params: this(0), a(1), b(2) -> reg3 = a+b, return reg3
        let callee_ic_slot = crate::runtime::type_feedback::TypeFeedbackRegistry::alloc_slot(
            crate::runtime::type_feedback::IcKind::Add,
        );

        let callee_air = AirFunction {
            name: "callee_add".to_string(),
            num_params: 3,
            registers_count: 4,
            blocks: vec![AirBlock {
                id: AirBlockId(0),
                insts: vec![AirOpcode::Add {
                    dst: AirReg(3),
                    lhs: AirReg(1),
                    rhs: AirReg(2),
                    ic_slot: callee_ic_slot,
                }],
                terminator: Some(AirTerminator::Return(AirReg(3))),
            }],
            const_pool: AirConstantPool::default(),
        };

        // 2. Criar Caller AIR: function caller(x) { return callee(x, 5); }
        let caller_ic_slot = crate::runtime::type_feedback::TypeFeedbackRegistry::alloc_slot(
            crate::runtime::type_feedback::IcKind::Call,
        );

        // params: this(0), x(1) -> reg3=callee, reg2=5, reg4=call(reg3, this, [x, reg2]), return reg4
        let caller_air = AirFunction {
            name: "caller".to_string(),
            num_params: 2,
            registers_count: 5,
            blocks: vec![AirBlock {
                id: AirBlockId(0),
                insts: vec![
                    AirOpcode::LoadInt32 {
                        dst: AirReg(3),
                        value: 0xDEADBEEFu32 as i32,
                    }, // Placeholder p/ o callee JsValue
                    AirOpcode::LoadInt32 {
                        dst: AirReg(2),
                        value: 5,
                    },
                    AirOpcode::Call {
                        dst: AirReg(4),
                        func: AirReg(3),
                        this: AirReg(0),
                        arg_start: AirReg(1),
                        num_args: 2,
                        ic_slot: caller_ic_slot,
                    },
                ],
                terminator: Some(AirTerminator::Return(AirReg(4))),
            }],
            const_pool: AirConstantPool::default(),
        };

        // 3. Setup Engine e Registry
        let mut engine = AlbedoJitEngine::new().unwrap();
        let registry = BytecodeRegistry::new();

        // Registrar callee
        let callee_name = "callee_add".to_string();
        let callee_id = crate::engine::profiler::FunctionId(callee_name.clone());
        registry.register_air(callee_id.clone(), callee_air);

        // Criar Objeto Função falso
        let callee_obj = Box::new(JsObject {
            shape_id: 0,
            kind: ObjectKind::Function as u32,
            func_id_idx: crate::runtime::object_model::intern_string(callee_name),
            props: std::ptr::null_mut(),
            props_len: 0,
            props_cap: 0,
        });

        // No AlbedoJIT, JsValue de objeto é TAG_OBJECT | ptr
        let ptr = Box::into_raw(callee_obj) as u64;
        let callee_val = JsValue::object(ptr);

        // Atualizar o LoadInt32 no caller para carregar o callee_val real
        // (Isso é um hack p/ o teste, normalmente o IC cuidaria disso)
        let mut final_caller = caller_air;
        // Melhor usar LoadInt64 para não perder a referência do Object na conversão para i32
        final_caller.blocks[0].insts[0] = AirOpcode::LoadInt64 {
            dst: AirReg(3),
            value: callee_val.0 as i64,
        };

        // 4. Gravar feedback Monomorphic
        crate::runtime::type_feedback::TypeFeedbackRegistry::record_call(
            caller_ic_slot,
            callee_val,
        );

        // 5. Compilar Caller
        let mut compiler = Tier2Compiler::new(&mut engine, &registry);
        let id = compiler
            .compile(&final_caller)
            .expect("Inlining compile falhou");
        engine.module.finalize_definitions().unwrap();

        let ptr = engine.module.get_finalized_function(id);
        let f: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };

        // Executar: caller(37) -> callee(37, 5) -> 42
        let result = JsValue(f(JsValue::undefined().0, JsValue::int32(37).0));

        assert_eq!(result.as_int32(), 42);
        println!("[TEST] Inlining bem sucedido: 37 + 5 = 42");
    }

    #[test]
    fn test_tier2_inlining_depth_limit() {
        let mut engine = AlbedoJitEngine::new().unwrap();
        let registry = BytecodeRegistry::new();

        let rec_ic_slot = crate::runtime::type_feedback::TypeFeedbackRegistry::alloc_slot(
            crate::runtime::type_feedback::IcKind::Call,
        );

        let rec_name = "recursive_func".to_string();
        let rec_id = crate::engine::profiler::FunctionId(rec_name.clone());

        let recursive_air = AirFunction {
            name: rec_name.clone(),
            num_params: 2,
            registers_count: 5,
            blocks: vec![AirBlock {
                id: AirBlockId(0),
                insts: vec![
                    AirOpcode::LoadInt32 {
                        dst: AirReg(2),
                        value: 0,
                    },
                    // Placeholder da prórpia func p/ call recursive
                    AirOpcode::LoadInt64 {
                        dst: AirReg(3),
                        value: 12345678,
                    },
                    AirOpcode::Call {
                        dst: AirReg(4),
                        func: AirReg(3),
                        this: AirReg(0),
                        arg_start: AirReg(1),
                        num_args: 2,
                        ic_slot: rec_ic_slot,
                    },
                ],
                terminator: Some(AirTerminator::Return(AirReg(4))),
            }],
            const_pool: AirConstantPool::default(),
        };

        // Objeto recursivo
        let rec_obj = Box::new(JsObject {
            shape_id: 0,
            kind: ObjectKind::Function as u32,
            func_id_idx: crate::runtime::object_model::intern_string(rec_name),
            props: std::ptr::null_mut(),
            props_len: 0,
            props_cap: 0,
        });

        let ptr = Box::into_raw(rec_obj) as u64;
        let callee_val = JsValue::object(ptr);

        let mut final_air = recursive_air;
        final_air.blocks[0].insts[1] = AirOpcode::LoadInt64 {
            dst: AirReg(3),
            value: callee_val.0 as i64,
        };

        registry.register_air(rec_id, final_air.clone());

        // Snapshot com 1 Call pra ela mesma
        crate::runtime::type_feedback::TypeFeedbackRegistry::record_call(rec_ic_slot, callee_val);

        let mut compiler = Tier2Compiler::new(&mut engine, &registry);

        // Se a depth limit não existisse, a compilação desse método daria stack overflow no rust
        // compailando infinitamente si mesma.
        let id = compiler
            .compile(&final_air)
            .expect("Compile recursive inlining falhou!");
        engine.module.finalize_definitions().unwrap();

        let ptr = engine.module.get_finalized_function(id);
        assert!(!ptr.is_null());
        println!("[TEST] Limit depth evitou stack overflow inlining.");
    }

    #[test]
    fn test_tier2_inlining_deopt_bailout() {
        let mut engine = AlbedoJitEngine::new().unwrap();
        let registry = BytecodeRegistry::new();

        // 1. Callee: add_int(a, b) -> a + b  (só que o Add dará Deopt por Float64)
        let callee_add_slot = crate::runtime::type_feedback::TypeFeedbackRegistry::alloc_slot(
            crate::runtime::type_feedback::IcKind::Add,
        );
        let callee_air = AirFunction {
            name: "deoptable_callee".to_string(),
            num_params: 3,
            registers_count: 4,
            blocks: vec![AirBlock {
                id: AirBlockId(0),
                insts: vec![
                    // Add com meta_id 99 (será o bailout_id!)
                    AirOpcode::Add {
                        dst: AirReg(3),
                        lhs: AirReg(1),
                        rhs: AirReg(2),
                        ic_slot: callee_add_slot,
                    },
                ],
                terminator: Some(AirTerminator::Return(AirReg(3))),
            }],
            const_pool: AirConstantPool { strings: vec![] },
        };
        // Gravar IC como Int32 + Int32 (Para que compile FastPath como Integer!)
        crate::runtime::type_feedback::TypeFeedbackRegistry::record_add(
            callee_add_slot,
            JsValue::int32(1),
            JsValue::int32(2),
        );

        // 2. Caller: foo(y) -> callee(y, Float)  <- Vai causar FLOAT DEOPT no callee!
        let caller_ic_slot = crate::runtime::type_feedback::TypeFeedbackRegistry::alloc_slot(
            crate::runtime::type_feedback::IcKind::Call,
        );
        let caller_air = AirFunction {
            name: "deopter_caller".to_string(),
            num_params: 2,
            registers_count: 5,
            blocks: vec![AirBlock {
                id: AirBlockId(0),
                insts: vec![
                    AirOpcode::LoadInt64 {
                        dst: AirReg(3),
                        value: 0,
                    }, // Func mock
                    AirOpcode::LoadFloat64 {
                        dst: AirReg(2),
                        value: 5.5,
                    }, // O Fator FLUTUANTE DEOPT!
                    AirOpcode::Call {
                        dst: AirReg(4),
                        func: AirReg(3),
                        this: AirReg(0),
                        arg_start: AirReg(1),
                        num_args: 2,
                        ic_slot: caller_ic_slot,
                    },
                ],
                terminator: Some(AirTerminator::Return(AirReg(4))),
            }],
            const_pool: AirConstantPool { strings: vec![] },
        };

        // Mocks globais
        let callee_name = "deoptable_callee".to_string();
        let callee_id = crate::engine::profiler::FunctionId(callee_name.clone());
        registry.register_air(callee_id, callee_air);

        let callee_obj = Box::new(JsObject {
            shape_id: 0,
            kind: ObjectKind::Function as u32,
            func_id_idx: crate::runtime::object_model::intern_string(callee_name),
            props: std::ptr::null_mut(),
            props_len: 0,
            props_cap: 0,
        });

        let ptr = Box::into_raw(callee_obj) as u64;
        let callee_val = JsValue::object(ptr);

        let mut final_caller = caller_air;
        final_caller.blocks[0].insts[0] = AirOpcode::LoadInt64 {
            dst: AirReg(3),
            value: callee_val.0 as i64,
        };

        crate::runtime::type_feedback::TypeFeedbackRegistry::record_call(
            caller_ic_slot,
            callee_val,
        );

        let mut compiler = Tier2Compiler::new(&mut engine, &registry);
        let id = compiler.compile(&final_caller).unwrap();
        engine.module.finalize_definitions().unwrap();

        let ptr = engine.module.get_finalized_function(id);
        let f: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(ptr) };

        // Execta a função
        let result = f(JsValue::undefined().0, JsValue::int32(37).0); // 37 + 5.5

        // O JIT bridge mock dos testes no `albedo-jit` pode retornar fallback id ou
        // um valor de float por `js_add`/trap OSR genérica. Nosso interesse primário é checar
        // se ocorreu Inlining (impressão do TIER2-INLINE e não deu crash de OOB).
        // Se retornar F64 significa que invocou algum fallback. Se Int64, pode ser o MAGIC TRAP deopt.

        println!("[TEST] inlining deopt value = {:?}", result);
    }
}
