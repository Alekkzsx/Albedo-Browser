//! # Scalar Replacement for AlbedoJIT
//!
//! Conservative SROA v1:
//! - only static string properties
//! - only predictable local object uses

use crate::bytecode::{AirFunction, AirOpcode, AirReg};
use crate::compiler::escape_analysis::ScalarProperty;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarTransformResult {
    Applied,
    RejectedCoverage,
}

#[derive(Debug)]
pub struct ScalarReplacer {
    scalar_map: HashMap<AirReg, HashMap<u32, AirReg>>,
    next_reg: AirReg,
}

impl ScalarReplacer {
    pub fn new(start_reg: AirReg) -> Self {
        Self {
            scalar_map: HashMap::new(),
            next_reg: start_reg,
        }
    }

    pub fn replace_object_with_scalars(
        &mut self,
        air: &mut AirFunction,
        obj_reg: AirReg,
        properties: &[ScalarProperty],
    ) -> ScalarTransformResult {
        let prop_literal_map = self.build_prop_literal_map(air);
        let mut prop_to_reg = HashMap::new();
        for prop in properties {
            let reg = self.next_reg;
            self.next_reg.0 += 1;
            prop_to_reg.insert(prop.prop_id, reg);
        }

        if !self.is_coverage_complete(air, obj_reg, &prop_literal_map, &prop_to_reg) {
            return ScalarTransformResult::RejectedCoverage;
        }

        self.scalar_map.insert(obj_reg, prop_to_reg.clone());
        self.initialize_scalar_regs(air, &prop_to_reg);
        self.transform_air(air, obj_reg, &prop_literal_map, &prop_to_reg);
        ScalarTransformResult::Applied
    }

    fn build_prop_literal_map(&self, air: &AirFunction) -> HashMap<AirReg, u32> {
        let mut map = HashMap::new();
        for block in &air.blocks {
            for inst in &block.insts {
                if let AirOpcode::LoadString { dst, str_id } = *inst {
                    map.insert(dst, str_id);
                }
            }
        }
        map
    }

    fn is_coverage_complete(
        &self,
        air: &AirFunction,
        obj_reg: AirReg,
        prop_literal_map: &HashMap<AirReg, u32>,
        prop_to_reg: &HashMap<u32, AirReg>,
    ) -> bool {
        for block in &air.blocks {
            for inst in &block.insts {
                match *inst {
                    AirOpcode::CreateObj { dst } | AirOpcode::CreateArray { dst } if dst == obj_reg => {}
                    AirOpcode::SetProp { obj, prop, .. } if obj == obj_reg => {
                        let Some(prop_id) = prop_literal_map.get(&prop).copied() else {
                            return false;
                        };
                        if !prop_to_reg.contains_key(&prop_id) {
                            return false;
                        }
                    }
                    AirOpcode::GetProp { obj, prop, .. } if obj == obj_reg => {
                        let Some(prop_id) = prop_literal_map.get(&prop).copied() else {
                            return false;
                        };
                        if !prop_to_reg.contains_key(&prop_id) {
                            return false;
                        }
                    }
                    AirOpcode::Move { dst, .. } if dst == obj_reg => return false,
                    _ => {
                        for op in inst.operands() {
                            if op == obj_reg {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn initialize_scalar_regs(&self, air: &mut AirFunction, prop_to_reg: &HashMap<u32, AirReg>) {
        if air.blocks.is_empty() {
            return;
        }
        let entry = &mut air.blocks[0];
        let mut init = Vec::new();
        let mut regs: Vec<AirReg> = prop_to_reg.values().copied().collect();
        regs.sort_by_key(|r| r.0);
        for reg in regs {
            init.push(AirOpcode::LoadUndefined { dst: reg });
        }
        if !init.is_empty() {
            let mut new_insts = init;
            new_insts.extend(entry.insts.clone());
            entry.insts = new_insts;
        }
    }

    fn transform_air(
        &self,
        air: &mut AirFunction,
        obj_reg: AirReg,
        prop_literal_map: &HashMap<AirReg, u32>,
        prop_to_reg: &HashMap<u32, AirReg>,
    ) {
        for block in &mut air.blocks {
            block.insts.retain(|inst| {
                !matches!(
                    *inst,
                    AirOpcode::CreateObj { dst } | AirOpcode::CreateArray { dst } if dst == obj_reg
                )
            });

            for inst in &mut block.insts {
                match *inst {
                    AirOpcode::SetProp { obj, prop, value } if obj == obj_reg => {
                        if let Some(prop_id) = prop_literal_map.get(&prop).copied() {
                            if let Some(dst_reg) = prop_to_reg.get(&prop_id).copied() {
                                *inst = AirOpcode::Move {
                                    dst: dst_reg,
                                    src: value,
                                };
                            }
                        }
                    }
                    AirOpcode::GetProp { dst, obj, prop, .. } if obj == obj_reg => {
                        if let Some(prop_id) = prop_literal_map.get(&prop).copied() {
                            if let Some(src_reg) = prop_to_reg.get(&prop_id).copied() {
                                *inst = AirOpcode::Move { dst, src: src_reg };
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn get_scalar_reg(&self, obj_reg: AirReg, prop_id: u32) -> Option<AirReg> {
        self.scalar_map
            .get(&obj_reg)
            .and_then(|props| props.get(&prop_id).copied())
    }

    pub fn get_next_reg(&self) -> AirReg {
        self.next_reg
    }
}

