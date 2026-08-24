//! # Eventos e Primitivas de Interação com o Usuário
//!
//! Tipos fundamentais e agnósticos para captura de entrada do usuário (mouse, teclado, touch e ponteiros).

pub mod input;

pub use input::{KeyLocation, ModifiersState, PointerButton, PointerButtons, PointerType};
