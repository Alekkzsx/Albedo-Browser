//! # Linha do Tempo de Performance W3C (Performance Timeline & User Timing)
//!
//! Primitivas para marcas de tempo de alta resolução, medição de duração de passes gráficos (layout/style)
//! e perfilamento para integração com o DevTools.

use crate::error::AceError;
use crate::time::{Clock, MonotonicClock};
use parking_lot::Mutex;
use smol_str::SmolStr;
use std::sync::Arc;

/// Uma marca de tempo instantânea na linha do tempo de performance (`performance.mark()`).
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceMark {
    pub name: SmolStr,
    pub start_time_ms: f64,
}

/// Uma medição de intervalo de tempo entre duas marcas (`performance.measure()`).
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceMeasure {
    pub name: SmolStr,
    pub start_time_ms: f64,
    pub duration_ms: f64,
}

/// Entrada genérica na linha do tempo de performance.
#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceEntry {
    Mark(PerformanceMark),
    Measure(PerformanceMeasure),
}

/// Gerenciador thread-safe da Linha do Tempo de Performance de uma página ou contexto.
pub struct PerformanceTimeline {
    clock: Arc<dyn Clock>,
    marks: Mutex<Vec<PerformanceMark>>,
    measures: Mutex<Vec<PerformanceMeasure>>,
}

impl PerformanceTimeline {
    /// Inicializa a linha do tempo com o relógio monotônico padrão do sistema.
    pub fn new() -> Self {
        Self::with_clock(Arc::new(MonotonicClock::new()))
    }

    /// Inicializa a linha do tempo com um relógio customizado (ex: `MockClock` para testes determinísticos).
    pub fn with_clock(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            marks: Mutex::new(Vec::new()),
            measures: Mutex::new(Vec::new()),
        }
    }

    /// Registra uma nova marca de tempo com o nome especificado.
    pub fn mark(&self, name: impl Into<SmolStr>) -> PerformanceMark {
        let mark = PerformanceMark {
            name: name.into(),
            start_time_ms: self.clock.now_ms() as f64,
        };
        self.marks.lock().push(mark.clone());
        mark
    }

    /// Calcula e armazena a duração entre duas marcas existentes na timeline.
    pub fn measure(
        &self,
        name: impl Into<SmolStr>,
        start_mark_name: &str,
        end_mark_name: &str,
    ) -> Result<PerformanceMeasure, AceError> {
        let marks = self.marks.lock();
        let start = marks
            .iter()
            .rfind(|m| m.name == start_mark_name)
            .ok_or_else(|| {
                AceError::not_found(format!(
                    "Marca inicial '{}' não encontrada na timeline",
                    start_mark_name
                ))
            })?;

        let end = marks
            .iter()
            .rfind(|m| m.name == end_mark_name)
            .ok_or_else(|| {
                AceError::not_found(format!(
                    "Marca final '{}' não encontrada na timeline",
                    end_mark_name
                ))
            })?;

        let duration_ms = (end.start_time_ms - start.start_time_ms).max(0.0);
        let measure = PerformanceMeasure {
            name: name.into(),
            start_time_ms: start.start_time_ms,
            duration_ms,
        };

        self.measures.lock().push(measure.clone());
        Ok(measure)
    }

    /// Inicia uma medição de escopo RAII que calcula a duração automaticamente ao ser destruída (`Drop`).
    pub fn scoped_measure<'a>(&'a self, name: impl Into<SmolStr>) -> ScopedMeasure<'a> {
        let name_str = name.into();
        let start_time_ms = self.clock.now_ms() as f64;
        ScopedMeasure {
            timeline: self,
            name: name_str,
            start_time_ms,
        }
    }

    /// Retorna todas as entradas com o nome especificado.
    pub fn get_entries_by_name(&self, name: &str) -> Vec<PerformanceEntry> {
        let mut result = Vec::new();
        for mark in self.marks.lock().iter() {
            if mark.name == name {
                result.push(PerformanceEntry::Mark(mark.clone()));
            }
        }
        for measure in self.measures.lock().iter() {
            if measure.name == name {
                result.push(PerformanceEntry::Measure(measure.clone()));
            }
        }
        result
    }

    /// Limpa todas as marcas registradas.
    pub fn clear_marks(&self) {
        self.marks.lock().clear();
    }

    /// Limpa todas as medidas registradas.
    pub fn clear_measures(&self) {
        self.measures.lock().clear();
    }
}

impl Default for PerformanceTimeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Guarda RAII para medição de blocos de código com encerramento automático.
pub struct ScopedMeasure<'a> {
    timeline: &'a PerformanceTimeline,
    name: SmolStr,
    start_time_ms: f64,
}

impl<'a> Drop for ScopedMeasure<'a> {
    fn drop(&mut self) {
        let end_time_ms = self.timeline.clock.now_ms() as f64;
        let duration_ms = (end_time_ms - self.start_time_ms).max(0.0);
        let measure = PerformanceMeasure {
            name: self.name.clone(),
            start_time_ms: self.start_time_ms,
            duration_ms,
        };
        self.timeline.measures.lock().push(measure);
    }
}
