//! # Espaços de Coordenadas Tipados
//!
//! Previne confusão catastrófica entre coordenadas lógicas CSS (`px`) e pixels físicos do monitor (`DevicePixel`).

/// Unidade CSS padrão (`px`). 1 CSS pixel equivale a 1/96 de polegada na especificação W3C.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct CssPixel;

/// Pixel físico da matriz do hardware do monitor / Frame Buffer da GPU.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct DevicePixel;

/// Unidade de coordenadas intermediária usada internamente pelos algoritmos de Layout.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct LayoutPixel;

/// Coordenadas de posicionamento global na área de trabalho do Sistema Operacional.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct ScreenPixel;

/// Fator de Escala de DPI (Device Pixel Ratio / Display Scaling).
/// Ex: Telas normais = 1.0; Telas Retina / 4K com 150% = 1.5.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DpiScale(pub f32);

impl DpiScale {
    /// Fator de escala padrão 1:1 (96 DPI).
    pub const DEFAULT: Self = Self(1.0);

    /// Cria um novo fator de escala.
    #[inline]
    pub const fn new(scale: f32) -> Self {
        Self(scale)
    }

    /// Converte um comprimento em CSS pixels para pixels físicos do dispositivo.
    #[inline]
    pub fn to_device_pixels(&self, css: f32) -> f32 {
        css * self.0
    }

    /// Converte pixels físicos do dispositivo de volta para CSS pixels lógicos.
    #[inline]
    pub fn to_css_pixels(&self, device: f32) -> f32 {
        if self.0 == 0.0 {
            0.0
        } else {
            device / self.0
        }
    }
}

impl Default for DpiScale {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}
