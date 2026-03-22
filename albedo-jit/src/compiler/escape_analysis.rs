//! # Escape Analysis for Albedo AIR
//!
//! Este módulo identifica objetos e arrays que não "escapam" da função atual,
//! permitindo que sejam alocados no stack em vez do heap.

use crate::bytecode::{AirFunction, AirOpcode, AirReg, AirTerminator};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeStatus {
    /// O objeto não escapa da função e pode ser alocado no stack.
    NonEscaping,
    /// O objeto escapa via Return, Call ou ao ser armazenado em um objeto que escapa.
    Escaping,
}

/// Resultado da análise de escape para uma função.
pub struct EscapeAnalysisResult {
    pub statuses: HashMap<AirReg, EscapeStatus>,
}

pub struct EscapeAnalysis;

impl EscapeAnalysis {
    /// Executa a análise de escape em uma AirFunction.
    pub fn run(air: &AirFunction) -> EscapeAnalysisResult {
        let mut statuses = HashMap::new();
        let mut allocations = HashSet::new();

        // 1. Identificar todas as alocações (CreateObj, CreateArray)
        for block in &air.blocks {
            for inst in &block.insts {
                match inst.opcode {
                    AirOpcode::CreateObj | AirOpcode::CreateArray => {
                        if let Some(dst) = inst.dst_reg() {
                            statuses.insert(dst, EscapeStatus::NonEscaping);
                            allocations.insert(dst);
                        }
                    }
                    _ => {}
                }
            }
        }

        // 2. Análise de propagação (Iterativa até ponto fixo)
        let mut changed = true;
        while changed {
            changed = false;

            for block in &air.blocks {
                for inst in &block.insts {
                    match &inst {
                        AirOpcode::Call { func: _, this, arg_start, num_args, .. }
                        | AirOpcode::NewCall { func: _, this, arg_start, num_args, .. } => {
                            // 'this' e argumentos de funções escapam
                            let mut args = vec![*this];
                            for i in 0..*num_args {
                                args.push(AirReg(arg_start.0 + i));
                            }
                            
                            for &reg in &args {
                                if allocations.contains(&reg) && statuses.get(&reg) == Some(&EscapeStatus::NonEscaping) {
                                    statuses.insert(reg, EscapeStatus::Escaping);
                                    changed = true;
                                }
                            }
                        }
                        AirOpcode::SetProp { obj, value, .. } => {
                            // Se o obj escapar, o valor armazenado nele também escapa
                            if statuses.get(obj) == Some(&EscapeStatus::Escaping) {
                                if allocations.contains(value) && statuses.get(value) == Some(&EscapeStatus::NonEscaping) {
                                    statuses.insert(*value, EscapeStatus::Escaping);
                                    changed = true;
                                }
                            }
                            // Se o obj não for uma alocação rastreada (ex: global, arg), o valor escapa
                            if !allocations.contains(obj) {
                                if allocations.contains(value) && statuses.get(value) == Some(&EscapeStatus::NonEscaping) {
                                    statuses.insert(*value, EscapeStatus::Escaping);
                                    changed = true;
                                }
                            }
                        }
                        AirOpcode::ArrayPush { arr, value } => {
                            // Similar ao SetProp
                            if statuses.get(arr) == Some(&EscapeStatus::Escaping) || !allocations.contains(arr) {
                                if allocations.contains(value) && statuses.get(value) == Some(&EscapeStatus::NonEscaping) {
                                    statuses.insert(*value, EscapeStatus::Escaping);
                                    changed = true;
                                }
                            }
                        }
                        AirOpcode::Move { dst, src } => {
                            // Aliasing simples: se um escapar, o outro escapa
                            if statuses.get(src) == Some(&EscapeStatus::Escaping) {
                                if allocations.contains(dst) && statuses.get(dst) == Some(&EscapeStatus::NonEscaping) {
                                    statuses.insert(*dst, EscapeStatus::Escaping);
                                    changed = true;
                                }
                            }
                            if statuses.get(dst) == Some(&EscapeStatus::Escaping) {
                                if allocations.contains(src) && statuses.get(src) == Some(&EscapeStatus::NonEscaping) {
                                    statuses.insert(*src, EscapeStatus::Escaping);
                                    changed = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Verificar terminador
                if let Some(AirTerminator::Return(reg)) = &block.terminator {
                    if allocations.contains(reg) && statuses.get(reg) == Some(&EscapeStatus::NonEscaping) {
                        statuses.insert(*reg, EscapeStatus::Escaping);
                        changed = true;
                    }
                }
            }
        }

        EscapeAnalysisResult { statuses }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{AirBlock, AirFunction, AirConstantPool};

    #[test]
    fn test_non_escaping_object() {
        let mut air = AirFunction {
            name: "test".to_string(),
            num_params: 0,
            registers_count: 2,
            blocks: vec![
                AirBlock {
                    id: crate::bytecode::AirBlockId(0),
                    insts: vec![
                        AirOpcode::CreateObj { dst: AirReg(0) },
                        AirOpcode::LoadInt32 { dst: AirReg(1), value: 42 },
                        AirOpcode::SetProp { obj: AirReg(0), prop: AirReg(1), value: AirReg(1) },
                    ],
                    terminator: Some(AirTerminator::Return(AirReg(1))),
                }
            ],
            const_pool: AirConstantPool::default(),
        };

        let res = EscapeAnalysis::run(&air);
        assert_eq!(res.statuses.get(&AirReg(0)), Some(&EscapeStatus::NonEscaping));
    }

    #[test]
    fn test_escaping_via_return() {
        let mut air = AirFunction {
            name: "test".to_string(),
            num_params: 0,
            registers_count: 1,
            blocks: vec![
                AirBlock {
                    id: crate::bytecode::AirBlockId(0),
                    insts: vec![
                        AirOpcode::CreateObj { dst: AirReg(0) },
                    ],
                    terminator: Some(AirTerminator::Return(AirReg(0))),
                }
            ],
            const_pool: AirConstantPool::default(),
        };

        let res = EscapeAnalysis::run(&air);
        assert_eq!(res.statuses.get(&AirReg(0)), Some(&EscapeStatus::Escaping));
    }

    #[test]
    fn test_escaping_via_call() {
        let mut air = AirFunction {
            name: "test".to_string(),
            num_params: 0,
            registers_count: 2,
            blocks: vec![
                AirBlock {
                    id: crate::bytecode::AirBlockId(0),
                    insts: vec![
                        AirOpcode::CreateObj { dst: AirReg(0) },
                        AirOpcode::LoadInt32 { dst: AirReg(1), value: 0 }, // dummy func
                        AirOpcode::Call { 
                            dst: AirReg(1), 
                            func: AirReg(1), 
                            this: AirReg(0), // escapes as 'this'
                            arg_start: AirReg(0), 
                            num_args: 0, 
                            ic_slot: 0 
                        },
                    ],
                    terminator: Some(AirTerminator::Return(AirReg(1))),
                }
            ],
            const_pool: AirConstantPool::default(),
        };

        let res = EscapeAnalysis::run(&air);
        assert_eq!(res.statuses.get(&AirReg(0)), Some(&EscapeStatus::Escaping));
    }
}
