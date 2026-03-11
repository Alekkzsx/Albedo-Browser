//! # Deoptimization (Tier 2 -> Tier 0)
//!
//! Registra metadados de deopt e expõe o bailout usado pelo JIT.

use std::sync::{Arc, OnceLock};
use parking_lot::RwLock;

use crate::air_interpreter::AirInterpreter;
use crate::bytecode::AirFunction;
use crate::js_value::JsValue;

#[derive(Debug, Clone)]
pub struct DeoptPoint {
    pub block_id: u32,
    pub inst_index: u32,
}

#[derive(Debug)]
pub struct DeoptMeta {
    pub air: Arc<AirFunction>,
    pub points: Vec<DeoptPoint>,
    pub regs_count: u32,
    pub spill_stride: u32,
    pub spill_size: u32,
}

impl DeoptMeta {
    pub fn reg_offset(&self, reg_index: u32) -> u32 {
        reg_index * self.spill_stride
    }
}

static DEOPT_REGISTRY: OnceLock<RwLock<Vec<Arc<DeoptMeta>>>> = OnceLock::new();

fn registry() -> &'static RwLock<Vec<Arc<DeoptMeta>>> {
    DEOPT_REGISTRY.get_or_init(|| RwLock::new(Vec::new()))
}

pub fn register_meta(meta: DeoptMeta) -> u32 {
    let mut reg = registry().write();
    let id = reg.len() as u32;
    reg.push(Arc::new(meta));
    id
}

pub fn get_meta(id: u32) -> Arc<DeoptMeta> {
    registry()
        .read()
        .get(id as usize)
        .cloned()
        .expect("DeoptMeta inválido")
}

/// Bailout: reconstrói o frame e continua no Tier 0.
#[no_mangle]
pub extern "C" fn js_deopt_bailout(meta_id: u64, deopt_id: u64, spill_ptr: u64) -> u64 {
    let meta = get_meta(meta_id as u32);
    let point = meta.points.get(deopt_id as usize).expect("DeoptPoint inválido");
    let regs_count = meta.regs_count as usize;
    let spill = unsafe { std::slice::from_raw_parts(spill_ptr as *const u64, regs_count) };

    let mut regs: Vec<JsValue> = spill.iter().map(|v| JsValue(*v)).collect();
    let result = AirInterpreter::execute_from(&meta.air, &mut regs, point.block_id, point.inst_index as usize);
    result.0
}
