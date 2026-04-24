//! # Loop Optimizations for Albedo AIR
//!
//! Este módulo implementa análise de loops (CFG + Dominadores) e otimizações:
//! 1. LICM (Loop Invariant Code Motion)
//! 2. Loop Unrolling (≤ 8 iterações)
//! 3. Strength Reduction (Mul -> Shl)

use crate::bytecode::{AirBlock, AirBlockId, AirFunction, AirOpcode, AirReg, AirTerminator};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct LoopOptConfig {
    pub enable_strength_reduction: bool,
    pub enable_licm: bool,
    pub enable_unroll: bool,
    pub max_unroll_trip: u32,
    pub debug: bool,
}

impl Default for LoopOptConfig {
    fn default() -> Self {
        let debug = std::env::var("ALBEDO_JIT_LOOP_OPTS_DEBUG")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);

        Self {
            enable_strength_reduction: true,
            enable_licm: true,
            enable_unroll: true,
            max_unroll_trip: 8,
            debug,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LoopOptReport {
    pub loops_detected: usize,
    pub licm_moved: usize,
    pub unroll_applied: usize,
    pub strength_reduced: usize,
    pub skipped_reasons: HashMap<String, usize>,
    pub validation_passed: bool,
    pub rolled_back: bool,
}

impl LoopOptReport {
    fn skip(&mut self, reason: &str) {
        *self.skipped_reasons.entry(reason.to_string()).or_insert(0) += 1;
    }
}

/// Analisador de loops baseado em dominadores.
pub struct LoopAnalysis {
    pub dominators: HashMap<AirBlockId, HashSet<AirBlockId>>,
    pub loops: Vec<NaturalLoop>,
    pub predecessors: HashMap<AirBlockId, Vec<AirBlockId>>,
    pub successors: HashMap<AirBlockId, Vec<AirBlockId>>,
    pub reg_defs: HashMap<AirReg, AirBlockId>,
}

#[derive(Debug, Clone)]
pub struct NaturalLoop {
    pub header: AirBlockId,
    pub latch: AirBlockId,
    pub body: HashSet<AirBlockId>,
    pub preheader: Option<AirBlockId>,
    pub exits: HashSet<AirBlockId>,
}

impl LoopAnalysis {
    pub fn new(air: &AirFunction) -> Self {
        let mut loop_analysis = Self {
            dominators: HashMap::new(),
            loops: Vec::new(),
            predecessors: HashMap::new(),
            successors: HashMap::new(),
            reg_defs: HashMap::new(),
        };
        loop_analysis.compute_edges(air);
        loop_analysis.compute_reg_defs(air);
        loop_analysis.compute_dominators(air);
        loop_analysis.find_loops(air);
        loop_analysis
    }

    fn compute_edges(&mut self, air: &AirFunction) {
        for block in &air.blocks {
            let mut succs = Vec::new();
            if let Some(term) = &block.terminator {
                match term {
                    AirTerminator::Jump(target) => {
                        succs.push(*target);
                        self.predecessors.entry(*target).or_default().push(block.id);
                    }
                    AirTerminator::JumpIf {
                        then_blk, else_blk, ..
                    } => {
                        succs.push(*then_blk);
                        succs.push(*else_blk);
                        self.predecessors
                            .entry(*then_blk)
                            .or_default()
                            .push(block.id);
                        self.predecessors
                            .entry(*else_blk)
                            .or_default()
                            .push(block.id);
                    }
                    AirTerminator::Return(_) => {}
                }
            }
            self.successors.insert(block.id, succs);
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
        if air.blocks.is_empty() {
            return;
        }
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
                if block.id == entry_node {
                    continue;
                }

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
        let mut seen = HashSet::new();
        for block in &air.blocks {
            let targets = self.successors.get(&block.id).cloned().unwrap_or_default();
            for target in targets {
                // Back-edge: target domina o bloco atual (block.id)
                if let Some(doms) = self.dominators.get(&block.id) {
                    if doms.contains(&target) && seen.insert((target, block.id)) {
                        let body = self.find_loop_body(target, block.id, &self.predecessors);
                        let exits = self.find_loop_exits(&body);
                        let preheader = self.find_existing_preheader(target, &body);
                        self.loops.push(NaturalLoop {
                            header: target,
                            latch: block.id,
                            body,
                            preheader,
                            exits,
                        });
                    }
                }
            }
        }
        self.loops.sort_by_key(|lp| (lp.header.0, lp.latch.0));
    }

    fn find_loop_body(
        &self,
        header: AirBlockId,
        back_edge_src: AirBlockId,
        predecessors: &HashMap<AirBlockId, Vec<AirBlockId>>,
    ) -> HashSet<AirBlockId> {
        let mut body = HashSet::new();
        body.insert(header);
        body.insert(back_edge_src);

        let mut stack = vec![back_edge_src];

        while let Some(node) = stack.pop() {
            if node == header {
                continue;
            }
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

    fn find_loop_exits(&self, body: &HashSet<AirBlockId>) -> HashSet<AirBlockId> {
        let mut exits = HashSet::new();
        for &block_id in body {
            if let Some(succs) = self.successors.get(&block_id) {
                for &succ in succs {
                    if !body.contains(&succ) {
                        exits.insert(succ);
                    }
                }
            }
        }
        exits
    }

    fn find_existing_preheader(
        &self,
        header: AirBlockId,
        body: &HashSet<AirBlockId>,
    ) -> Option<AirBlockId> {
        let external_preds: Vec<AirBlockId> = self
            .predecessors
            .get(&header)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|pred| !body.contains(pred))
            .collect();
        if external_preds.len() == 1 {
            Some(external_preds[0])
        } else {
            None
        }
    }
}

/// Otimizador de loops.
pub struct LoopOptimizer<'a> {
    pub air: &'a mut AirFunction,
    config: LoopOptConfig,
    created_preheaders: HashMap<AirBlockId, AirBlockId>,
}

impl<'a> LoopOptimizer<'a> {
    pub fn new(air: &'a mut AirFunction) -> Self {
        Self::with_config(air, LoopOptConfig::default())
    }

    pub fn with_config(air: &'a mut AirFunction, config: LoopOptConfig) -> Self {
        Self {
            air,
            config,
            created_preheaders: HashMap::new(),
        }
    }

    pub fn run(&mut self) -> LoopOptReport {
        let backup = self.air.clone();
        let mut report = LoopOptReport::default();
        let analysis = LoopAnalysis::new(self.air);
        report.loops_detected = analysis.loops.len();

        // 1. Strength Reduction
        if self.config.enable_strength_reduction {
            self.apply_strength_reduction(&mut report);
        }

        // 2. LICM & Unrolling
        if self.config.enable_licm {
            for lp in &analysis.loops {
                self.apply_licm(lp, &analysis, &mut report);
            }
        }

        if self.config.enable_unroll {
            let post_licm_analysis = LoopAnalysis::new(self.air);
            for lp in &post_licm_analysis.loops {
                self.apply_loop_unrolling(lp, &post_licm_analysis, &mut report);
            }
        }

        match validate_air_cfg(self.air) {
            Ok(()) => report.validation_passed = true,
            Err(err) => {
                report.validation_passed = false;
                report.rolled_back = true;
                report.skip("validation_failed_rollback");
                *self.air = backup;
                self.debug_log(&format!("[LoopOpts] validation failed: {err}"));
            }
        }

        self.debug_log(&format!("[LoopOpts] report: {:?}", report));
        report
    }

    fn apply_strength_reduction(&mut self, report: &mut LoopOptReport) {
        let constants = self.collect_i32_constants();
        let int32_proven = self.compute_int32_proven_regs(&constants);

        for block in &mut self.air.blocks {
            let mut shift_regs: HashMap<i32, AirReg> = HashMap::new();
            for inst in &block.insts {
                if let AirOpcode::LoadInt32 { dst, value } = *inst {
                    shift_regs.insert(value, dst);
                }
            }

            let mut i = 0;
            while i < block.insts.len() {
                let (dst, lhs, rhs) = match block.insts[i] {
                    AirOpcode::Mul { dst, lhs, rhs } => (dst, lhs, rhs),
                    _ => {
                        i += 1;
                        continue;
                    }
                };

                let (other, shift) = match extract_pow2_mul(lhs, rhs, &constants) {
                    Some(v) => v,
                    None => {
                        report.skip("strength_not_pow2_const");
                        i += 1;
                        continue;
                    }
                };

                if !int32_proven.contains(&other) {
                    report.skip("strength_other_not_int32_proven");
                    i += 1;
                    continue;
                }

                let shift_reg = match shift_regs.get(&shift).copied() {
                    Some(reg) => reg,
                    None => {
                        let new_reg = AirReg(self.air.registers_count);
                        self.air.registers_count += 1;
                        block.insts.insert(
                            i,
                            AirOpcode::LoadInt32 {
                                dst: new_reg,
                                value: shift,
                            },
                        );
                        shift_regs.insert(shift, new_reg);
                        i += 1;
                        new_reg
                    }
                };

                block.insts[i] = AirOpcode::Shl {
                    dst,
                    lhs: other,
                    rhs: shift_reg,
                };
                report.strength_reduced += 1;
                i += 1;
            }
        }
    }

    fn apply_licm(
        &mut self,
        lp: &NaturalLoop,
        analysis: &LoopAnalysis,
        report: &mut LoopOptReport,
    ) {
        let mut defs_in_loop: HashMap<AirReg, usize> = HashMap::new();
        for &block_id in &lp.body {
            if let Some(block) = self.find_block(block_id) {
                for inst in &block.insts {
                    if let Some(dst) = inst.dst_reg() {
                        *defs_in_loop.entry(dst).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut body_ids: Vec<AirBlockId> = lp.body.iter().copied().collect();
        body_ids.sort_by_key(|id| id.0);

        let mut selected: Vec<(AirBlockId, usize, AirOpcode, AirReg)> = Vec::new();
        let mut selected_keys: HashSet<(AirBlockId, usize)> = HashSet::new();
        let mut hoisted_defs: HashSet<AirReg> = HashSet::new();

        let mut changed = true;
        while changed {
            changed = false;
            for &block_id in &body_ids {
                let block = match self.find_block(block_id) {
                    Some(b) => b,
                    None => continue,
                };
                for (inst_idx, inst) in block.insts.iter().enumerate() {
                    let dst = match inst.dst_reg() {
                        Some(dst) => dst,
                        None => continue,
                    };
                    if selected_keys.contains(&(block_id, inst_idx)) {
                        continue;
                    }
                    if defs_in_loop.get(&dst).copied().unwrap_or(0) != 1 {
                        report.skip("licm_dst_redefined_in_loop");
                        continue;
                    }
                    if !is_licm_safe_opcode(inst) {
                        report.skip("licm_opcode_not_safe");
                        continue;
                    }
                    if !self.operands_invariant(inst, lp, analysis, &hoisted_defs) {
                        report.skip("licm_operands_not_invariant");
                        continue;
                    }

                    selected.push((block_id, inst_idx, inst.clone(), dst));
                    selected_keys.insert((block_id, inst_idx));
                    hoisted_defs.insert(dst);
                    changed = true;
                }
            }
        }

        if selected.is_empty() {
            report.skip("licm_no_hoistable_insts");
            return;
        }

        let pre_header_id = self.ensure_preheader(lp, analysis);
        selected.sort_by_key(|(block_id, inst_idx, _, _)| (block_id.0, *inst_idx));

        let mut remove_map: HashMap<AirBlockId, Vec<usize>> = HashMap::new();
        let mut moved_insts = Vec::with_capacity(selected.len());
        for (block_id, inst_idx, inst, _) in selected {
            remove_map.entry(block_id).or_default().push(inst_idx);
            moved_insts.push(inst);
        }

        for (block_id, mut idxs) in remove_map {
            idxs.sort_by(|a, b| b.cmp(a));
            if let Some(block) = self.find_block_mut(block_id) {
                for idx in idxs {
                    if idx < block.insts.len() {
                        block.insts.remove(idx);
                    }
                }
            }
        }

        if let Some(pre_header) = self.find_block_mut(pre_header_id) {
            for inst in moved_insts {
                pre_header.insts.push(inst);
                report.licm_moved += 1;
            }
        }
    }

    fn apply_loop_unrolling(
        &mut self,
        lp: &NaturalLoop,
        analysis: &LoopAnalysis,
        report: &mut LoopOptReport,
    ) {
        let counted = match self.detect_counted_loop(lp, analysis) {
            Some(c) => c,
            None => {
                report.skip("unroll_not_canonical_or_not_exact");
                return;
            }
        };

        if counted.trip_count > self.config.max_unroll_trip {
            report.skip("unroll_trip_above_limit");
            return;
        }

        let latch_block = match self.find_block(counted.latch) {
            Some(b) => b.clone(),
            None => {
                report.skip("unroll_latch_not_found");
                return;
            }
        };

        let unrolled_id = AirBlockId(self.air.blocks.len() as u32);
        let mut unrolled = AirBlock::new(unrolled_id.0);
        for _ in 0..counted.trip_count {
            for inst in &latch_block.insts {
                unrolled.insts.push(inst.clone());
            }
        }
        unrolled.terminator = Some(AirTerminator::Jump(counted.exit));

        self.redirect_external_preds(lp, counted.header, unrolled_id, analysis);
        self.air.blocks.push(unrolled);
        report.unroll_applied += 1;
    }

    fn ensure_preheader(&mut self, lp: &NaturalLoop, analysis: &LoopAnalysis) -> AirBlockId {
        if let Some(id) = self.created_preheaders.get(&lp.header).copied() {
            return id;
        }
        if let Some(existing) = lp.preheader {
            self.created_preheaders.insert(lp.header, existing);
            return existing;
        }

        let new_id = AirBlockId(self.air.blocks.len() as u32);
        let mut pre_header = AirBlock::new(new_id.0);
        pre_header.terminator = Some(AirTerminator::Jump(lp.header));
        self.redirect_external_preds(lp, lp.header, new_id, analysis);
        self.air.blocks.push(pre_header);
        self.created_preheaders.insert(lp.header, new_id);
        new_id
    }

    fn redirect_external_preds(
        &mut self,
        lp: &NaturalLoop,
        from: AirBlockId,
        to: AirBlockId,
        analysis: &LoopAnalysis,
    ) {
        let external_preds: Vec<AirBlockId> = analysis
            .predecessors
            .get(&from)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|pred| !lp.body.contains(pred))
            .collect();

        for pred_id in external_preds {
            let Some(pred_block) = self.find_block_mut(pred_id) else {
                continue;
            };
            if let Some(term) = &mut pred_block.terminator {
                match term {
                    AirTerminator::Jump(target) if *target == from => *target = to,
                    AirTerminator::JumpIf {
                        then_blk, else_blk, ..
                    } => {
                        if *then_blk == from {
                            *then_blk = to;
                        }
                        if *else_blk == from {
                            *else_blk = to;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn operands_invariant(
        &self,
        inst: &AirOpcode,
        lp: &NaturalLoop,
        analysis: &LoopAnalysis,
        hoisted_defs: &HashSet<AirReg>,
    ) -> bool {
        for op in inst.operands() {
            if op.0 < self.air.num_params || hoisted_defs.contains(&op) {
                continue;
            }
            let Some(def_block) = analysis.reg_defs.get(&op).copied() else {
                return false;
            };
            if lp.body.contains(&def_block) {
                return false;
            }
        }
        true
    }

    fn collect_i32_constants(&self) -> HashMap<AirReg, i32> {
        let mut constants = HashMap::new();
        for block in &self.air.blocks {
            for inst in &block.insts {
                if let AirOpcode::LoadInt32 { dst, value } = *inst {
                    constants.insert(dst, value);
                }
            }
        }
        constants
    }

    fn find_block(&self, block_id: AirBlockId) -> Option<&AirBlock> {
        self.air.blocks.iter().find(|b| b.id == block_id)
    }

    fn find_block_mut(&mut self, block_id: AirBlockId) -> Option<&mut AirBlock> {
        self.air.blocks.iter_mut().find(|b| b.id == block_id)
    }

    fn find_def_block(&self, reg: AirReg) -> Option<AirBlockId> {
        for block in &self.air.blocks {
            for inst in &block.insts {
                if inst.dst_reg() == Some(reg) {
                    return Some(block.id);
                }
            }
        }
        None
    }

    fn compute_int32_proven_regs(&self, constants: &HashMap<AirReg, i32>) -> HashSet<AirReg> {
        let mut proven: HashSet<AirReg> = constants.keys().copied().collect();
        let mut changed = true;
        while changed {
            changed = false;
            for block in &self.air.blocks {
                for inst in &block.insts {
                    let Some(dst) = inst.dst_reg() else { continue };
                    let can_mark = match *inst {
                        AirOpcode::Move { src, .. } => proven.contains(&src),
                        AirOpcode::BitAnd { .. }
                        | AirOpcode::BitOr { .. }
                        | AirOpcode::BitXor { .. }
                        | AirOpcode::Shl { .. }
                        | AirOpcode::Shr { .. }
                        | AirOpcode::UShr { .. }
                        | AirOpcode::LoadBool { .. }
                        | AirOpcode::LoadNull { .. }
                        | AirOpcode::LoadUndefined { .. }
                        | AirOpcode::LoadInt32 { .. } => true,
                        _ => false,
                    };
                    if can_mark && proven.insert(dst) {
                        changed = true;
                    }
                }
            }
        }
        proven
    }

    fn resolve_i32_const_reg(&self, reg: AirReg, constants: &HashMap<AirReg, i32>) -> Option<i32> {
        let mut seen = HashSet::new();
        let mut cur = reg;
        loop {
            if !seen.insert(cur) {
                return None;
            }
            if let Some(v) = constants.get(&cur).copied() {
                return Some(v);
            }
            let def_block = self.find_def_block(cur)?;
            let block = self.find_block(def_block)?;
            let def_inst = block
                .insts
                .iter()
                .find(|inst| inst.dst_reg() == Some(cur))?;
            match *def_inst {
                AirOpcode::Move { src, .. } => cur = src,
                _ => return None,
            }
        }
    }

    fn find_induction_step(
        &self,
        lp: &NaturalLoop,
        analysis: &LoopAnalysis,
        induction: AirReg,
        constants: &HashMap<AirReg, i32>,
    ) -> Option<i32> {
        let latch = self.find_block(lp.latch)?;
        for inst in &latch.insts {
            match *inst {
                AirOpcode::Add { dst, lhs, rhs, .. } if dst == induction => {
                    if lhs == induction {
                        return self.resolve_i32_const_reg(rhs, constants);
                    }
                    if rhs == induction {
                        return self.resolve_i32_const_reg(lhs, constants);
                    }
                }
                AirOpcode::Sub { dst, lhs, rhs } if dst == induction && lhs == induction => {
                    return self.resolve_i32_const_reg(rhs, constants).map(|v| -v);
                }
                AirOpcode::Move { dst, src } if dst == induction => {
                    if let Some(def_block) = analysis.reg_defs.get(&src) {
                        if *def_block != lp.latch {
                            continue;
                        }
                    }
                    let src_def = latch.insts.iter().find(|c| c.dst_reg() == Some(src));
                    if let Some(src_def) = src_def {
                        match *src_def {
                            AirOpcode::Add { lhs, rhs, .. } => {
                                if lhs == induction {
                                    return self.resolve_i32_const_reg(rhs, constants);
                                }
                                if rhs == induction {
                                    return self.resolve_i32_const_reg(lhs, constants);
                                }
                            }
                            AirOpcode::Sub { lhs, rhs, .. } if lhs == induction => {
                                return self.resolve_i32_const_reg(rhs, constants).map(|v| -v);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn detect_counted_loop(
        &self,
        lp: &NaturalLoop,
        analysis: &LoopAnalysis,
    ) -> Option<CountedLoop> {
        if lp.body.len() != 2 || !lp.body.contains(&lp.header) || !lp.body.contains(&lp.latch) {
            return None;
        }
        let header = self.find_block(lp.header)?;
        let latch = self.find_block(lp.latch)?;
        let (cond_reg, exit_blk) = match header.terminator {
            Some(AirTerminator::JumpIf {
                cond,
                then_blk,
                else_blk,
            }) => {
                if then_blk == lp.latch {
                    (cond, else_blk)
                } else if else_blk == lp.latch {
                    (cond, then_blk)
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        match latch.terminator {
            Some(AirTerminator::Jump(target)) if target == lp.header => {}
            _ => return None,
        }
        let cond_inst = header
            .insts
            .iter()
            .find(|inst| inst.dst_reg() == Some(cond_reg))?;
        let (cmp_kind, ind_reg, limit_reg) = match *cond_inst {
            AirOpcode::Lt { lhs, rhs, .. } => (CmpKind::Lt, lhs, rhs),
            AirOpcode::Lte { lhs, rhs, .. } => (CmpKind::Lte, lhs, rhs),
            AirOpcode::Gt { lhs, rhs, .. } => (CmpKind::Gt, lhs, rhs),
            AirOpcode::Gte { lhs, rhs, .. } => (CmpKind::Gte, lhs, rhs),
            _ => return None,
        };
        let constants = self.collect_i32_constants();
        let init = self.resolve_i32_const_reg(ind_reg, &constants)?;
        let limit = self.resolve_i32_const_reg(limit_reg, &constants)?;
        let step = self.find_induction_step(lp, analysis, ind_reg, &constants)?;
        let trip_count = compute_trip_count(init, limit, step, cmp_kind)?;
        Some(CountedLoop {
            header: lp.header,
            latch: lp.latch,
            exit: exit_blk,
            trip_count,
        })
    }

    fn debug_log(&self, msg: &str) {
        if self.config.debug {
            eprintln!("{msg}");
        }
    }
}

fn is_licm_safe_opcode(inst: &AirOpcode) -> bool {
    matches!(
        inst,
        AirOpcode::LoadInt32 { .. }
            | AirOpcode::LoadFloat64 { .. }
            | AirOpcode::LoadInt64 { .. }
            | AirOpcode::LoadBool { .. }
            | AirOpcode::LoadUndefined { .. }
            | AirOpcode::LoadNull { .. }
            | AirOpcode::LoadString { .. }
            | AirOpcode::Move { .. }
    )
}

fn extract_pow2_mul(
    lhs: AirReg,
    rhs: AirReg,
    constants: &HashMap<AirReg, i32>,
) -> Option<(AirReg, i32)> {
    let lhs_const = constants.get(&lhs).copied();
    let rhs_const = constants.get(&rhs).copied();
    if let Some(v) = lhs_const {
        if v > 0 && (v as u32).is_power_of_two() {
            return Some((rhs, v.trailing_zeros() as i32));
        }
    }
    if let Some(v) = rhs_const {
        if v > 0 && (v as u32).is_power_of_two() {
            return Some((lhs, v.trailing_zeros() as i32));
        }
    }
    None
}

#[derive(Debug, Clone, Copy)]
enum CmpKind {
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, Copy)]
struct CountedLoop {
    header: AirBlockId,
    latch: AirBlockId,
    exit: AirBlockId,
    trip_count: u32,
}

fn compute_trip_count(init: i32, limit: i32, step: i32, cmp: CmpKind) -> Option<u32> {
    if step == 0 {
        return None;
    }
    let trip = match cmp {
        CmpKind::Lt if step > 0 => {
            if init >= limit {
                0
            } else {
                let delta = (limit as i64) - (init as i64);
                ((delta + step as i64 - 1) / step as i64) as i32
            }
        }
        CmpKind::Lte if step > 0 => {
            if init > limit {
                0
            } else {
                let delta = (limit as i64) - (init as i64);
                (delta / step as i64 + 1) as i32
            }
        }
        CmpKind::Gt if step < 0 => {
            if init <= limit {
                0
            } else {
                let delta = (init as i64) - (limit as i64);
                let abs_step = (-step) as i64;
                ((delta + abs_step - 1) / abs_step) as i32
            }
        }
        CmpKind::Gte if step < 0 => {
            if init < limit {
                0
            } else {
                let delta = (init as i64) - (limit as i64);
                let abs_step = (-step) as i64;
                (delta / abs_step + 1) as i32
            }
        }
        _ => return None,
    };
    if trip < 0 {
        return None;
    }
    Some(trip as u32)
}

pub fn validate_air_cfg(air: &AirFunction) -> Result<(), String> {
    if !air.is_valid() {
        return Err("AIR function is not structurally valid".to_string());
    }

    let mut ids = HashSet::new();
    for block in &air.blocks {
        if !ids.insert(block.id) {
            return Err(format!("duplicate block id {}", block.id.0));
        }
    }
    let all_ids: HashSet<AirBlockId> = air.blocks.iter().map(|b| b.id).collect();

    for block in &air.blocks {
        for inst in &block.insts {
            if let Some(dst) = inst.dst_reg() {
                if dst.0 >= air.registers_count {
                    return Err(format!("dst register {} out of bounds", dst.0));
                }
            }
            for op in inst.operands() {
                if op.0 >= air.registers_count {
                    return Err(format!("operand register {} out of bounds", op.0));
                }
            }
        }
        match block.terminator {
            Some(AirTerminator::Jump(target)) => {
                if !all_ids.contains(&target) {
                    return Err(format!("jump target {} does not exist", target.0));
                }
            }
            Some(AirTerminator::JumpIf {
                cond,
                then_blk,
                else_blk,
            }) => {
                if cond.0 >= air.registers_count {
                    return Err(format!("jump-if cond {} out of bounds", cond.0));
                }
                if !all_ids.contains(&then_blk) {
                    return Err(format!("then target {} does not exist", then_blk.0));
                }
                if !all_ids.contains(&else_blk) {
                    return Err(format!("else target {} does not exist", else_blk.0));
                }
            }
            Some(AirTerminator::Return(reg)) => {
                if reg.0 >= air.registers_count {
                    return Err(format!("return reg {} out of bounds", reg.0));
                }
            }
            None => return Err(format!("block {} has no terminator", block.id.0)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::AirBuilder;

    fn run_default(air: &mut AirFunction) -> LoopOptReport {
        let mut optimizer = LoopOptimizer::new(air);
        optimizer.run()
    }

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
        let report = run_default(&mut air);

        assert!(report.validation_passed);
        assert!(report.licm_moved >= 1);
        assert!(air.blocks.len() >= 4);
    }

    #[test]
    fn test_licm_does_not_hoist_impure_arithmetic() {
        let mut builder = AirBuilder::new("licm_no_arith", 1, 0);
        let b_cond = builder.create_block();
        let b_body = builder.create_block();
        let b_exit = builder.create_block();

        let n = builder.param(0);
        let i = builder.emit_load_int32(0);
        builder.emit_jump(b_cond);

        builder.switch_block(b_cond);
        let cond = builder.emit_lt(i, n);
        builder.emit_jump_if(cond, b_body, b_exit);

        builder.switch_block(b_body);
        let one = builder.emit_load_int32(1);
        let i_next = builder.emit_add(i, one);
        builder.emit_move(i, i_next);
        builder.emit_jump(b_cond);

        builder.switch_block(b_exit);
        builder.emit_return(i);

        let mut air = builder.build();
        let _ = run_default(&mut air);

        let add_count: usize = air
            .blocks
            .iter()
            .map(|b| {
                b.insts
                    .iter()
                    .filter(|i| matches!(i, AirOpcode::Add { .. }))
                    .count()
            })
            .sum();
        assert!(add_count >= 1, "Add should remain in loop");
    }

    #[test]
    fn test_strength_reduction_requires_int32_proof() {
        let mut builder = AirBuilder::new("test_sr_no", 2, 0);
        let c8 = builder.emit_load_int32(8);
        let p = builder.param(1);
        let res = builder.emit_mul(c8, p);
        builder.emit_return(res);

        let mut air = builder.build();
        let report = run_default(&mut air);

        let block = &air.blocks[0];
        let has_shl = block
            .insts
            .iter()
            .any(|inst| matches!(inst, AirOpcode::Shl { .. }));
        assert!(!has_shl);
        assert_eq!(report.strength_reduced, 0);
    }

    #[test]
    fn test_strength_reduction_when_int32_proven() {
        let mut builder = AirBuilder::new("test_sr_yes", 0, 0);
        let int32_proven = builder.emit_load_int32(7);
        let c8 = builder.emit_load_int32(8);
        let res = builder.emit_mul(int32_proven, c8);
        builder.emit_return(res);

        let mut air = builder.build();
        let report = run_default(&mut air);

        let block = &air.blocks[0];
        let has_shl = block
            .insts
            .iter()
            .any(|inst| matches!(inst, AirOpcode::Shl { .. }));
        assert!(has_shl);
        assert!(report.strength_reduced >= 1);
    }

    #[test]
    fn test_loop_unrolling_exact_trip_count() {
        let mut builder = AirBuilder::new("test_unroll", 0, 0);
        let b_cond = builder.create_block();
        let b_body = builder.create_block();
        let b_exit = builder.create_block();

        let i = builder.emit_load_int32(0);
        builder.emit_jump(b_cond);

        // Header do loop (b_cond)
        builder.switch_block(b_cond);
        let limit = builder.emit_load_int32(3);
        let cond = builder.emit_lt(i, limit);
        builder.emit_jump_if(cond, b_body, b_exit);

        builder.switch_block(b_body);
        let one = builder.emit_load_int32(1);
        let i_next = builder.emit_add(i, one);
        builder.emit_move(i, i_next);
        builder.emit_jump(b_cond);

        builder.switch_block(b_exit);
        builder.emit_return(i);

        let mut air = builder.build();
        let before_blocks = air.blocks.len();
        let report = run_default(&mut air);

        assert!(report.unroll_applied >= 1);
        assert!(air.blocks.len() > before_blocks);
        assert!(validate_air_cfg(&air).is_ok());
    }

    #[test]
    fn test_validate_air_cfg_rejects_bad_jump() {
        let mut b = AirBuilder::new("bad_cfg", 0, 0);
        let target = AirBlockId(99);
        b.emit_jump(target);
        let air = b.build();
        assert!(validate_air_cfg(&air).is_err());
    }
}
