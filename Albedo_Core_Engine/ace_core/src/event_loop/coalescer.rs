//! # Coalescência e Pacing de Eventos de Entrada (W3C Pointer Events Level 3)
//!
//! Agrupamento de eventos de alta frequência (mousemove, touchmove, wheel)
//! para evitar sobrecarga do Event Loop e manter a taxa de renderização fluida a 120 FPS.

/// Evento de movimento consolidado resultante da fusão de múltiplos micro-eventos.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CoalescedMovement {
    /// Posição horizontal mais recente (CSS pixels).
    pub last_x: f32,
    /// Posição vertical mais recente (CSS pixels).
    pub last_y: f32,
    /// Deslocamento horizontal acumulado ($\Delta x$).
    pub delta_x: f32,
    /// Deslocamento vertical acumulado ($\Delta y$).
    pub delta_y: f32,
    /// Total de micro-movimentos agrupados neste lote.
    pub count: u32,
    /// Timestamp do último movimento recebido ($\mu s$).
    pub timestamp_us: u64,
}

/// Coalescedor thread-safe de eventos de entrada por dispositivo/tipo.
#[derive(Debug, Default)]
pub struct InputEventCoalescer {
    pending_movement: Option<CoalescedMovement>,
}

impl InputEventCoalescer {
    /// Cria um novo coalescedor vazio.
    pub fn new() -> Self {
        Self {
            pending_movement: None,
        }
    }

    /// Adiciona um novo micro-movimento, acumulando deltas e atualizando a posição final.
    pub fn push_movement(&mut self, x: f32, y: f32, dx: f32, dy: f32, timestamp_us: u64) {
        match &mut self.pending_movement {
            Some(m) => {
                m.last_x = x;
                m.last_y = y;
                m.delta_x += dx;
                m.delta_y += dy;
                m.count += 1;
                m.timestamp_us = timestamp_us;
            }
            None => {
                self.pending_movement = Some(CoalescedMovement {
                    last_x: x,
                    last_y: y,
                    delta_x: dx,
                    delta_y: dy,
                    count: 1,
                    timestamp_us,
                });
            }
        }
    }

    /// Drena o evento consolidado para entrega no tick de animação (`requestAnimationFrame`).
    pub fn drain(&mut self) -> Option<CoalescedMovement> {
        self.pending_movement.take()
    }

    /// Retorna `true` se houver eventos pendentes para entrega.
    #[inline]
    pub fn has_pending(&self) -> bool {
        self.pending_movement.is_some()
    }
}
