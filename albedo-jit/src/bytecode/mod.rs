//! Módulo de Bytecode intermediário do AlbedoJIT (AIR).

pub mod builder;
pub mod opcodes;

pub use builder::AirBuilder;
pub use opcodes::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_simple_add() {
        // add(a, b) -> a + b
        let mut builder = AirBuilder::new("add", 2, 0);
        
        let a = builder.param(0);
        let b = builder.param(1);
        let sum = builder.emit_add(a, b);
        
        builder.emit_return(sum);
        
        let func = builder.build();
        
        // Verifica metadata
        assert_eq!(func.name, "add");
        assert_eq!(func.num_params, 2);
        assert_eq!(func.registers_count, 3); // 2 params + 1 local (sum)
        assert_eq!(func.blocks.len(), 1); // Apenas entry block
        
        // Verifica instruções
        let blk = &func.blocks[0];
        assert_eq!(blk.insts.len(), 1);
        match &blk.insts[0] {
            AirOpcode::Add { dst, lhs, rhs, ic_slot: _ } => {
                assert_eq!(*dst, AirReg(2));
                assert_eq!(*lhs, AirReg(0));
                assert_eq!(*rhs, AirReg(1));
            }
            other => panic!("Esperado Add, veio {:?}", other),
        }
        assert_eq!(blk.terminator, Some(AirTerminator::Return(AirReg(2))));
    }

    #[test]
    fn test_build_fibonacci_skeleton() {
        // Skeleton para branch conditional
        // if (n < 2) return n; else return fib(n-1) + fib(n-2);
        let mut builder = AirBuilder::new("fib", 1, 0);
        
        // Blocos
        let b_then = builder.create_block();
        let b_else = builder.create_block();
        
        // --- Entry Block (b0) ---
        let n = builder.param(0);
        let two = builder.emit_load_int32(2);
        let cond = builder.emit_lt(n, two);
        builder.emit_jump_if(cond, b_then, b_else);
        
        // --- Then Block (b1) ---
        builder.switch_block(b_then);
        builder.emit_return(n);
        
        // --- Else Block (b2) ---
        builder.switch_block(b_else);
        let um = builder.emit_load_int32(1);
        let sub = builder.emit_sub(n, um);
        // (omitida a chamada recursiva por simplicidade do teste do esqueleto)
        builder.emit_return(sub);
        
        let func = builder.build();
        assert_eq!(func.blocks.len(), 3);
        assert!(func.is_valid());
    }
}
