//! # Albedo Core Engine (ACE) - Core Foundation
//!
//! Bem-vindo à camada fundacional do **Albedo Browser**.
//!
//! A crate `ace_core` atua como o sistema nervoso central do navegador, provendo:
//! - **Geometria & Matemática Tipada** ([`math`]): Espaços de coordenadas (`CssPixel`, `DevicePixel`), matrizes afins 2D/3D e cores CSS3/4.
//! - **Gestão de Memória Anti-Ciclos** ([`arena`]): Arenas geracionais (`Arena`) e bump allocation (`BumpArena`) para nós DOM e payloads sem `Rc<RefCell>`.
//! - **Event Loop WHATWG** ([`event_loop`]): Despacho de tarefas por `TaskSource`, filas dinâmicas de microtasks e suporte a `requestAnimationFrame`.
//! - **Identificadores Globais** ([`id`]): Newtypes atômicos fortemente tipados (`NodeId`, `TabId`, `FrameId`, `TaskId`, etc.).
//! - **String Pooling O(1)** ([`intern`]): Átomos estáticos e dinâmicos para tags HTML e propriedades CSS.
//! - **Tratamento Rico de Erros** ([`error`]): `AceError` com rastreabilidade de código-fonte (`SourceLocation`).
//! - **Coleções Especializadas** ([`collections`]): BitSets sem alocação heap para dirty tracking de nós.
//! - **Resiliência CPU** ([`task`]): Execução com captura de pânico (`spawn_safe`).
//! - **Relógios Determinísticos** ([`time`]): `MonotonicClock` de produção e `MockClock` para testes determinísticos.

pub mod arena;
pub mod collections;
pub mod error;
pub mod event_loop;
pub mod id;
pub mod intern;
pub mod math;
pub mod task;
pub mod time;
pub mod utils;

// Re-exports estratégicos para uso em todo o workspace Albedo
pub use anyhow::{Context, Result};
pub use euclid;
pub use smol_str::SmolStr;
pub use tracing::{debug, error, info, trace, warn};
