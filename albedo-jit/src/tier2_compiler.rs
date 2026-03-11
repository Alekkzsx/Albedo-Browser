//! # Tier 2 Compiler (Type Specialization)
//!
//! Usa type feedback coletado via Inline Caches para gerar código especializado
//! (IADD/FADD e acesso direto por offset de propriedade).

use cranelift_codegen::ir::types::{I32, I64, F64};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, StackSlot, StackSlotData, StackSlotKind};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::HashMap;
use std::sync::Arc;

use crate::bytecode::{AirFunction, AirOpcode, AirTerminator};
use crate::jit_engine::{AlbedoJitEngine, JitError};
use crate::js_value::{JsValue, TAG_INT32, TAG_MASK, TAG_MIN, PAYLOAD_MASK, FLOAT_NAN};
use crate::object_model::{JSOBJ_SHAPE_OFFSET, JSOBJ_PROPS_OFFSET};
use crate::type_feedback::{AddFeedbackSnapshot, GetPropFeedbackSnapshot, IcState, TypeFeedbackRegistry, TypePair, ValueType};
use crate::deopt::{DeoptMeta, DeoptPoint, register_meta};

const MIN_FEEDBACK_SAMPLES: u64 = 32;

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
        let func_id = self.engine.module.declare_function(
            &func_name,
            Linkage::Export,
            &sig,
        )?;

        // Pré-mapeamento de deopt points (1 por instrução AIR)
        let (deopt_points, deopt_map) = build_deopt_points(air);
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
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_add_ic", 3, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_sub", 2, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_mul", 2, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_strict_eq", 2, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_eq", 2, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_lt", 2, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_to_bool", 1, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_get_prop_ic", 3, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_call_ic", 2, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_create_obj", 0, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_create_array", 0, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_set_prop", 3, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_deopt_bailout", 3, &mut ext_funcs)?;
        Self::declare_runtime_helper(&mut self.engine.module, &mut builder, "js_unimplemented", 0, &mut ext_funcs)?;

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
                    AirOpcode::Add { dst, lhs, rhs, ic_slot } => {
                        let a = builder.use_var(vars[lhs.0 as usize]);
                        let b = builder.use_var(vars[rhs.0 as usize]);
                        let slot = *ic_slot;
                        if let Some(snap) = TypeFeedbackRegistry::add_snapshot(slot) {
                            if let Some(res) = Self::emit_specialized_add(
                                &mut builder,
                                &ext_funcs,
                                a,
                                b,
                                slot,
                                &snap,
                                meta_id,
                                deopt_id,
                                spill_slot,
                                &vars,
                            ) {
                                builder.def_var(vars[dst.0 as usize], res);
                                continue;
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
                        // Tentar specialization int32
                        if let Some(res) = Self::emit_sub_int32(&mut builder, &ext_funcs, a, b) {
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
                        // Tentar specialization int32
                        if let Some(res) = Self::emit_mul_int32(&mut builder, &ext_funcs, a, b) {
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
                    AirOpcode::GetProp { dst, obj, prop, ic_slot } => {
                        let o = builder.use_var(vars[obj.0 as usize]);
                        let p = builder.use_var(vars[prop.0 as usize]);
                        let slot = *ic_slot;
                        if let Some(snap) = TypeFeedbackRegistry::get_prop_snapshot(slot) {
                            if let Some(res) = Self::emit_specialized_get_prop(
                                &mut builder,
                                &ext_funcs,
                                o,
                                p,
                                slot,
                                &snap,
                                meta_id,
                                deopt_id,
                                spill_slot,
                                &vars,
                            ) {
                                builder.def_var(vars[dst.0 as usize], res);
                                continue;
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
                    AirOpcode::Call { dst, func, arg_start: _, num_args: _, ic_slot } => {
                        let f = builder.use_var(vars[func.0 as usize]);
                        let slot_val = builder.ins().iconst(I64, *ic_slot as i64);
                        let func_ref = *ext_funcs.get("js_call_ic").unwrap();
                        let call = builder.ins().call(func_ref, &[f, slot_val]);
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
                        let func_ref = *ext_funcs.get("js_unimplemented").unwrap();
                        let call = builder.ins().call(func_ref, &[]);
                        let _res = builder.inst_results(call)[0];
                        println!("[Tier2] Warning: Unsupported Opcode {:?} - fallback to undefined", inst);
                    }
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
                    AirTerminator::JumpIf { cond, then_blk, else_blk } => {
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

            builder.seal_block(cl_block);
        }

        builder.finalize();

        self.engine
            .module
            .define_function(func_id, &mut ctx)
            .map_err(|e| JitError::Compilation(e.to_string()))?;

        self.engine.module.clear_context(&mut ctx);
        Ok(func_id)
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
            TypePair(ValueType::Int32, ValueType::Int32) => {
                Some(Self::emit_add_int32(builder, ext_funcs, a, b, slot, meta_id, deopt_id, spill_slot, vars))
            }
            TypePair(ValueType::Float64, ValueType::Float64) => {
                Some(Self::emit_add_float64(builder, ext_funcs, a, b, slot, meta_id, deopt_id, spill_slot, vars))
            }
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
        builder.ins().brif(both_ok, fast_block, &[], slow_block, &[]);

        builder.switch_to_block(fast_block);
        let a_payload = builder.ins().band_imm(a, 0xFFFF_FFFF);
        let b_payload = builder.ins().band_imm(b, 0xFFFF_FFFF);
        let a_i32 = builder.ins().ireduce(I32, a_payload);
        let b_i32 = builder.ins().ireduce(I32, b_payload);
        let (sum, overflow) = builder.ins().sadd_overflow(a_i32, b_i32);
        let overflowed = builder.ins().icmp_imm(IntCC::NotEqual, overflow, 0);
        builder.ins().brif(overflowed, slow_block, &[], fast_ok_block, &[]);

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
        builder.ins().brif(both_ok, fast_block, &[], slow_block, &[]);

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
        let tag_obj = builder.ins().iconst(I64, crate::js_value::TAG_OBJECT as i64);
        let obj_tag = builder.ins().band(obj, tag_mask);
        let is_obj = builder.ins().icmp(IntCC::Equal, obj_tag, tag_obj);
        builder.ins().brif(is_obj, obj_block, &[], slow_block, &[]);

        builder.switch_to_block(obj_block);
        // Guard: prop id match (string id)
        let tag_str = builder.ins().iconst(I64, crate::js_value::TAG_STRING as i64);
        let prop_tag = builder.ins().band(prop, tag_mask);
        let is_str = builder.ins().icmp(IntCC::Equal, prop_tag, tag_str);
        builder.ins().brif(is_str, prop_block, &[], slow_block, &[]);

        builder.switch_to_block(prop_block);
        let prop_expected = builder.ins().iconst(I64, mono.prop_id as i64);
        let prop_id = builder.ins().band_imm(prop, PAYLOAD_MASK as i64);
        let prop_ok = builder.ins().icmp(IntCC::Equal, prop_id, prop_expected);
        builder.ins().brif(prop_ok, shape_block, &[], slow_block, &[]);

        builder.switch_to_block(shape_block);
        // Object ptr
        let payload_mask = builder.ins().iconst(I64, PAYLOAD_MASK as i64);
        let obj_ptr = builder.ins().band(obj, payload_mask);

        let shape_ptr = builder.ins().iadd_imm(obj_ptr, JSOBJ_SHAPE_OFFSET as i64);
        let shape = builder.ins().load(I64, MemFlags::new(), shape_ptr, 0);
        let expected_shape = builder.ins().iconst(I64, mono.shape_id as i64);
        let shape_ok = builder.ins().icmp(IntCC::Equal, shape, expected_shape);
        builder.ins().brif(shape_ok, load_block, &[], slow_block, &[]);

        builder.switch_to_block(load_block);
        let props_ptr = builder.ins().load(I64, MemFlags::new(), obj_ptr, JSOBJ_PROPS_OFFSET as i32);
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
        let call = builder.ins().call(func_ref, &[meta_val, deopt_val, spill_ptr]);
        builder.inst_results(call)[0]
    }

    /// ISUB (int32) com guard de tipo e overflow.
    fn emit_sub_int32(
        builder: &mut FunctionBuilder,
        ext_funcs: &HashMap<String, cranelift_codegen::ir::FuncRef>,
        a: cranelift_codegen::ir::Value,
        b: cranelift_codegen::ir::Value,
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
        builder.ins().brif(both_ok, fast_block, &[], slow_block, &[]);

        builder.switch_to_block(fast_block);
        let a_i32 = builder.ins().ireduce(I32, a);
        let b_i32 = builder.ins().ireduce(I32, b);
        let (diff, overflow) = builder.ins().ssub_overflow(a_i32, b_i32);
        let overflowed = builder.ins().icmp_imm(IntCC::NotEqual, overflow, 0);
        builder.ins().brif(overflowed, slow_block, &[], fast_ok_block, &[]);

        builder.switch_to_block(fast_ok_block);
        let diff_i64 = builder.ins().uextend(I64, diff);
        let boxed = builder.ins().bor(diff_i64, tag_int);
        let cont_args = [boxed.into()];
        builder.ins().jump(cont_block, &cont_args);

        builder.switch_to_block(slow_block);
        let func_ref = *ext_funcs.get("js_sub").unwrap();
        let call = builder.ins().call(func_ref, &[a, b]);
        let res = builder.inst_results(call)[0];
        let cont_args = [res.into()];
        builder.ins().jump(cont_block, &cont_args);

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
        builder.ins().brif(both_ok, fast_block, &[], slow_block, &[]);

        builder.switch_to_block(fast_block);
        let a_i32 = builder.ins().ireduce(I32, a);
        let b_i32 = builder.ins().ireduce(I32, b);
        let (prod, overflow) = builder.ins().smul_overflow(a_i32, b_i32);
        let overflowed = builder.ins().icmp_imm(IntCC::NotEqual, overflow, 0);
        builder.ins().brif(overflowed, slow_block, &[], fast_ok_block, &[]);

        builder.switch_to_block(fast_ok_block);
        let prod_i64 = builder.ins().uextend(I64, prod);
        let boxed = builder.ins().bor(prod_i64, tag_int);
        let cont_args = [boxed.into()];
        builder.ins().jump(cont_block, &cont_args);

        builder.switch_to_block(slow_block);
        let func_ref = *ext_funcs.get("js_mul").unwrap();
        let call = builder.ins().call(func_ref, &[a, b]);
        let res = builder.inst_results(call)[0];
        let cont_args = [res.into()];
        builder.ins().jump(cont_block, &cont_args);

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
}

fn build_deopt_points(
    air: &AirFunction,
) -> (Vec<DeoptPoint>, HashMap<(u32, usize), u32>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{AirBlock, AirOpcode, AirReg, AirTerminator, AirConstantPool};
    use crate::bytecode::opcodes::AirBlockId;
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
        let air = make_binary_air("sub_t2", AirOpcode::Sub {
            dst: AirReg(2), lhs: AirReg(0), rhs: AirReg(1),
        });
        let result = tier2_run_2(&air, JsValue::int32(10), JsValue::int32(3));
        assert_eq!(result.as_int32(), 7);
    }

    #[test]
    fn test_tier2_mul_int32_specialization() {
        let air = make_binary_air("mul_t2", AirOpcode::Mul {
            dst: AirReg(2), lhs: AirReg(0), rhs: AirReg(1),
        });
        let result = tier2_run_2(&air, JsValue::int32(6), JsValue::int32(7));
        assert_eq!(result.as_int32(), 42);
    }

    #[test]
    fn test_tier2_sub_deopt_bailout() {
        // Float64 tipos → guard int32 falha → slow path via js_sub
        let air = make_binary_air("sub_deopt", AirOpcode::Sub {
            dst: AirReg(2), lhs: AirReg(0), rhs: AirReg(1),
        });
        let result = tier2_run_2(&air, JsValue::float64(10.5), JsValue::float64(3.5));
        assert_eq!(result.as_float64(), 7.0);
    }

    #[test]
    fn test_tier2_mul_deopt_bailout() {
        // Float64 tipos → guard int32 falha → slow path via js_mul
        let air = make_binary_air("mul_deopt", AirOpcode::Mul {
            dst: AirReg(2), lhs: AirReg(0), rhs: AirReg(1),
        });
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
                insts: vec![
                    AirOpcode::Add {
                        dst: AirReg(2),
                        lhs: AirReg(0),
                        rhs: AirReg(1),
                        ic_slot: 999,
                    },
                ],
                terminator: Some(AirTerminator::Return(AirReg(2))),
            }],
            const_pool: AirConstantPool::default(),
        };
        let result = tier2_run_2(&air, JsValue::int32(20), JsValue::int32(22));
        assert_eq!(result.as_int32(), 42);
    }
}
