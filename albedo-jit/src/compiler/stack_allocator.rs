//! # Stack Allocator for AlbedoJIT
//!
//! Gerencia alocação de objetos no stack para escape analysis.

use crate::bytecode::AirReg;
use std::collections::HashMap;

/// Alocação de um objeto no stack
#[derive(Debug, Clone)]
pub struct StackAllocation {
    pub offset: i32,      // Offset relativo ao frame pointer (rbp)
    pub size: i32,        // Tamanho em bytes
    pub num_properties: usize,
}

/// Gerenciador de alocação no stack
#[derive(Debug, Default)]
pub struct StackAllocator {
    /// Offset atual no stack frame
    current_offset: i32,
    /// Tamanho total do stack frame
    frame_size: i32,
    /// Mapeamento: obj_reg → stack allocation
    allocations: HashMap<AirReg, StackAllocation>,
}

impl StackAllocator {
    pub fn new() -> Self {
        Self {
            current_offset: 0,
            frame_size: 0,
            allocations: HashMap::new(),
        }
    }

    /// Alocar espaço para um objeto no stack
    /// Layout: [prop0: JsValue][prop1: JsValue]...[padding]
    pub fn allocate_object(&mut self, obj_reg: AirReg, num_properties: usize) -> StackAllocation {
        // Alinhar a 8 bytes (tamanho de u64/JsValue)
        let prop_size = std::mem::size_of::<u64>() as i32;
        let size = (num_properties as i32 * prop_size + 7) & !7; // align to 8

        let offset = self.current_offset;
        self.current_offset += size;
        self.frame_size = self.current_offset;

        let alloc = StackAllocation {
            offset,
            size,
            num_properties,
        };

        self.allocations.insert(obj_reg, alloc.clone());
        alloc
    }

    /// Obter alocação de um objeto
    pub fn get_allocation(&self, obj_reg: AirReg) -> Option<&StackAllocation> {
        self.allocations.get(&obj_reg)
    }

    /// Obter todos os allocations
    pub fn get_all_allocations(&self) -> &HashMap<AirReg, StackAllocation> {
        &self.allocations
    }

    /// Obter tamanho total do frame
    pub fn get_frame_size(&self) -> i32 {
        self.frame_size
    }

    /// Calcular offset de uma propriedade
    pub fn get_property_offset(&self, obj_reg: AirReg, prop_index: usize) -> Option<i32> {
        self.allocations.get(&obj_reg).map(|alloc| {
            alloc.offset + (prop_index as i32 * std::mem::size_of::<u64>() as i32)
        })
    }

    /// Limpar todas as alocações
    pub fn clear(&mut self) {
        self.current_offset = 0;
        self.frame_size = 0;
        self.allocations.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_object() {
        let mut allocator = StackAllocator::new();
        let alloc = allocator.allocate_object(AirReg(0), 3);

        assert_eq!(alloc.num_properties, 3);
        assert_eq!(alloc.size, 24); // 3 * 8 bytes
        assert_eq!(alloc.offset, 0);
        assert_eq!(allocator.get_frame_size(), 24);
    }

    #[test]
    fn test_multiple_allocations() {
        let mut allocator = StackAllocator::new();
        let alloc1 = allocator.allocate_object(AirReg(0), 2);
        let alloc2 = allocator.allocate_object(AirReg(1), 3);

        assert_eq!(alloc1.offset, 0);
        assert_eq!(alloc1.size, 16); // 2 * 8

        assert_eq!(alloc2.offset, 16); // after alloc1
        assert_eq!(alloc2.size, 24); // 3 * 8

        assert_eq!(allocator.get_frame_size(), 40);
    }

    #[test]
    fn test_get_property_offset() {
        let mut allocator = StackAllocator::new();
        allocator.allocate_object(AirReg(0), 5);

        assert_eq!(allocator.get_property_offset(AirReg(0), 0), Some(0));
        assert_eq!(allocator.get_property_offset(AirReg(0), 1), Some(8));
        assert_eq!(allocator.get_property_offset(AirReg(0), 2), Some(16));
        assert_eq!(allocator.get_property_offset(AirReg(0), 3), Some(24));
        assert_eq!(allocator.get_property_offset(AirReg(0), 4), Some(32));
    }
}
