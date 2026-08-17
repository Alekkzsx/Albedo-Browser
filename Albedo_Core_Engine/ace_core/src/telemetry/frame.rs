//! # Rastreador de Orçamento de Frame e Detecção de Jank a 120 FPS
//!
//! Telemetria de alta resolução para monitoramento contínuo dos estágios de renderização
//! (DOM -> Style -> Layout -> Paint -> Composite) com detecção de frames descartados.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Estágios principais do pipeline de renderização do quadro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameStage {
    DomEvents = 0,
    StyleRecalc = 1,
    Layout = 2,
    Paint = 3,
    Composite = 4,
}

/// Métricas consolidadas de um único quadro renderizado.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameMetrics {
    /// Número sequencial do quadro.
    pub frame_number: u64,
    /// Duração total do quadro em milissegundos.
    pub total_duration_ms: f32,
    /// Orçamento alocado para o frame (ex: 8.33ms para 120 FPS ou 16.66ms para 60 FPS).
    pub budget_ms: f32,
    /// Indica se o quadro estourou o orçamento de tempo (*Jank / Frame Drop*).
    pub is_jank: bool,
    /// Tempo individual gasto em cada estágio em milissegundos.
    pub stages_ms: [f32; 5],
}

/// Rastreador de desempenho e orçamento de taxa de quadros (Frame Budget).
pub struct FrameBudgetTracker {
    budget_ms: f32,
    frame_counter: AtomicU64,
    jank_counter: AtomicU64,
}

impl FrameBudgetTracker {
    /// Cria um novo rastreador com o orçamento por frame especificado (ex: 8.333 para 120 FPS).
    pub fn new(budget_ms: f32) -> Self {
        Self {
            budget_ms,
            frame_counter: AtomicU64::new(0),
            jank_counter: AtomicU64::new(0),
        }
    }

    /// Cria um rastreador configurado para a taxa padrão de 60 FPS (16.666 ms).
    pub fn for_60hz() -> Self {
        Self::new(16.6666)
    }

    /// Cria um rastreador configurado para monitores de alta taxa de atualização de 120 FPS (8.333 ms).
    pub fn for_120hz() -> Self {
        Self::new(8.3333)
    }

    /// Inicia a medição de um novo quadro retornando um gravador de estágios RAII.
    pub fn begin_frame(&self) -> FrameRecorder {
        let frame_num = self.frame_counter.fetch_add(1, Ordering::Relaxed) + 1;
        FrameRecorder {
            frame_number: frame_num,
            budget_ms: self.budget_ms,
            start_time: Instant::now(),
            stages_ms: [0.0; 5],
        }
    }

    /// Registra e consolida um quadro concluído.
    pub fn finish_frame(&self, recorder: FrameRecorder) -> FrameMetrics {
        let total_ms = recorder.start_time.elapsed().as_secs_f32() * 1000.0;
        let is_jank = total_ms > self.budget_ms;

        if is_jank {
            self.jank_counter.fetch_add(1, Ordering::Relaxed);
        }

        FrameMetrics {
            frame_number: recorder.frame_number,
            total_duration_ms: total_ms,
            budget_ms: self.budget_ms,
            is_jank,
            stages_ms: recorder.stages_ms,
        }
    }

    /// Retorna o total de quadros renderizados.
    #[inline]
    pub fn total_frames(&self) -> u64 {
        self.frame_counter.load(Ordering::Relaxed)
    }

    /// Retorna o total de quadros com engasgo (*Jank / Dropped Frames*).
    #[inline]
    pub fn total_janks(&self) -> u64 {
        self.jank_counter.load(Ordering::Relaxed)
    }
}

/// Gravador de tempos por estágio para um quadro em andamento.
pub struct FrameRecorder {
    frame_number: u64,
    budget_ms: f32,
    start_time: Instant,
    stages_ms: [f32; 5],
}

impl FrameRecorder {
    /// Retorna o número deste quadro.
    #[inline]
    pub const fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Retorna o orçamento deste quadro.
    #[inline]
    pub const fn budget_ms(&self) -> f32 {
        self.budget_ms
    }

    /// Registra a duração de um estágio específico.
    pub fn record_stage(&mut self, stage: FrameStage, duration_ms: f32) {
        self.stages_ms[stage as usize] = duration_ms;
    }
}

