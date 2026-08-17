//! # Padrão Observer Reentrante
//!
//! Gerenciamento de eventos e observadores imunes a perigos de reentrância (adicionar/remover ouvintes durante notificações).

pub mod list;

pub use list::ObserverList;
