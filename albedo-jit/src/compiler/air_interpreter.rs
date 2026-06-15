//! # AIR Interpreter (Tier 0)
//!
//! Interpreter simples para AirFunction. Usado em deopt/OSR.

use crate::bytecode::{AirFunction, AirOpcode, AirTerminator};
use crate::compiler::tier2_compiler::Tier2Compiler;
use crate::engine::jit_engine::AlbedoJitEngine;
use crate::runtime::builtins::{call_builtin, BuiltinId};
use crate::runtime::js_value::JsValue;
use crate::runtime::object_model;
use crate::runtime::runtime_helpers;
use std::collections::{HashMap, HashSet};

pub struct AirInterpreter;

impl AirInterpreter {
    pub fn execute_with_osr(
        air: &AirFunction,
        regs: &mut [JsValue],
        osr: &mut OsrManager,
    ) -> JsValue {
        Self::execute_internal(air, regs, 0, 0, Some(osr))
    }

    pub fn execute_from(
        air: &AirFunction,
        regs: &mut [JsValue],
        start_block_id: u32,
        start_inst: usize,
    ) -> JsValue {
        Self::execute_internal(air, regs, start_block_id, start_inst, None)
    }

    fn execute_internal(
        air: &AirFunction,
        regs: &mut [JsValue],
        start_block_id: u32,
        start_inst: usize,
        mut osr: Option<&mut OsrManager>,
    ) -> JsValue {
        let mut block_idx = find_block_index(air, start_block_id);
        let mut inst_idx = start_inst;
        let loop_headers = if osr.is_some() {
            compute_loop_headers(air)
        } else {
            HashSet::new()
        };

        loop {
            let block = &air.blocks[block_idx];
            if let Some(osr_ctx) = osr.as_deref_mut() {
                if loop_headers.contains(&block.id.0) {
                    if osr_ctx.bump(air, block.id.0) {
                        let spill: Vec<u64> = regs.iter().map(|v| v.0).collect();
                        println!("[OSR-TIER0] Jumping to JIT at block {}...", block.id.0);
                        let ptr = osr_ctx.get_or_compile(air, block.id.0, 0);
                        let func: extern "C" fn(u64) -> u64 = unsafe { std::mem::transmute(ptr) };
                        let res = func(spill.as_ptr() as u64);
                        println!("[OSR-TIER0] JIT return value: {:016x}", res);
                        return JsValue(res);
                    }
                }
            }
            let mut i = inst_idx;
            while i < block.insts.len() {
                match &block.insts[i] {
                    AirOpcode::LoadInt32 { dst, value } => {
                        regs[dst.0 as usize] = JsValue::int32(*value);
                    }
                    AirOpcode::LoadFloat64 { dst, value } => {
                        regs[dst.0 as usize] = JsValue::float64(*value);
                    }
                    AirOpcode::LoadInt64 { dst, value } => {
                        regs[dst.0 as usize] = JsValue(*value as u64);
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
                        let v =
                            runtime_helpers::js_add(regs[lhs.0 as usize].0, regs[rhs.0 as usize].0);
                        regs[dst.0 as usize] = JsValue(v);
                    }
                    AirOpcode::Sub { dst, lhs, rhs } => {
                        let v =
                            runtime_helpers::js_sub(regs[lhs.0 as usize].0, regs[rhs.0 as usize].0);
                        regs[dst.0 as usize] = JsValue(v);
                    }
                    AirOpcode::Mul { dst, lhs, rhs } => {
                        let v =
                            runtime_helpers::js_mul(regs[lhs.0 as usize].0, regs[rhs.0 as usize].0);
                        regs[dst.0 as usize] = JsValue(v);
                    }
                    AirOpcode::StrictEq { dst, lhs, rhs } => {
                        let v = runtime_helpers::js_strict_eq(
                            regs[lhs.0 as usize].0,
                            regs[rhs.0 as usize].0,
                        );
                        regs[dst.0 as usize] = JsValue(v);
                    }
                    AirOpcode::Eq { dst, lhs, rhs } => {
                        let v =
                            runtime_helpers::js_eq(regs[lhs.0 as usize].0, regs[rhs.0 as usize].0);
                        regs[dst.0 as usize] = JsValue(v);
                    }
                    AirOpcode::Lt { dst, lhs, rhs } => {
                        let v =
                            runtime_helpers::js_lt(regs[lhs.0 as usize].0, regs[rhs.0 as usize].0);
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
                    AirOpcode::Call {
                        dst,
                        func,
                        arg_start,
                        num_args,
                        ..
                    } => {
                        let f = regs[func.0 as usize];
                        if f.is_builtin() {
                            let id = f.as_builtin_id() as u32;
                            if let Some(bid) = BuiltinId::from_u32(id) {
                                let mut args = Vec::with_capacity(*num_args as usize);
                                for i in 0..*num_args {
                                    let reg = crate::bytecode::AirReg(arg_start.0 + i);
                                    args.push(regs[reg.0 as usize]);
                                }
                                regs[dst.0 as usize] = call_builtin(bid, &args);
                                i += 1;
                                continue;
                            }
                        }
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
                    let target_idx = find_block_index(air, target.0);
                    block_idx = target_idx;
                    inst_idx = 0;
                }
                Some(AirTerminator::JumpIf {
                    cond,
                    then_blk,
                    else_blk,
                }) => {
                    let cv = regs[cond.0 as usize];
                    let b = JsValue(runtime_helpers::js_to_bool(cv.0)).as_bool();
                    let next = if b { then_blk.0 } else { else_blk.0 };
                    let target_idx = find_block_index(air, next);
                    block_idx = target_idx;
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
    air.blocks.iter().position(|b| b.id.0 == id).unwrap_or(0)
}

fn compute_loop_headers(air: &AirFunction) -> HashSet<u32> {
    let mut headers = HashSet::new();
    let mut index_map: HashMap<u32, usize> = HashMap::new();
    for (idx, block) in air.blocks.iter().enumerate() {
        index_map.insert(block.id.0, idx);
    }

    for (from_idx, block) in air.blocks.iter().enumerate() {
        if let Some(term) = &block.terminator {
            match term {
                AirTerminator::Jump(target) => {
                    if let Some(&to_idx) = index_map.get(&target.0) {
                        if to_idx <= from_idx {
                            headers.insert(target.0);
                        }
                    }
                }
                AirTerminator::JumpIf {
                    then_blk, else_blk, ..
                } => {
                    if let Some(&to_idx) = index_map.get(&then_blk.0) {
                        if to_idx <= from_idx {
                            headers.insert(then_blk.0);
                        }
                    }
                    if let Some(&to_idx) = index_map.get(&else_blk.0) {
                        if to_idx <= from_idx {
                            headers.insert(else_blk.0);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    headers
}


pub struct OsrManager {
    engine: AlbedoJitEngine,
    threshold: u32,
    counters: HashMap<(String, u32), u32>,
    cache: HashMap<(String, u32, usize), *const u8>,
}

impl OsrManager {
    pub fn new(threshold: u32) -> Self {
        Self {
            engine: AlbedoJitEngine::new().expect("Falha ao criar JIT engine"),
            threshold,
            counters: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    pub fn bump(&mut self, air: &AirFunction, block_id: u32) -> bool {
        let key = (air.name.clone(), block_id);
        let count = self.counters.entry(key).or_insert(0);
        *count += 1;
        *count == self.threshold
    }

    pub fn get_or_compile(
        &mut self,
        air: &AirFunction,
        block_id: u32,
        inst_index: usize,
    ) -> *const u8 {
        let key = (air.name.clone(), block_id, inst_index);
        if let Some(ptr) = self.cache.get(&key) {
            return *ptr;
        }
        let dummy_registry = crate::engine::jit_bridge::BytecodeRegistry::new();
        let mut compiler = Tier2Compiler::new(&mut self.engine, &dummy_registry);
        let ptr = compiler
            .compile_osr(air, block_id, inst_index)
            .expect("OSR compile falhou");
        self.cache.insert(key, ptr);
        ptr
    }

    pub fn compiled_count(&self) -> usize {
        self.cache.len()
    }
}
