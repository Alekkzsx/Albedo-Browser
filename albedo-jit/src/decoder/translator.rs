//! O coração do Decoder.
//! Converte a pilha dinâmica do QuickJS na representação fixa Register-based
//! (SSA-like) do AIR.
//!
//! Exemplo:
//! QJS:
//!   PushI32(10)
//!   PushI32(20)
//!   Add
//!
//! AIR:
//!   v0 = LoadInt32(10)
//!   v1 = LoadInt32(20)
//!   v2 = Add(v0, v1)
//!
//! A Pilha Virtual neste módulo cuida do mapeamento em tempo-de-compilação.

use std::collections::HashMap;

use crate::bytecode::{AirBuilder, AirFunction, AirReg};
use crate::decoder::qjs_opcodes::{QjsBytecodeFunction, QjsOpcode};
use crate::decoder::source_map::AirSourceMap;

pub struct StackToRegisterTranslator {
    builder: AirBuilder,
    source_map: AirSourceMap,
    const_pool_strings: Vec<String>,

    /// A Pilha Virtual.
    /// Simula a pilha de valores de execução do QuickJS com os "Registradores" do AIR.
    /// Ex: Se a pilha tem `[AirReg(2), AirReg(3)]`, e fazemos Add, nós popamos os dois.
    vstack: Vec<AirReg>,

    /// Mapa de offsets do QuickJS (i32) para AIR Blocks (para Gotos)
    jump_targets: HashMap<i32, crate::bytecode::AirBlockId>,
}

impl StackToRegisterTranslator {
    pub fn new(func: &QjsBytecodeFunction) -> Self {
        // Pré-analisa Jumps e aloca os blocos AIR para cada target de salto
        let mut jump_targets = HashMap::new();
        let mut builder = AirBuilder::new(&func.name, func.num_args, func.num_locals);

        let mut offset = 0i32;
        for op in &func.opcodes {
            match op {
                QjsOpcode::Goto(target_offset)
                | QjsOpcode::IfTrue(target_offset)
                | QjsOpcode::IfFalse(target_offset) => {
                    let absolute_target = offset + *target_offset; // branch target
                    if !jump_targets.contains_key(&absolute_target) {
                        jump_targets.insert(absolute_target, builder.create_block());
                    }

                    let fallthrough_target = offset + 1; // branch sequencial
                    if !jump_targets.contains_key(&fallthrough_target) {
                        jump_targets.insert(fallthrough_target, builder.create_block());
                    }
                }
                _ => {}
            }
            offset += 1;
        }

        Self {
            builder,
            source_map: AirSourceMap::new(),
            const_pool_strings: func.constant_pool_strings.clone(),
            vstack: Vec::new(),
            jump_targets,
        }
    }

    /// O loop principal da Tradução
    pub fn translate(mut self, func: QjsBytecodeFunction) -> (AirFunction, AirSourceMap) {
        // Para simplificar a fase atual, adicionaremos as strings lazy-load.

        for (i, op) in func.opcodes.iter().enumerate() {
            let offset = i as i32;

            println!(
                "OFFSET {}: {:?} (Current Blk: {})",
                offset, op, self.builder.current_block
            );

            // Se a instrução for inicializada num offset mapeado como Jump Target
            if let Some(block_id) = self.jump_targets.get(&offset) {
                let curr_idx = self.builder.current_block as usize;

                // Se o bloco anterior não terminou, ele tem um fallthrough implícito pra este novo bloco.
                if curr_idx != block_id.0 as usize
                    && self.builder.blocks[curr_idx].terminator.is_none()
                {
                    self.builder.emit_jump(*block_id);
                }

                self.builder.switch_block(*block_id);
                self.source_map.map_block(block_id.0, offset as usize);
            }

            self.translate_instruction(op, offset as usize);
        }

        // Em um Fallback sem Return final no QuickJS, emitimos um ReturnUndefined
        let last_blk = &self.builder.build(); // O builder clona ou resgata as refs, mas aqui fazemos build
        let func_res = last_blk.clone();
        (func_res, self.source_map)
    }

    fn translate_instruction(&mut self, op: &QjsOpcode, qjs_offset: usize) {
        match op {
            QjsOpcode::PushI32(val) => {
                let dst = self.builder.emit_load_int32(*val);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::PushFloat64(val) => {
                let dst = self.builder.emit_load_float64(*val);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::PushBool(val) => {
                let dst = self.builder.emit_load_bool(*val);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::PushUndefined => {
                let dst = self.builder.emit_load_undefined();
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::PushNull => {
                let dst = self.builder.emit_load_null();
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::PushString(str_idx) => {
                let s = self
                    .const_pool_strings
                    .get(*str_idx as usize)
                    .cloned()
                    .unwrap_or_else(|| "".to_string());
                let dst = self.builder.emit_load_string(s);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }

            // --- Variáveis Locais ---
            QjsOpcode::PutLoc(loc_idx) => {
                let src_val = self.vstack.pop().expect("Stack underflow em PutLoc");
                let dst_loc = self.builder.local(*loc_idx);
                self.builder.emit_move(dst_loc, src_val);
            }
            QjsOpcode::GetLoc(loc_idx) => {
                let local_reg = self.builder.local(*loc_idx);
                let dst = self.builder.new_reg();
                self.builder.emit_move(dst, local_reg);
                self.vstack.push(dst);
            }

            // --- Argumentos ---
            QjsOpcode::GetArg(arg_idx) => {
                let param_reg = self.builder.param(*arg_idx);
                self.vstack.push(param_reg);
            }

            // --- Aritmética Dinâmica ---
            QjsOpcode::Add => {
                let rhs = self.vstack.pop().expect("Underflow Add rhs");
                let lhs = self.vstack.pop().expect("Underflow Add lhs");
                let dst = self.builder.emit_add(lhs, rhs);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::Sub => {
                let rhs = self.vstack.pop().expect("Underflow Sub");
                let lhs = self.vstack.pop().expect("Underflow Sub");
                let dst = self.builder.emit_sub(lhs, rhs);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::Mul => {
                let rhs = self.vstack.pop().expect("Underflow Mul");
                let lhs = self.vstack.pop().expect("Underflow Mul");
                let dst = self.builder.emit_mul(lhs, rhs);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::Eq => {
                let rhs = self.vstack.pop().unwrap();
                let lhs = self.vstack.pop().unwrap();
                let dst = self.builder.emit_eq(lhs, rhs);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::StrictEq => {
                let rhs = self.vstack.pop().unwrap();
                let lhs = self.vstack.pop().unwrap();
                let dst = self.builder.emit_strict_eq(lhs, rhs);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::Lt => {
                let rhs = self.vstack.pop().unwrap();
                let lhs = self.vstack.pop().unwrap();
                let dst = self.builder.emit_lt(lhs, rhs);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::GetField(str_idx) => {
                let obj = self.vstack.pop().expect("Underflow GetField obj");
                let s = self
                    .const_pool_strings
                    .get(*str_idx as usize)
                    .cloned()
                    .unwrap_or_else(|| "".to_string());
                let prop = self.builder.emit_load_string(s);
                let dst = self.builder.emit_get_prop(obj, prop);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }
            QjsOpcode::Call(num_args) => {
                let mut args = Vec::with_capacity(*num_args as usize);
                for _ in 0..*num_args {
                    args.push(self.vstack.pop().expect("Underflow Call arg"));
                }
                args.reverse();
                let func_reg = self.vstack.pop().expect("Underflow Call func");

                let arg_start = if args.is_empty() {
                    self.builder.emit_load_undefined()
                } else {
                    let mut arg_regs = Vec::with_capacity(args.len());
                    for arg in args {
                        let dst = self.builder.new_reg();
                        self.builder.emit_move(dst, arg);
                        arg_regs.push(dst);
                    }
                    arg_regs[0]
                };

                let dst = self.builder.emit_call(func_reg, arg_start, *num_args);
                self.vstack.push(dst);
                self.source_map.map_reg(dst, qjs_offset);
            }

            // --- Control Flow ---
            QjsOpcode::Return => {
                let ret_val = self
                    .vstack
                    .pop()
                    .unwrap_or_else(|| self.builder.emit_load_undefined());
                self.builder.emit_return(ret_val);
            }

            QjsOpcode::Goto(offset_jump) => {
                let target_offset = qjs_offset as i32 + *offset_jump;
                let target_block = self.jump_targets.get(&target_offset).unwrap();
                self.builder.emit_jump(*target_block);
            }

            QjsOpcode::IfFalse(offset_jump) => {
                let cond = self.vstack.pop().unwrap(); // Booleano
                let target_offset = qjs_offset as i32 + *offset_jump;
                let else_block = *self.jump_targets.get(&target_offset).unwrap();

                // Em IfFalse, a condição verdadeira (Then) é o Fallthrough (offset + 1).
                let fallthrough_offset = qjs_offset as i32 + 1;

                // Pega o bloco do fallthrough alvo que foi garantido na pré-passagem
                let then_block = *self
                    .jump_targets
                    .get(&fallthrough_offset)
                    .unwrap_or_else(|| {
                        panic!("Missing then_block for offset {}", fallthrough_offset)
                    });

                self.builder.emit_jump_if(cond, then_block, else_block);
            }

            // Ignorando os opcodes não completamente portados por simplificação da Fase
            _ => {
                println!("[Decoder] Ignorando Opcode {:?}", op);
            }
        }
    }
}
