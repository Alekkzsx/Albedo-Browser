//! # Especificação WebIDL e Despacho de Tipos (WHATWG WebIDL Standard)
//!
//! Tipagem normativa, conversão de argumentos, tratamento de exceções DOM
//! e macros de geração de despacho para interfaces WebIDL.

use crate::error::DomError;
use smol_str::SmolStr;
use std::fmt;

/// Valores dinâmicos padronizados no ecossistema WebIDL.
#[derive(Debug, Clone, PartialEq)]
pub enum JSValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(SmolStr),
    Object(super::wrapper::JSObjectId),
    Array(Vec<JSValue>),
}

impl JSValue {
    /// Extrai o valor como booleano normativo.
    pub fn to_boolean(&self) -> bool {
        match self {
            Self::Undefined | Self::Null => false,
            Self::Boolean(b) => *b,
            Self::Number(n) => *n != 0.0 && !n.is_nan(),
            Self::String(s) => !s.is_empty(),
            Self::Object(_) | Self::Array(_) => true,
        }
    }

    /// Extrai o valor como string WebIDL (`DOMString`).
    pub fn to_dom_string(&self) -> SmolStr {
        match self {
            Self::Undefined => SmolStr::new("undefined"),
            Self::Null => SmolStr::new("null"),
            Self::Boolean(b) => SmolStr::new(if *b { "true" } else { "false" }),
            Self::Number(n) => SmolStr::new(format!("{n}")),
            Self::String(s) => s.clone(),
            Self::Object(id) => SmolStr::new(format!("[object Object#{}", id.0)),
            Self::Array(_) => SmolStr::new("[object Array]"),
        }
    }

    /// Retorna `true` se o valor for nulo ou indefinido.
    #[inline]
    pub fn is_null_or_undefined(&self) -> bool {
        matches!(self, Self::Undefined | Self::Null)
    }
}

/// Exceções normativas WebIDL (DOMException).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebIDLException {
    TypeError(String),
    HierarchyRequestError(String),
    NotFoundError(String),
    NotSupportedError(String),
    InvalidStateError(String),
    SyntaxError(String),
    SecurityError(String),
}

impl fmt::Display for WebIDLException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeError(msg) => write!(f, "TypeError: {msg}"),
            Self::HierarchyRequestError(msg) => write!(f, "HierarchyRequestError: {msg}"),
            Self::NotFoundError(msg) => write!(f, "NotFoundError: {msg}"),
            Self::NotSupportedError(msg) => write!(f, "NotSupportedError: {msg}"),
            Self::InvalidStateError(msg) => write!(f, "InvalidStateError: {msg}"),
            Self::SyntaxError(msg) => write!(f, "SyntaxError: {msg}"),
            Self::SecurityError(msg) => write!(f, "SecurityError: {msg}"),
        }
    }
}

impl std::error::Error for WebIDLException {}

impl From<DomError> for WebIDLException {
    fn from(err: DomError) -> Self {
        match err {
            DomError::HierarchyRequestError(msg) => WebIDLException::HierarchyRequestError(msg),
            DomError::NotFoundError(msg) => WebIDLException::NotFoundError(msg),
            DomError::NotSupportedError(msg) => WebIDLException::NotSupportedError(msg),
            DomError::InvalidStateError(msg) => WebIDLException::InvalidStateError(msg),
            DomError::SyntaxError(msg) => WebIDLException::SyntaxError(msg),
            DomError::SecurityError(msg) => WebIDLException::SecurityError(msg),
            other => WebIDLException::TypeError(other.to_string()),
        }
    }
}

/// Resultado normativo de uma invocação WebIDL.
pub type WebIDLResult<T> = Result<T, WebIDLException>;

/// Macro para definição padronizada de interfaces WebIDL com reflexão de getters/setters e métodos.
#[macro_export]
macro_rules! define_webidl_interface {
    (
        interface $interface_name:ident {
            $(
                $(#[doc = $attr_doc:expr])*
                attribute $attr_name:ident : $attr_type:ident [ get => $getter:ident $(, set => $setter:ident)? ];
            )*
            $(
                $(#[doc = $method_doc:expr])*
                operation $method_name:ident ( $( $param:ident : $param_type:ident ),* ) -> $ret_type:ident => $method_impl:ident;
            )*
        }
    ) => {
        pub struct $interface_name;

        impl $interface_name {
            pub const NAME: &'static str = stringify!($interface_name);
        }
    };
}
