//! Source Map do Decoder JIT para Debugging
//!
//! Associa cada instrução AIR (registrador de destino e tipo) convertida
//! pelo Translator ao offset original do bytecode do QuickJS que o originou.

use std::collections::HashMap;

use crate::bytecode::AirReg;

#[derive(Debug, Clone, PartialEq)]
pub struct AirSourceMap {
    /// Mapeia o Registrador AIR de destino para o offset (índice da instrução) no QuickJS
    pub reg_to_qjs_offset: HashMap<u32, usize>,

    /// Mapeia o ID do Bloco AIR para o offset de entrada correspondente no QuickJS
    pub block_to_qjs_offset: HashMap<u32, usize>,
}

impl AirSourceMap {
    pub fn new() -> Self {
        Self {
            reg_to_qjs_offset: HashMap::new(),
            block_to_qjs_offset: HashMap::new(),
        }
    }

    pub fn map_reg(&mut self, reg: AirReg, qjs_offset: usize) {
        self.reg_to_qjs_offset.insert(reg.0, qjs_offset);
    }

    pub fn map_block(&mut self, block_id: u32, qjs_offset: usize) {
        self.block_to_qjs_offset.insert(block_id, qjs_offset);
    }

    pub fn get_qjs_offset_for_reg(&self, reg: AirReg) -> Option<usize> {
        self.reg_to_qjs_offset.get(&reg.0).copied()
    }
}
