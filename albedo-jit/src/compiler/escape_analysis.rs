//! # Escape Analysis for Albedo AIR
//!
//! Conservative v1 pipeline:
//! - classify allocations as Escaping / StackOnly / ScalarReplaceable
//! - expose explicit scalar and stack candidates for Tier2

use crate::bytecode::{AirFunction, AirOpcode, AirReg, AirTerminator};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EscapeStatus {
    Escaping,
    StackOnly,
    ScalarReplaceable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EscapeReason {
    Returned,
    CallArgument,
    StoredIntoUnknownObject,
    StoredIntoEscapingObject,
    UnknownUse,
    DynamicProperty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocationKind {
    Object,
    Array,
}

#[derive(Debug, Clone)]
pub struct ScalarProperty {
    pub prop_id: u32,
    pub prop_reg: AirReg,
    pub is_writable: bool,
}

#[derive(Debug, Clone)]
pub struct ScalarCandidate {
    pub properties: Vec<ScalarProperty>,
}

#[derive(Debug, Clone)]
pub struct StackCandidate {
    pub kind: AllocationKind,
    pub num_slots: usize,
}

#[derive(Debug, Default)]
pub struct EscapeAnalysisResult {
    pub statuses: HashMap<AirReg, EscapeStatus>,
    pub reasons: HashMap<AirReg, EscapeReason>,
    pub stack_allocatable: HashSet<AirReg>,
    pub scalar_replaceable: HashSet<AirReg>,
    pub scalar_properties: HashMap<AirReg, Vec<ScalarProperty>>,
    pub scalar_candidates: HashMap<AirReg, ScalarCandidate>,
    pub stack_candidates: HashMap<AirReg, StackCandidate>,
}

#[derive(Debug, Default, Clone)]
struct PropertyUsage {
    reads: usize,
    writes: usize,
}

#[derive(Debug, Default, Clone)]
struct ObjectUsage {
    dynamic_property: bool,
    unknown_use: bool,
    prop_usage: HashMap<u32, PropertyUsage>,
    array_pushes: usize,
}

pub fn run_field_sensitive(air: &AirFunction) -> EscapeAnalysisResult {
    let mut allocations: HashMap<AirReg, AllocationKind> = HashMap::new();
    let mut usage: HashMap<AirReg, ObjectUsage> = HashMap::new();
    let mut prop_literals: HashMap<AirReg, u32> = HashMap::new();

    for block in &air.blocks {
        for inst in &block.insts {
            match *inst {
                AirOpcode::CreateObj { dst } => {
                    allocations.insert(dst, AllocationKind::Object);
                    usage.entry(dst).or_default();
                }
                AirOpcode::CreateArray { dst } => {
                    allocations.insert(dst, AllocationKind::Array);
                    usage.entry(dst).or_default();
                }
                AirOpcode::LoadString { dst, str_id } => {
                    prop_literals.insert(dst, str_id);
                }
                _ => {}
            }
        }
    }

    let mut escaping: HashSet<AirReg> = HashSet::new();
    let mut reasons: HashMap<AirReg, EscapeReason> = HashMap::new();
    let mut changed = true;

    while changed {
        changed = false;

        for block in &air.blocks {
            for inst in &block.insts {
                match *inst {
                    AirOpcode::Call {
                        this,
                        arg_start,
                        num_args,
                        ..
                    }
                    | AirOpcode::NewCall {
                        this,
                        arg_start,
                        num_args,
                        ..
                    } => {
                        if mark_escape(
                            this,
                            EscapeReason::CallArgument,
                            &allocations,
                            &mut escaping,
                            &mut reasons,
                        ) {
                            changed = true;
                        }
                        for i in 0..num_args {
                            let arg = AirReg(arg_start.0 + i);
                            if mark_escape(
                                arg,
                                EscapeReason::CallArgument,
                                &allocations,
                                &mut escaping,
                                &mut reasons,
                            ) {
                                changed = true;
                            }
                        }
                    }
                    AirOpcode::SetProp { obj, prop, value } => {
                        if let Some(obj_usage) = usage.get_mut(&obj) {
                            if let Some(prop_id) = prop_literals.get(&prop).copied() {
                                obj_usage.prop_usage.entry(prop_id).or_default().writes += 1;
                            } else {
                                obj_usage.dynamic_property = true;
                            }
                        }

                        if allocations.contains_key(&value) {
                            if !allocations.contains_key(&obj) {
                                if mark_escape(
                                    value,
                                    EscapeReason::StoredIntoUnknownObject,
                                    &allocations,
                                    &mut escaping,
                                    &mut reasons,
                                ) {
                                    changed = true;
                                }
                            } else if escaping.contains(&obj)
                                && mark_escape(
                                    value,
                                    EscapeReason::StoredIntoEscapingObject,
                                    &allocations,
                                    &mut escaping,
                                    &mut reasons,
                                )
                            {
                                changed = true;
                            }
                        }
                    }
                    AirOpcode::GetProp { obj, prop, .. } => {
                        if let Some(obj_usage) = usage.get_mut(&obj) {
                            if let Some(prop_id) = prop_literals.get(&prop).copied() {
                                obj_usage.prop_usage.entry(prop_id).or_default().reads += 1;
                            } else {
                                obj_usage.dynamic_property = true;
                            }
                        }
                    }
                    AirOpcode::ArrayPush { arr, value } => {
                        if let Some(arr_usage) = usage.get_mut(&arr) {
                            arr_usage.array_pushes += 1;
                        }
                        if allocations.contains_key(&value) {
                            if !allocations.contains_key(&arr) {
                                if mark_escape(
                                    value,
                                    EscapeReason::StoredIntoUnknownObject,
                                    &allocations,
                                    &mut escaping,
                                    &mut reasons,
                                ) {
                                    changed = true;
                                }
                            } else if escaping.contains(&arr)
                                && mark_escape(
                                    value,
                                    EscapeReason::StoredIntoEscapingObject,
                                    &allocations,
                                    &mut escaping,
                                    &mut reasons,
                                )
                            {
                                changed = true;
                            }
                        }
                    }
                    AirOpcode::Move { dst, src } => {
                        if allocations.contains_key(&dst) && escaping.contains(&src) {
                            if mark_escape(
                                dst,
                                EscapeReason::UnknownUse,
                                &allocations,
                                &mut escaping,
                                &mut reasons,
                            ) {
                                changed = true;
                            }
                        }
                        if allocations.contains_key(&src) && escaping.contains(&dst) {
                            if mark_escape(
                                src,
                                EscapeReason::UnknownUse,
                                &allocations,
                                &mut escaping,
                                &mut reasons,
                            ) {
                                changed = true;
                            }
                        }
                    }
                    _ => {
                        for op in inst.operands() {
                            if !allocations.contains_key(&op) {
                                continue;
                            }
                            if is_local_safe_use(inst, op) {
                                continue;
                            }
                            if let Some(obj_usage) = usage.get_mut(&op) {
                                obj_usage.unknown_use = true;
                            }
                            if mark_escape(
                                op,
                                EscapeReason::UnknownUse,
                                &allocations,
                                &mut escaping,
                                &mut reasons,
                            ) {
                                changed = true;
                            }
                        }
                    }
                }
            }

            if let Some(AirTerminator::Return(reg)) = block.terminator {
                if mark_escape(
                    reg,
                    EscapeReason::Returned,
                    &allocations,
                    &mut escaping,
                    &mut reasons,
                ) {
                    changed = true;
                }
            }
        }
    }

    let mut result = EscapeAnalysisResult::default();

    for (obj_reg, kind) in allocations {
        if escaping.contains(&obj_reg) {
            result.statuses.insert(obj_reg, EscapeStatus::Escaping);
            continue;
        }

        let obj_usage = usage.get(&obj_reg).cloned().unwrap_or_default();
        let scalar_ok = matches!(kind, AllocationKind::Object)
            && !obj_usage.dynamic_property
            && !obj_usage.unknown_use
            && !obj_usage.prop_usage.is_empty();

        if scalar_ok {
            let mut properties: Vec<ScalarProperty> = obj_usage
                .prop_usage
                .iter()
                .map(|(prop_id, p)| ScalarProperty {
                    prop_id: *prop_id,
                    prop_reg: AirReg(0),
                    is_writable: p.writes > 0,
                })
                .collect();
            properties.sort_by_key(|p| p.prop_id);
            result
                .statuses
                .insert(obj_reg, EscapeStatus::ScalarReplaceable);
            result.scalar_replaceable.insert(obj_reg);
            result.scalar_properties.insert(obj_reg, properties.clone());
            result
                .scalar_candidates
                .insert(obj_reg, ScalarCandidate { properties });
        } else {
            if obj_usage.dynamic_property {
                reasons
                    .entry(obj_reg)
                    .or_insert(EscapeReason::DynamicProperty);
            }
            result.statuses.insert(obj_reg, EscapeStatus::StackOnly);
            result.stack_allocatable.insert(obj_reg);
            let num_slots = if matches!(kind, AllocationKind::Array) {
                obj_usage.array_pushes.max(4)
            } else {
                obj_usage.prop_usage.len().max(4)
            };
            result
                .stack_candidates
                .insert(obj_reg, StackCandidate { kind, num_slots });
        }
    }

    result.reasons = reasons;
    result
}

fn mark_escape(
    reg: AirReg,
    reason: EscapeReason,
    allocations: &HashMap<AirReg, AllocationKind>,
    escaping: &mut HashSet<AirReg>,
    reasons: &mut HashMap<AirReg, EscapeReason>,
) -> bool {
    if !allocations.contains_key(&reg) {
        return false;
    }
    let inserted = escaping.insert(reg);
    if inserted {
        reasons.entry(reg).or_insert(reason);
    }
    inserted
}

fn is_local_safe_use(inst: &AirOpcode, op: AirReg) -> bool {
    match *inst {
        AirOpcode::SetProp { obj, .. } if obj == op => true,
        AirOpcode::GetProp { obj, .. } if obj == op => true,
        AirOpcode::ArrayPush { arr, .. } if arr == op => true,
        AirOpcode::Move { src, .. } if src == op => true,
        _ => false,
    }
}
