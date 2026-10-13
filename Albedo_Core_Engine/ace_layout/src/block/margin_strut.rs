//! # Acumulador Linear de Margens Verticais (`MarginStrut`)
//!
//! Implementa o acumulador linear $O(1)$ para colapso de margens verticais conforme
//! a especificação W3C CSS 2.2 §8.3.1 e a arquitetura Chromium Blink LayoutNG (`NGMarginStrut`).
//!
//! ## Regras Normativas de Colapso (CSS 2.2 §8.3.1)
//! 1. **Margens puramente positivas:** A margem resultante é a maior das margens positivas:
//!    $$\text{Margin} = \max(m_1, m_2, \dots, m_k)$$
//! 2. **Margens puramente negativas:** A margem resultante é a de maior magnitude absoluta
//!    (o menor valor negativo algébrico):
//!    $$\text{Margin} = \min(m_1, m_2, \dots, m_k)$$
//! 3. **Margens mistas:** A margem resultante é a soma da maior positiva com a mais negativa:
//!    $$\text{Margin} = \max(m_{\text{pos}}) + \min(m_{\text{neg}})$$
//! 4. Se nenhuma margem foi acumulada, a margem resultante é $0$.

use ace_core::math::layout_unit::LayoutUnit;

/// Acumulador linear $O(1)$ de margens verticais em colapso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarginStrut {
    /// A maior margem positiva encontrada até o momento ($\ge 0$).
    pub positive_margin: LayoutUnit,
    /// A margem negativa de maior magnitude (armazenada como valor negativo ou zero, $\le 0$).
    pub negative_margin: LayoutUnit,
    /// Sinalizador de margem em modo Quirks (CSS Quirks Mode).
    pub has_quirks: bool,
}

impl MarginStrut {
    /// Cria um novo `MarginStrut` vazio com margens zeradas.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            positive_margin: LayoutUnit::ZERO,
            negative_margin: LayoutUnit::ZERO,
            has_quirks: false,
        }
    }

    /// Cria um `MarginStrut` inicializado com uma margem inicial.
    #[inline]
    pub fn from_margin(margin: LayoutUnit) -> Self {
        let mut strut = Self::empty();
        strut.append(margin);
        strut
    }

    /// Acumula uma nova margem na estrutura em tempo $O(1)$.
    ///
    /// Se a margem for positiva ($\ge 0$), atualiza `positive_margin` para $\max(\text{atual}, \text{margin})$.
    /// Se a margem for negativa ($< 0$), atualiza `negative_margin` para $\min(\text{atual}, \text{margin})$.
    #[inline]
    pub fn append(&mut self, margin: LayoutUnit) {
        if margin.raw() >= 0 {
            if margin > self.positive_margin {
                self.positive_margin = margin;
            }
        } else if margin < self.negative_margin {
            self.negative_margin = margin;
        }
    }

    /// Acumula outro `MarginStrut` inteiro neste acumulador em tempo $O(1)$.
    ///
    /// Usado para colapso de margens entre pai e primeiro/último filho ou
    /// blocos vazios sem backtracking recursivo.
    #[inline]
    pub fn append_strut(&mut self, other: &MarginStrut) {
        if other.positive_margin > self.positive_margin {
            self.positive_margin = other.positive_margin;
        }
        if other.negative_margin < self.negative_margin {
            self.negative_margin = other.negative_margin;
        }
        self.has_quirks |= other.has_quirks;
    }

    /// Resolve a margem efetiva resultante combinando a maior positiva com a mais negativa.
    ///
    /// Corresponde exatamente à fórmula normativa:
    /// $$\text{solve}() = \max(m_{\text{pos}}) + \min(m_{\text{neg}})$$
    #[inline]
    pub fn solve(&self) -> LayoutUnit {
        self.positive_margin.saturating_add(self.negative_margin)
    }

    /// Alias para `solve()`.
    #[inline]
    pub fn resolve(&self) -> LayoutUnit {
        self.solve()
    }

    /// Retorna `true` se nenhuma margem foi acumulada (ambas são zero).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.positive_margin == LayoutUnit::ZERO && self.negative_margin == LayoutUnit::ZERO
    }

    /// Limpa o acumulador para novo ciclo entre irmãos ou blocos.
    #[inline]
    pub fn reset(&mut self) {
        self.positive_margin = LayoutUnit::ZERO;
        self.negative_margin = LayoutUnit::ZERO;
        self.has_quirks = false;
    }

    /// Retorna a maior margem positiva acumulada.
    #[inline]
    pub fn positive_margin(&self) -> LayoutUnit {
        self.positive_margin
    }

    /// Retorna a margem negativa mais extrema acumulada.
    #[inline]
    pub fn negative_margin(&self) -> LayoutUnit {
        self.negative_margin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_margin_strut_empty() {
        let strut = MarginStrut::empty();
        assert!(strut.is_empty());
        assert_eq!(strut.solve(), LayoutUnit::ZERO);
        assert_eq!(strut.resolve(), LayoutUnit::ZERO);
    }

    #[test]
    fn test_margin_strut_positive_collapse() {
        let mut strut = MarginStrut::empty();
        strut.append(LayoutUnit::from_px(10));
        strut.append(LayoutUnit::from_px(30));
        strut.append(LayoutUnit::from_px(20));

        assert!(!strut.is_empty());
        assert_eq!(strut.positive_margin(), LayoutUnit::from_px(30));
        assert_eq!(strut.negative_margin(), LayoutUnit::ZERO);
        assert_eq!(strut.solve(), LayoutUnit::from_px(30));
    }

    #[test]
    fn test_margin_strut_negative_collapse() {
        let mut strut = MarginStrut::empty();
        strut.append(LayoutUnit::from_px(-10));
        strut.append(LayoutUnit::from_px(-25));
        strut.append(LayoutUnit::from_px(-5));

        assert_eq!(strut.positive_margin(), LayoutUnit::ZERO);
        assert_eq!(strut.negative_margin(), LayoutUnit::from_px(-25));
        assert_eq!(strut.solve(), LayoutUnit::from_px(-25));
    }

    #[test]
    fn test_margin_strut_mixed_collapse() {
        let mut strut = MarginStrut::empty();
        strut.append(LayoutUnit::from_px(40));
        strut.append(LayoutUnit::from_px(-15));
        strut.append(LayoutUnit::from_px(20));
        strut.append(LayoutUnit::from_px(-5));

        // Max positive = 40, Min negative = -15 -> 40 + (-15) = 25px
        assert_eq!(strut.positive_margin(), LayoutUnit::from_px(40));
        assert_eq!(strut.negative_margin(), LayoutUnit::from_px(-15));
        assert_eq!(strut.solve(), LayoutUnit::from_px(25));
    }

    #[test]
    fn test_margin_strut_append_strut() {
        let mut s1 = MarginStrut::empty();
        s1.append(LayoutUnit::from_px(20));
        s1.append(LayoutUnit::from_px(-10));

        let mut s2 = MarginStrut::empty();
        s2.append(LayoutUnit::from_px(35));
        s2.append(LayoutUnit::from_px(-30));

        s1.append_strut(&s2);
        assert_eq!(s1.positive_margin(), LayoutUnit::from_px(35));
        assert_eq!(s1.negative_margin(), LayoutUnit::from_px(-30));
        assert_eq!(s1.solve(), LayoutUnit::from_px(5));
    }

    #[test]
    fn test_margin_strut_reset() {
        let mut strut = MarginStrut::empty();
        strut.append(LayoutUnit::from_px(100));
        strut.append(LayoutUnit::from_px(-50));
        assert!(!strut.is_empty());

        strut.reset();
        assert!(strut.is_empty());
        assert_eq!(strut.solve(), LayoutUnit::ZERO);
    }

    #[test]
    fn test_margin_strut_deep_sibling_chain() {
        let mut strut = MarginStrut::empty();
        // 10,000 margins: all +20px and -10px
        for _ in 0..5_000 {
            strut.append(LayoutUnit::from_px(20));
            strut.append(LayoutUnit::from_px(-10));
        }
        assert_eq!(strut.solve(), LayoutUnit::from_px(10));
    }
}
