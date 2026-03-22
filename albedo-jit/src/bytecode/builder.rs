//! API de construção do Albedo IR.
//! Permite construir blocos e inserir instruções de forma fluente e segura.
//! O QuickJS Decoder ou o Profiler utilizarão esse Builder.

use super::opcodes::*;
use crate::runtime::object_model;
use crate::runtime::type_feedback::{IcKind, TypeFeedbackRegistry};

pub struct AirBuilder {
    name: String,
    num_params: u32,
    num_locals: u32,
    next_reg: u32,
    pub blocks: Vec<AirBlock>,
    const_pool: AirConstantPool,
    pub current_block: u32,
}

impl AirBuilder {
    /// Inicia uma nova função AIR com blocos base vazios
    pub fn new(name: impl Into<String>, num_params: u32, num_locals: u32) -> Self {
        let mut blocks = Vec::new();
        // Cria obrigatoriamente o bloco de entrada (Entry Block: b0)
        blocks.push(AirBlock::new(0));

        Self {
            name: name.into(),
            num_params,
            num_locals,
            next_reg: num_params + num_locals, // params [0..num_params], locals [num_params..num_params+num_locals]
            blocks,
            const_pool: AirConstantPool::default(),
            current_block: 0,
        }
    }

    /// Retorna o registrador associado a um parâmetro de entrada
    pub fn param(&self, index: u32) -> AirReg {
        assert!(index < self.num_params, "Índice de parâmetro inválido");
        AirReg(index)
    }

    /// Retorna o registrador associado a uma variável local mutável
    pub fn local(&self, index: u32) -> AirReg {
        assert!(index < self.num_locals, "Índice de local inválido");
        AirReg(self.num_params + index)
    }

    /// Aloca um novo registrador sequencial local
    pub fn new_reg(&mut self) -> AirReg {
        let r = AirReg(self.next_reg);
        self.next_reg += 1;
        r
    }

    /// Cria um novo bloco em branco (sem setar o current) e retorna o seu ID
    pub fn create_block(&mut self) -> AirBlockId {
        let id = self.blocks.len() as u32;
        self.blocks.push(AirBlock::new(id));
        AirBlockId(id)
    }

    /// Define o bloco atual no builder para inserção de instruções
    pub fn switch_block(&mut self, id: AirBlockId) {
        assert!((id.0 as usize) < self.blocks.len(), "Bloco não existe");
        self.current_block = id.0;
    }

    fn push_op(&mut self, op: AirOpcode) {
        let blk = &mut self.blocks[self.current_block as usize];
        assert!(
            blk.terminator.is_none(),
            "Não se pode inserir em bloco já terminado"
        );
        blk.insts.push(op);
    }

    // ---------------------------------------------------------
    // Emissão de Instruções -> Helpers de Construção
    // ---------------------------------------------------------

    pub fn emit_load_int32(&mut self, value: i32) -> AirReg {
        let dst = self.new_reg();
        self.push_op(AirOpcode::LoadInt32 { dst, value });
        dst
    }

    pub fn emit_load_float64(&mut self, value: f64) -> AirReg {
        let dst = self.new_reg();
        self.push_op(AirOpcode::LoadFloat64 { dst, value });
        dst
    }

    pub fn emit_load_bool(&mut self, value: bool) -> AirReg {
        let dst = self.new_reg();
        self.push_op(AirOpcode::LoadBool { dst, value });
        dst
    }

    pub fn emit_load_string(&mut self, s: String) -> AirReg {
        let _ = self.const_pool.add_string(s.clone());
        let str_id = object_model::intern_string(s);
        let dst = self.new_reg();
        self.push_op(AirOpcode::LoadString { dst, str_id });
        dst
    }

    pub fn emit_load_undefined(&mut self) -> AirReg {
        let dst = self.new_reg();
        self.push_op(AirOpcode::LoadUndefined { dst });
        dst
    }

    pub fn emit_load_null(&mut self) -> AirReg {
        let dst = self.new_reg();
        self.push_op(AirOpcode::LoadNull { dst });
        dst
    }

    pub fn emit_add(&mut self, lhs: AirReg, rhs: AirReg) -> AirReg {
        let dst = self.new_reg();
        let ic_slot = TypeFeedbackRegistry::alloc_slot(IcKind::Add);
        self.push_op(AirOpcode::Add {
            dst,
            lhs,
            rhs,
            ic_slot,
        });
        dst
    }

    pub fn emit_sub(&mut self, lhs: AirReg, rhs: AirReg) -> AirReg {
        let dst = self.new_reg();
        self.push_op(AirOpcode::Sub { dst, lhs, rhs });
        dst
    }

    pub fn emit_mul(&mut self, lhs: AirReg, rhs: AirReg) -> AirReg {
        let dst = self.new_reg();
        self.push_op(AirOpcode::Mul { dst, lhs, rhs });
        dst
    }

    pub fn emit_eq(&mut self, lhs: AirReg, rhs: AirReg) -> AirReg {
        let dst = self.new_reg();
        self.push_op(AirOpcode::Eq { dst, lhs, rhs });
        dst
    }

    pub fn emit_strict_eq(&mut self, lhs: AirReg, rhs: AirReg) -> AirReg {
        let dst = self.new_reg();
        self.push_op(AirOpcode::StrictEq { dst, lhs, rhs });
        dst
    }

    pub fn emit_lt(&mut self, lhs: AirReg, rhs: AirReg) -> AirReg {
        let dst = self.new_reg();
        self.push_op(AirOpcode::Lt { dst, lhs, rhs });
        dst
    }

    pub fn emit_move(&mut self, dst: AirReg, src: AirReg) {
        self.push_op(AirOpcode::Move { dst, src });
    }

    pub fn emit_get_prop(&mut self, obj: AirReg, prop: AirReg) -> AirReg {
        let dst = self.new_reg();
        let ic_slot = TypeFeedbackRegistry::alloc_slot(IcKind::GetProp);
        self.push_op(AirOpcode::GetProp {
            dst,
            obj,
            prop,
            ic_slot,
        });
        dst
    }

    pub fn emit_call(&mut self, func: AirReg, arg_start: AirReg, num_args: u32) -> AirReg {
        let dst = self.new_reg();
        let ic_slot = TypeFeedbackRegistry::alloc_slot(IcKind::Call);
        self.push_op(AirOpcode::Call {
            dst,
            func,
            arg_start,
            num_args,
            ic_slot,
        });
        dst
    }

    // ---------------------------------------------------------
    // Emissão de Terminadores de Bloco (Control Flow)
    // ---------------------------------------------------------

    pub fn emit_return(&mut self, value: AirReg) {
        let blk = &mut self.blocks[self.current_block as usize];
        assert!(blk.terminator.is_none(), "Bloco já possui terminador");
        blk.terminator = Some(AirTerminator::Return(value));
    }

    pub fn emit_jump(&mut self, target: AirBlockId) {
        let blk = &mut self.blocks[self.current_block as usize];
        assert!(blk.terminator.is_none());
        blk.terminator = Some(AirTerminator::Jump(target));
    }

    pub fn emit_jump_if(&mut self, cond: AirReg, then_blk: AirBlockId, else_blk: AirBlockId) {
        let blk = &mut self.blocks[self.current_block as usize];
        assert!(blk.terminator.is_none());
        blk.terminator = Some(AirTerminator::JumpIf {
            cond,
            then_blk,
            else_blk,
        });
    }

    // ---------------------------------------------------------
    // Finalização
    // ---------------------------------------------------------

    /// Encerra a construção e retorna o AirFunction.
    pub fn build(self) -> AirFunction {
        let func = AirFunction {
            name: self.name,
            num_params: self.num_params,
            registers_count: self.next_reg,
            blocks: self.blocks,
            const_pool: self.const_pool,
        };
        assert!(
            func.is_valid(),
            "Função AIR compilada não é válida (falta terminador?)"
        );
        func
    }
}
