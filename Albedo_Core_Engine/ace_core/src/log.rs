// ============================================================================
// Albedo Core Engine (ACE)
// File: log.rs
// Description: Infraestrutura manual de Logging, Telemetria e Observabilidade.
//              Isolamento garantido sem a crate `tracing` ou `env_logger`.
// Author: Albedo Browser Engineering Team
// ============================================================================

//! # Observabilidade Core
//!
//! O motor base precisa reportar anomalias e rotinas assíncronas do Event Loop de
//! forma otimizada. Usar Stdout serializado sem blocos síncronos longos.
//! Todo módulo tem acesso a macros para output (JSON ou Texto) condicionado
//! pela variável ambiental global `ACE_LOG`.

use std::env;
use std::io::{self, Write};
use std::sync::mpsc::{self, SyncSender};
use std::sync::OnceLock;
use std::thread;

// ----------------------------------------------------------------------------
// Log Configuration
// ----------------------------------------------------------------------------

/// Hierarquia restrita de severidade dos Logs no Ecossistema Albedo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Debug massivo e rotinas microscópicas (ex: cada instrução JIT).
    Trace,
    /// Mensagens de acompanhamento de engenharia.
    Debug,
    /// Status nominal das transições e vida útil do aplicativo.
    Info,
    /// Alertas reversíveis (ex: rede lenta, TLS fallback).
    Warn,
    /// Pânico interno recuperável mas que precisa intervir ou abater rotinas.
    Error,
}

impl LogLevel {
    /// Retorna a string serializada fixa para output em terminal.
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO ",
            LogLevel::Warn => "WARN ",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Extrai atomicamente (OnceLock) a configuração nativa do motor Albedo (ACE_LOG).
pub fn current_log_level() -> LogLevel {
    static LEVEL: OnceLock<LogLevel> = OnceLock::new();
    *LEVEL.get_or_init(|| {
        match env::var("ACE_LOG")
            .unwrap_or_else(|_| "INFO".to_string())
            .to_uppercase()
            .as_str()
        {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    })
}

// ----------------------------------------------------------------------------
// Sink de Escrita (Low Level Sink)
// ----------------------------------------------------------------------------

/// Singleton que guarda o transmissor do Logger Assíncrono.
/// Inicializa a Thread Operária em background na primeira chamada.
fn get_logger_sender() -> &'static SyncSender<String> {
    static SENDER: OnceLock<SyncSender<String>> = OnceLock::new();
    SENDER.get_or_init(|| {
        // Criamos uma fila MPMC de alta capacidade para evitar backpressure
        let (tx, rx) = mpsc::sync_channel::<String>(10000);
        
        // Spawna a Thead Operária (I/O)
        thread::Builder::new()
            .name("ACE_Logger".to_string())
            .spawn(move || {
                let stderr = io::stderr();
                // A trava (lock) ocorre apenas nesta Thread, liberando a Main Thread.
                let mut handle = stderr.lock();
                while let Ok(msg) = rx.recv() {
                    let _ = writeln!(handle, "{}", msg);
                }
            })
            .expect("Falha ao spawnar a Thread de Logs");
            
        tx
    })
}

/// Executor base lock-free: formata a mensagem na RAM e dispara pelo canal.
#[doc(hidden)]
pub fn _log(level: LogLevel, target: &str, args: std::fmt::Arguments) {
    if level >= current_log_level() {
        // Formata a string no Heap da Main Thread
        let msg = format!("[{}] [{}] {}", level.as_str(), target, args);
        
        let tx = get_logger_sender();
        // Dispara de forma Não-Bloqueante (Non-Blocking). Se o terminal for excessivamente lento
        // e o buffer atingir 10.000, as mensagens seguintes são dropadas até esvaziar,
        // garantindo que o `EventLoop` jamais engasgue.
        let _ = tx.try_send(msg);
    }
}

// ----------------------------------------------------------------------------
// Macros Públicas (Fronteira Externa)
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! ace_trace {
    ($($arg:tt)+) => ($crate::log::_log($crate::log::LogLevel::Trace, module_path!(), format_args!($($arg)+)))
}

#[macro_export]
macro_rules! ace_debug {
    ($($arg:tt)+) => ($crate::log::_log($crate::log::LogLevel::Debug, module_path!(), format_args!($($arg)+)))
}

#[macro_export]
macro_rules! ace_info {
    ($($arg:tt)+) => ($crate::log::_log($crate::log::LogLevel::Info, module_path!(), format_args!($($arg)+)))
}

#[macro_export]
macro_rules! ace_warn {
    ($($arg:tt)+) => ($crate::log::_log($crate::log::LogLevel::Warn, module_path!(), format_args!($($arg)+)))
}

#[macro_export]
macro_rules! ace_error {
    ($($arg:tt)+) => ($crate::log::_log($crate::log::LogLevel::Error, module_path!(), format_args!($($arg)+)))
}
