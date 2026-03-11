//! # AIR Interpreter (Tier 0)
//!
//! Interpreter simples para AirFunction. Usado em deopt/OSR.

use crate::bytecode::{AirFunction, AirOpcode, AirTerminator};
use crate::js_value::JsValue;
use crate::object_model;
use crate::runtime_helpers;

pub struct AirInterpreter;

impl AirInterpreter {
    pub fn execute_from(
        air: &AirFunction,
        regs: &mut [JsValue],
        start_block_id: u32,
        start_inst: usize,
    ) -> JsValue {
        let mut block_idx = find_block_index(air, start_block_id);
        let mut inst_idx = start_inst;

        loop {
            let block = &air.blocks[block_idx];
            let mut i = inst_idx;
            while i < block.insts.len() {
                match &block.insts[i] {
                    AirOpcode::LoadInt32 { dst, value } => {
                        regs[dst.0 as usize] = JsValue::int32(*value);
                    }
                    AirOpcode::LoadFloat64 { dst, value } => {
                        regs[dst.0 as usize] = JsValue::float64(*value);
                    }
                    AirOpcode::LoadBool { dst, value } => {
                        regs[dst.0 as usize] = JsValue::bool(*value);
                    }
                    AirOpcode::LoadUndefined { dst } => {
                        regs[dst.0 as usize] = JsValue::undefined();
                    }
                    AirOpcode::LoadNull { dst } => {
                        regs[dst.0 as usize] = JsValue::null();
                    }
                    AirOpcode::LoadString { dst, str_id } => {
                        regs[dst.0 as usize] = JsValue::string(*str_id as u64);
                    }
                    AirOpcode::Move { dst, src } => {
                        regs[dst.0 as usize] = regs[src.0 as usize];
                    }
                    AirOpcode::Add { dst, lhs, rhs, .. } => {
                        let v = runtime_helpers::js_add(regs[lhs.0 as usize].0, regs[rhs.0 as usize].0);
                        regs[dst.0 as usize] = JsValue(v);
                    }
                    AirOpcode::Sub { dst, lhs, rhs } => {
                        let v = runtime_helpers::js_sub(regs[lhs.0 as usize].0, regs[rhs.0 as usize].0);
                        regs[dst.0 as usize] = JsValue(v);
                    }
                    AirOpcode::Mul { dst, lhs, rhs } => {
                        let v = runtime_helpers::js_mul(regs[lhs.0 as usize].0, regs[rhs.0 as usize].0);
                        regs[dst.0 as usize] = JsValue(v);
                    }
                    AirOpcode::StrictEq { dst, lhs, rhs } => {
                        let v = runtime_helpers::js_strict_eq(regs[lhs.0 as usize].0, regs[rhs.0 as usize].0);
                        regs[dst.0 as usize] = JsValue(v);
                    }
                    AirOpcode::Eq { dst, lhs, rhs } => {
                        let v = runtime_helpers::js_eq(regs[lhs.0 as usize].0, regs[rhs.0 as usize].0);
                        regs[dst.0 as usize] = JsValue(v);
                    }
                    AirOpcode::Lt { dst, lhs, rhs } => {
                        let v = runtime_helpers::js_lt(regs[lhs.0 as usize].0, regs[rhs.0 as usize].0);
                        regs[dst.0 as usize] = JsValue(v);
                    }
                    AirOpcode::GetProp { dst, obj, prop, .. } => {
                        let o = regs[obj.0 as usize];
                        let p = regs[prop.0 as usize];
                        regs[dst.0 as usize] = object_model::get_prop(o, p);
                    }
                    AirOpcode::SetProp { obj, prop, value } => {
                        let o = regs[obj.0 as usize];
                        let p = regs[prop.0 as usize];
                        let v = regs[value.0 as usize];
                        object_model::set_prop(o, p, v);
                    }
                    AirOpcode::CreateObj { dst } => {
                        regs[dst.0 as usize] = object_model::alloc_object();
                    }
                    AirOpcode::CreateArray { dst } => {
                        regs[dst.0 as usize] = object_model::alloc_array();
                    }
                    AirOpcode::Call { dst, func, .. } => {
                        let f = regs[func.0 as usize];
                        // Stub por enquanto: sem builtins integrados aqui.
                        let _ = f;
                        regs[dst.0 as usize] = JsValue::undefined();
                    }
                    _ => {
                        // Opcodes não suportados no Tier0 v1
                    }
                }
                i += 1;
            }

            match &block.terminator {
                Some(AirTerminator::Return(reg)) => {
                    return regs[reg.0 as usize];
                }
                Some(AirTerminator::Jump(target)) => {
                    block_idx = find_block_index(air, target.0);
                    inst_idx = 0;
                }
                Some(AirTerminator::JumpIf { cond, then_blk, else_blk }) => {
                    let cv = regs[cond.0 as usize];
                    let b = JsValue(runtime_helpers::js_to_bool(cv.0)).as_bool();
                    block_idx = if b {
                        find_block_index(air, then_blk.0)
                    } else {
                        find_block_index(air, else_blk.0)
                    };
                    inst_idx = 0;
                }
                None => {
                    return JsValue::undefined();
                }
            }
        }
    }
}

fn find_block_index(air: &AirFunction, id: u32) -> usize {
    air.blocks
        .iter()
        .position(|b| b.id.0 == id)
        .unwrap_or(0)
}

