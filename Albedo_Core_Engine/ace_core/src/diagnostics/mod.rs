//! # Diagnóstico de Falhas, CrashKeys e Rastro de Execução
//!
//! Ferramentas de triage de produção inspiradas no Crashpad do Chromium.

pub mod breadcrumbs;
pub mod crash_keys;

pub use breadcrumbs::{BreadcrumbBuffer, BreadcrumbEntry};
pub use crash_keys::CrashKeyRegistry;
