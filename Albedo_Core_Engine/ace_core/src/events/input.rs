//! # Primitivas de Hardware de Entrada (W3C Pointer Events & UI Events)
//!
//! Tipos de baixo nível e máscaras de bits fortemente tipadas para mouse, toque,
//! caneta digital e modificadores de teclado.

/// Dispositivo apontador que gerou o evento (W3C Pointer Events Level 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PointerType {
    /// Mouse tradicional ou trackpad.
    #[default]
    Mouse,
    /// Caneta digital / Stylus.
    Pen,
    /// Toque direto em touchscreen.
    Touch,
}

/// Identificador de botão único do mouse / ponteiro (W3C PointerEvent.button).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PointerButton {
    /// Botão principal (geralmente botão esquerdo do mouse).
    #[default]
    Primary,
    /// Botão secundário (geralmente botão direito do mouse).
    Secondary,
    /// Botão auxiliar (geralmente botão do meio / clique da roda).
    Auxiliary,
    /// Botão de voltar (navegação do mouse).
    Back,
    /// Botão de avançar (navegação do mouse).
    Forward,
    /// Ponta de borracha de uma caneta digital.
    Eraser,
}

impl PointerButton {
    /// Retorna o índice numérico padronizado pelo W3C (0 = Primary, 1 = Auxiliary, 2 = Secondary, etc.).
    pub const fn to_w3c_index(self) -> i16 {
        match self {
            Self::Primary => 0,
            Self::Auxiliary => 1,
            Self::Secondary => 2,
            Self::Back => 3,
            Self::Forward => 4,
            Self::Eraser => 5,
        }
    }
}

/// Máscara de bits de botões pressionados simultaneamente (W3C PointerEvent.buttons).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PointerButtons(pub u16);

impl PointerButtons {
    pub const NONE: Self = Self(0);
    pub const PRIMARY: Self = Self(1 << 0);
    pub const SECONDARY: Self = Self(1 << 1);
    pub const AUXILIARY: Self = Self(1 << 2);
    pub const BACK: Self = Self(1 << 3);
    pub const FORWARD: Self = Self(1 << 4);
    pub const ERASER: Self = Self(1 << 5);

    #[inline]
    pub const fn has_primary(self) -> bool {
        (self.0 & Self::PRIMARY.0) != 0
    }

    #[inline]
    pub const fn has_secondary(self) -> bool {
        (self.0 & Self::SECONDARY.0) != 0
    }

    #[inline]
    pub const fn has_auxiliary(self) -> bool {
        (self.0 & Self::AUXILIARY.0) != 0
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// Estado das teclas modificadoras do teclado no instante do evento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ModifiersState {
    /// Tecla Alt / Option pressionada.
    pub alt: bool,
    /// Tecla Control pressionada.
    pub ctrl: bool,
    /// Tecla Shift pressionada.
    pub shift: bool,
    /// Tecla Meta (Command no macOS, Windows no PC).
    pub meta: bool,
}

impl ModifiersState {
    /// Retorna `true` se nenhuma tecla modificadora estiver ativa.
    #[inline]
    pub const fn is_empty(self) -> bool {
        !self.alt && !self.ctrl && !self.shift && !self.meta
    }
}

/// Localização física da tecla no teclado (W3C KeyboardEvent.location).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum KeyLocation {
    /// Posição padrão / única no teclado.
    #[default]
    Standard,
    /// Lado esquerdo do teclado (ex: Shift esquerdo, Ctrl esquerdo).
    Left,
    /// Lado direito do teclado (ex: Shift direito, AltGr).
    Right,
    /// Teclado numérico dedicado (Numpad).
    Numpad,
}
