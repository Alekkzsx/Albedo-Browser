//! # Utilitários de Alocação e Gestão de Memória
//!
//! Funções auxiliares para cálculo de alinhamento, estimativas de crescimento e formatação de telemetria.

/// Arredonda um endereço ou tamanho para o próximo múltiplo da potência de 2 especificada em `align`.
#[inline]
pub const fn align_up(val: usize, align: usize) -> usize {
    debug_assert!(
        align.is_power_of_two(),
        "O alinhamento deve ser uma potência de 2"
    );
    (val + align - 1) & !(align - 1)
}

/// Verifica se um endereço ou tamanho está alinhado para o alinhamento especificado.
#[inline]
pub const fn is_aligned(val: usize, align: usize) -> bool {
    (val & (align - 1)) == 0
}

/// Calcula a nova capacidade ao expandir um buffer ou arena, usando crescimento exponencial 1.5x/2x com limite de segurança.
#[inline]
pub fn calc_growth_capacity(current: usize, needed: usize) -> usize {
    let growth = current + (current / 2).max(8);
    growth.max(needed)
}

/// Formata uma quantidade de bytes em formato legível para humanos (B, KB, MB, GB).
pub fn format_bytes(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}
