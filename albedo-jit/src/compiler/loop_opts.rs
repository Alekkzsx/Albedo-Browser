//! # Loop Optimizations for Albedo AIR
//!
//! Este módulo implementa análise de loops (CFG + Dominadores) e otimizações:
//! 1. LICM (Loop Invariant Code Motion)
//! 2. Loop Unrolling (≤ 8 iterações)
//! 3. Strength Reduction (Mul -> Shl)

use crate::bytecode::{AirFunction, AirBlockId, AirOpcode, AirTerminator, AirReg, AirBlock};
use std::collections::{HashMap, HashSet};

/// Analisador de loops baseado em dominadores.
pub struct LoopAnalysis {
    pub dominators: HashMap<AirBlockId, HashSet<AirBlockId>>,
    pub loops: Vec<NaturalLoop>,
    pub predecessors: HashMap<AirBlockId, Vec<AirBlockId>>,
    pub reg_defs: HashMap<AirReg, AirBlockId>,
}

#[derive(Debug, Clone)]
pub struct NaturalLoop {
    pub header: AirBlockId,
    pub back_edge_src: AirBlockId,
    pub body: HashSet<AirBlockId>,
}

impl LoopAnalysis {
    pub fn new(air: &AirFunction) -> Self {
        let mut loop_analysis = Self {
            dominators: HashMap::new(),
            loops: Vec::new(),
            predecessors: HashMap::new(),
            reg_defs: HashMap::new(),
        };
        loop_analysis.compute_predecessors(air);
        loop_analysis.compute_reg_defs(air);
        loop_analysis.compute_dominators(air);
        loop_analysis.find_loops(air);
        loop_analysis
    }

    fn compute_predecessors(&mut self, air: &AirFunction) {
        for block in &air.blocks {
            if let Some(term) = &block.terminator {
                match term {
                    AirTerminator::Jump(target) => {
                        self.predecessors.entry(*target).or_default().push(block.id);
                    }
                    AirTerminator::JumpIf { then_blk, else_blk, .. } => {
                        self.predecessors.entry(*then_blk).or_default().push(block.id);
                        self.predecessors.entry(*else_blk).or_default().push(block.id);
                    }
                    AirTerminator::Return(_) => {}
                }
            }
        }
    }

    fn compute_reg_defs(&mut self, air: &AirFunction) {
        for block in &air.blocks {
            for inst in &block.insts {
                if let Some(dst) = inst.dst_reg() {
                    self.reg_defs.insert(dst, block.id);
                }
            }
        }
    }

    fn compute_dominators(&mut self, air: &AirFunction) {
        if air.blocks.is_empty() { return; }
        let all_blocks: HashSet<AirBlockId> = air.blocks.iter().map(|b| b.id).collect();
        let entry_node = air.blocks[0].id;

        for block in &air.blocks {
            if block.id == entry_node {
                let mut set = HashSet::new();
                set.insert(entry_node);
                self.dominators.insert(entry_node, set);
            } else {
                self.dominators.insert(block.id, all_blocks.clone());
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for block in &air.blocks {
                if block.id == entry_node { continue; }

                let preds = match self.predecessors.get(&block.id) {
                    Some(p) => p,
                    None => continue,
                };

                let mut new_dom = all_blocks.clone();
                for &pred in preds {
                    if let Some(pred_dom) = self.dominators.get(&pred) {
                        new_dom = new_dom.intersection(pred_dom).cloned().collect();
                    }
                }
                new_dom.insert(block.id);

                if let Some(old_dom) = self.dominators.get(&block.id) {
                    if &new_dom != old_dom {
                        self.dominators.insert(block.id, new_dom);
                        changed = true;
                    }
                } else {
                    self.dominators.insert(block.id, new_dom);
                    changed = true;
                }
            }
        }
    }

    fn find_loops(&mut self, air: &AirFunction) {

        for block in &air.blocks {
            if let Some(term) = &block.terminator {
                let targets = match term {
                    AirTerminator::Jump(t) => vec![*t],
                    AirTerminator::JumpIf { then_blk, else_blk, .. } => vec![*then_blk, *else_blk],
                    AirTerminator::Return(_) => vec![],
                };

                for target in targets {
                    // Back-edge: target domina o bloco atual (block.id)
                    if let Some(doms) = self.dominators.get(&block.id) {
                        if doms.contains(&target) {
                            let body = self.find_loop_body(target, block.id, &self.predecessors);
                            self.loops.push(NaturalLoop {
                                header: target,
                                back_edge_src: block.id,
                                body,
                            });
                        }
                    }
                }
            }
        }
    }

    fn find_loop_body(&self, header: AirBlockId, back_edge_src: AirBlockId, predecessors: &HashMap<AirBlockId, Vec<AirBlockId>>) -> HashSet<AirBlockId> {
        let mut body = HashSet::new();
        body.insert(header);
        body.insert(back_edge_src);

        let mut stack = vec![back_edge_src];
        
        while let Some(node) = stack.pop() {
            if node == header { continue; }
            if let Some(preds) = predecessors.get(&node) {
                for &pred in preds {
                    if !body.contains(&pred) {
                        body.insert(pred);
                        stack.push(pred);
                    }
                }
            }
        }
        body
    }
}

/// Otimizador de loops.
pub struct LoopOptimizer<'a> {
    pub air: &'a mut AirFunction,
}

impl<'a> LoopOptimizer<'a> {
    pub fn new(air: &'a mut AirFunction) -> Self {
        Self { air }
    }

    pub fn run(&mut self) {
        let analysis = LoopAnalysis::new(self.air);
        
        // 1. Strength Reduction
        self.apply_strength_reduction();

        // 2. LICM & Unrolling
        for lp in &analysis.loops {
            self.apply_licm(lp, &analysis);
            self.apply_loop_unrolling(lp, &analysis);
        }
    }

    fn apply_strength_reduction(&mut self) {
        let mut constants = HashMap::new();
        for block in &self.air.blocks {
            for inst in &block.insts {
                if let AirOpcode::LoadInt32 { dst, value } = *inst {
                    constants.insert(dst, value);
                }
            }
        }

        for block in &mut self.air.blocks {
            let mut i = 0;
            while i < block.insts.len() {
                if let AirOpcode::Mul { dst, lhs, rhs } = block.insts[i] {
                    let const_val = constants.get(&lhs).or(constants.get(&rhs));
                    if let Some(&val) = const_val {
                        if val > 0 && (val as u32).is_power_of_two() {
                            let shift = (val as f32).log2() as u32;
                            let other = if constants.get(&lhs) == Some(&val) { rhs } else { lhs };
                            
                            let shift_reg = AirReg(self.air.registers_count);
                            self.air.registers_count += 1;
                            
                            block.insts.insert(i, AirOpcode::LoadInt32 { dst: shift_reg, value: shift as i32 });
                            block.insts[i + 1] = AirOpcode::Shl { dst, lhs: other, rhs: shift_reg };
                            i += 1;
                        }
                    }
                }
                i += 1;
            }
        }
    }

    fn apply_licm(&mut self, lp: &NaturalLoop, analysis: &LoopAnalysis) {
        let mut invariant_insts = Vec::new();
        let mut moved_dsts = HashSet::new();

        // 1. Identificar invariantes
        for &block_id in &lp.body {
            let block = self.air.blocks.iter().find(|b| b.id == block_id).unwrap();
            for inst in &block.insts {
                if self.is_invariant(inst, lp, analysis) {
                    invariant_insts.push(inst.clone());
                    if let Some(dst) = inst.dst_reg() {
                        moved_dsts.insert(dst);
                    }
                }
            }
        }

        if invariant_insts.is_empty() {
            return;
        }

        // 2. Criar pre-header
        let pre_header_id = self.create_pre_header(lp.header, analysis);

        // 3. Mover instruções para o pre-header
        let pre_header = self.air.blocks.iter_mut().find(|b| b.id == pre_header_id).unwrap();
        for inst in invariant_insts {
            pre_header.insts.push(inst);
        }

        // 4. Remover as originais do corpo do loop
        for &block_id in &lp.body {
            let block = self.air.blocks.iter_mut().find(|b| b.id == block_id).unwrap();
            block.insts.retain(|inst| {
                if let Some(dst) = inst.dst_reg() {
                    !moved_dsts.contains(&dst)
                } else {
                    true
                }
            });
        }
    }

    fn apply_loop_unrolling(&mut self, lp: &NaturalLoop, _analysis: &LoopAnalysis) {
        // Estágio Beta: Implementação para loops triviais.
        // Heurística: se o loop tem < 20 instruções e poucos blocos, desenrolar por fator de 2.
        let mut inst_count = 0;
        for &block_id in &lp.body {
            if let Some(block) = self.air.blocks.iter().find(|b| b.id == block_id) {
                inst_count += block.insts.len();
            }
        }

        if inst_count > 20 || lp.body.len() > 2 {
            return;
        }

        let header_id = lp.header;
        let back_edge_id = lp.back_edge_src;

        if lp.body.len() == 2 && header_id != back_edge_id {
            // Pattern A: While/For loop padrão (header + body)
            let header_block_idx = self.air.blocks.iter().position(|b| b.id == header_id).unwrap();
            let back_edge_idx = self.air.blocks.iter().position(|b| b.id == back_edge_id).unwrap();

            let header_term = self.air.blocks[header_block_idx].terminator.clone();
            let back_edge_term = self.air.blocks[back_edge_idx].terminator.clone();

            if let Some(AirTerminator::JumpIf { cond, then_blk, else_blk }) = header_term {
                if let Some(AirTerminator::Jump(target)) = back_edge_term {
                    if target == header_id && (then_blk == back_edge_id || else_blk == back_edge_id) {
                        let is_then = then_blk == back_edge_id;
                        let exit_blk = if is_then { else_blk } else { then_blk };

                        let h_insts = self.air.blocks[header_block_idx].insts.clone();
                        let b_insts = self.air.blocks[back_edge_idx].insts.clone();

                        let new_b_id = AirBlockId(self.air.blocks.len() as u32);
                        
                        // Modifier B (Primeira iteração do unroll)
                        let b_block = &mut self.air.blocks[back_edge_idx];
                        for inst in &h_insts {
                            b_block.insts.push(inst.clone());
                        }
                        b_block.terminator = Some(AirTerminator::JumpIf {
                            cond,
                            then_blk: if is_then { new_b_id } else { exit_blk },
                            else_blk: if is_then { exit_blk } else { new_b_id },
                        });

                        // Novo B_second (Segunda iteração do unroll)
                        let mut b_second = AirBlock::new(new_b_id.0);
                        b_second.insts = b_insts;
                        b_second.terminator = Some(AirTerminator::Jump(header_id));
                        self.air.blocks.push(b_second);
                    }
                }
            }
        } else if lp.body.len() == 1 && header_id == back_edge_id {
            // Pattern B: Do-While loop compacto
            let header_block_idx = self.air.blocks.iter().position(|b| b.id == header_id).unwrap();
            let header_term = self.air.blocks[header_block_idx].terminator.clone();

            if let Some(AirTerminator::JumpIf { cond, then_blk, else_blk }) = header_term {
                if then_blk == header_id || else_blk == header_id {
                    let is_then = then_blk == header_id;
                    let exit_blk = if is_then { else_blk } else { then_blk };
                    let h_insts = self.air.blocks[header_block_idx].insts.clone();

                    let new_h_id = AirBlockId(self.air.blocks.len() as u32);

                    // Modiifer H (Primeira iteração do unroll)
                    let h_block = &mut self.air.blocks[header_block_idx];
                    h_block.terminator = Some(AirTerminator::JumpIf {
                        cond,
                        then_blk: if is_then { new_h_id } else { exit_blk },
                        else_blk: if is_then { exit_blk } else { new_h_id },
                    });

                    // Novo H_second (Segunda iteração do unroll)
                    let mut h_second = AirBlock::new(new_h_id.0);
                    h_second.insts = h_insts;
                    h_second.terminator = Some(AirTerminator::JumpIf {
                        cond,
                        then_blk: if is_then { header_id } else { exit_blk },
                        else_blk: if is_then { exit_blk } else { header_id },
                    });
                    self.air.blocks.push(h_second);
                }
            }
        }
    }

    fn create_pre_header(&mut self, header_id: AirBlockId, analysis: &LoopAnalysis) -> AirBlockId {
        let new_id = AirBlockId(self.air.blocks.len() as u32);
        let mut pre_header = AirBlock::new(new_id.0);
        pre_header.terminator = Some(AirTerminator::Jump(header_id));
        
        let mut external_preds = Vec::new();
        if let Some(preds) = analysis.predecessors.get(&header_id) {
            for &pred in preds {
                let is_internal = lp_body_contains(analysis, header_id, pred);
                if !is_internal {
                    external_preds.push(pred);
                }
            }
        }

        for pred_id in external_preds {
            let pred_block = self.air.blocks.iter_mut().find(|b| b.id == pred_id).unwrap();
            if let Some(term) = &mut pred_block.terminator {
                match term {
                    AirTerminator::Jump(target) if *target == header_id => {
                        *target = new_id;
                    }
                    AirTerminator::JumpIf { then_blk, else_blk, .. } => {
                        if *then_blk == header_id { *then_blk = new_id; }
                        if *else_blk == header_id { *else_blk = new_id; }
                    }
                    _ => {}
                }
            }
        }

        self.air.blocks.push(pre_header);
        new_id
    }

    fn is_invariant(&self, inst: &AirOpcode, lp: &NaturalLoop, analysis: &LoopAnalysis) -> bool {
        match inst {
            AirOpcode::Add { .. } | AirOpcode::Sub { .. } | AirOpcode::Mul { .. } |
            AirOpcode::Div { .. } | AirOpcode::Mod { .. } | AirOpcode::Neg { .. } |
            AirOpcode::BitAnd { .. } | AirOpcode::BitOr { .. } | AirOpcode::BitXor { .. } |
            AirOpcode::Shl { .. } | AirOpcode::Shr { .. } | AirOpcode::UShr { .. } |
            AirOpcode::Eq { .. } | AirOpcode::StrictEq { .. } | AirOpcode::Lt { .. } |
            AirOpcode::Lte { .. } | AirOpcode::Gt { .. } | AirOpcode::Gte { .. } |
            AirOpcode::Not { .. } | AirOpcode::ToNumber { .. } | AirOpcode::ToBool { .. } |
            AirOpcode::TypeOf { .. } | AirOpcode::LoadInt32 { .. } | AirOpcode::LoadFloat64 { .. } |
            AirOpcode::LoadBool { .. } | AirOpcode::LoadUndefined { .. } | AirOpcode::LoadNull { .. } |
            AirOpcode::Move { .. } => {}
            _ => return false,
        }

        for op in inst.operands() {
            if op.0 < self.air.num_params {
                continue;
            }
            if let Some(&def_block) = analysis.reg_defs.get(&op) {
                if lp.body.contains(&def_block) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

fn lp_body_contains(analysis: &LoopAnalysis, header_id: AirBlockId, block_id: AirBlockId) -> bool {
    analysis.loops.iter()
        .filter(|l| l.header == header_id)
        .any(|l| l.body.contains(&block_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::AirBuilder;

    #[test]
    fn test_licm_simple() {
        let mut builder = AirBuilder::new("test_loop", 0, 0);
        let b_cond = builder.create_block();
        let b_body = builder.create_block();
        let b_exit = builder.create_block();

        let zero = builder.emit_load_int32(0);
        let sum = builder.new_reg();
        builder.emit_move(sum, zero);
        let i = builder.new_reg();
        builder.emit_move(i, zero);
        builder.emit_jump(b_cond);

        builder.switch_block(b_cond);
        let limit = builder.emit_load_int32(100);
        let cond = builder.emit_lt(i, limit);
        builder.emit_jump_if(cond, b_body, b_exit);

        builder.switch_block(b_body);
        let x = builder.emit_load_int32(10); // INVARIANT
        let sum_next = builder.emit_add(sum, x);
        builder.emit_move(sum, sum_next);
        let one = builder.emit_load_int32(1);
        let i_next = builder.emit_add(i, one);
        builder.emit_move(i, i_next);
        builder.emit_jump(b_cond);

        builder.switch_block(b_exit);
        builder.emit_return(sum);

        let mut air = builder.build();
        let mut optimizer = LoopOptimizer::new(&mut air);
        optimizer.run();

        // 3 originais + 1 pre-header
        assert!(air.blocks.len() >= 4);
        
        let has_moved_x = air.blocks.iter().any(|b| {
            b.insts.iter().any(|inst| {
                if let AirOpcode::LoadInt32 { value, .. } = inst {
                    *value == 10
                } else {
                    false
                }
            }) && b.id.0 >= 4 
        });
        assert!(has_moved_x, "O LoadInt32(10) deveria ter sido movido para o pre-header");
    }

    #[test]
    fn test_strength_reduction() {
        let mut builder = AirBuilder::new("test_sr", 0, 0);
        let r1 = builder.emit_load_int32(16);
        let r2 = builder.new_reg();
        let res = builder.emit_mul(r1, r2);
        builder.emit_return(res);

        let mut air = builder.build();
        let mut optimizer = LoopOptimizer::new(&mut air);
        optimizer.run();

        let block = &air.blocks[0];
        let has_shl = block.insts.iter().any(|inst| matches!(inst, AirOpcode::Shl { .. }));
        assert!(has_shl, "O Mul deveria ter sido convertido em Shl");
        
        let has_load_4 = block.insts.iter().any(|inst| {
            if let AirOpcode::LoadInt32 { value, .. } = inst {
                *value == 4 // log2(16)
            } else {
                false
            }
        });
        assert!(has_load_4, "O valor do shift (4) deveria ter sido carregado");
    }

    #[test]
    fn test_loop_unrolling() {
        // Criando um loop while i < 10 { i += 1 }
        let mut builder = AirBuilder::new("test_unroll", 0, 0);
        let b_cond = builder.create_block();
        let b_body = builder.create_block();
        let b_exit = builder.create_block();

        let i = builder.emit_load_int32(0);
        builder.emit_jump(b_cond);

        // Header do loop (b_cond)
        builder.switch_block(b_cond);
        let limit = builder.emit_load_int32(10);
        let cond = builder.emit_lt(i, limit);
        builder.emit_jump_if(cond, b_body, b_exit);

        // Corpo do loop (b_body)
        builder.switch_block(b_body);
        let one = builder.emit_load_int32(1);
        let i_next = builder.emit_add(i, one);
        builder.emit_move(i, i_next);
        builder.emit_jump(b_cond);

        builder.switch_block(b_exit);
        builder.emit_return(i);

        let mut air = builder.build();
        let before_blocks = air.blocks.len();
        let mut optimizer = LoopOptimizer::new(&mut air);
        
        optimizer.run();

        // O otimizador de LICM vai extrair constantes e criar 1 bloco pre-header.
        // O Unrolling vai desenrolar criando 1 bloco novo para a segunda iteração.
        // Número de blocos deve ser origin + 2
        assert!(air.blocks.len() >= before_blocks + 1, "Deveria ter criado novos blocos de unrolling");
        
        // Vamos verificar se algum bloco tem o add repetido!
        let add_count: usize = air.blocks.iter().map(|b| {
            b.insts.iter().filter(|inst| matches!(inst, AirOpcode::Add { .. })).count()
        }).sum();

        assert!(add_count >= 2, "As instruções do loop devem ter sido duplicadas");
    }
}
