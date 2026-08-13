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

struct LogConfig {
    global_level: LogLevel,
    module_levels: std::collections::HashMap<String, LogLevel>,
    is_json: bool,
}

/// Extrai atomicamente (OnceLock) a configuração nativa do motor Albedo (ACE_LOG).
fn get_log_config() -> &'static LogConfig {
    static CONFIG: OnceLock<LogConfig> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let is_json = env::var("ACE_LOG_JSON").map(|v| v == "1" || v == "true").unwrap_or(false);
        
        let mut global_level = LogLevel::Info;
        let mut module_levels = std::collections::HashMap::new();

        if let Ok(val) = env::var("ACE_LOG") {
            for part in val.split(',') {
                let part = part.trim();
                if let Some((mod_name, level_str)) = part.split_once('=') {
                    if let Some(level) = parse_level(level_str) {
                        module_levels.insert(mod_name.trim().to_string(), level);
                    }
                } else if let Some(level) = parse_level(part) {
                    global_level = level;
                }
            }
        }

        LogConfig {
            global_level,
            module_levels,
            is_json,
        }
    })
}

fn parse_level(s: &str) -> Option<LogLevel> {
    match s.trim().to_uppercase().as_str() {
        "TRACE" => Some(LogLevel::Trace),
        "DEBUG" => Some(LogLevel::Debug),
        "INFO" => Some(LogLevel::Info),
        "WARN" => Some(LogLevel::Warn),
        "ERROR" => Some(LogLevel::Error),
        _ => None,
    }
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
    let config = get_log_config();
    
    // Verifica se este módulo tem um level específico, senão usa o global
    let threshold = config.module_levels.get(target).unwrap_or(&config.global_level);
    
    if level >= *threshold {
        let msg = if config.is_json {
            // Escapa as aspas duplas na mensagem para JSON válido
            let escaped_args = format!("{}", args).replace("\"", "\\\"");
            format!(
                r#"{{"level":"{}","module":"{}","msg":"{}"}}"#,
                level.as_str().trim(), target, escaped_args
            )
        } else {
            format!("[{}] [{}] {}", level.as_str(), target, args)
        };
        
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
