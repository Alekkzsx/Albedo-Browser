//! # Intrinsic Sizing Architecture (`MinMaxSizes`)
//!
//! Representation and caching of intrinsic sizes (`min-content` and `max-content`)
//! with percentage dependency tracking for layout boxes.

use ace_core::math::layout_unit::LayoutUnit;
use bitflags::bitflags;
use std::ops::{Add, AddAssign};

bitflags! {
    /// Flags indicating contextual sizing dependencies for intrinsic sizes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct MinMaxDependencyFlags: u8 {
        /// The intrinsic sizing calculation depends on the container's inline percentage resolution.
        const DEPENDS_ON_PERCENTAGE_INLINE = 1 << 0;
        /// The intrinsic sizing calculation depends on the container's block percentage resolution.
        const DEPENDS_ON_PERCENTAGE_BLOCK = 1 << 1;
        /// The intrinsic sizing calculation depends on container block constraints.
        const DEPENDS_ON_BLOCK_CONSTRAINTS = 1 << 2;
    }
}

/// Par de tamanhos intrínsecos de uma caixa: `min_content` e `max_content`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MinMaxSizes {
    /// Tamanho mínimo que a caixa requer sem transbordar desnecessariamente.
    pub min_content: LayoutUnit,
    /// Tamanho que a caixa ocuparia se tivesse espaço infinito para suas linhas/itens.
    pub max_content: LayoutUnit,
    /// Sinalizador de dependência de resolução percentual em relação ao contêiner pai.
    pub depends_on_percentage: bool,
}

impl MinMaxSizes {
    /// Cria um novo par `MinMaxSizes` com valores explícitos.
    #[inline]
    pub const fn new(
        min_content: LayoutUnit,
        max_content: LayoutUnit,
        depends_on_percentage: bool,
    ) -> Self {
        Self {
            min_content,
            max_content,
            depends_on_percentage,
        }
    }

    /// Retorna `MinMaxSizes` zerado.
    #[inline]
    pub const fn zero() -> Self {
        Self {
            min_content: LayoutUnit::ZERO,
            max_content: LayoutUnit::ZERO,
            depends_on_percentage: false,
        }
    }

    /// Cria um `MinMaxSizes` a partir de um valor escalar único para min e max.
    #[inline]
    pub fn from_single(size: LayoutUnit) -> Self {
        Self {
            min_content: size,
            max_content: size,
            depends_on_percentage: false,
        }
    }

    /// Restringe o tamanho fornecido entre `min_content` e `max_content`.
    #[inline]
    pub fn clamp_size(&self, size: LayoutUnit) -> LayoutUnit {
        size.clamp(self.min_content, self.max_content)
    }

    /// Garante o invariante CSS onde `min_content <= max_content`.
    #[inline]
    pub fn enforce_invariants(&mut self) {
        if self.min_content > self.max_content {
            self.max_content = self.min_content;
        }
    }

    /// Retorna uma cópia garantindo o invariante `min_content <= max_content`.
    #[inline]
    pub fn with_invariants_enforced(mut self) -> Self {
        self.enforce_invariants();
        self
    }

    /// Acumula o valor máximo de cada dimensão entre duas instâncias.
    #[inline]
    pub fn max_assign(&mut self, other: &MinMaxSizes) {
        self.min_content = self.min_content.max(other.min_content);
        self.max_content = self.max_content.max(other.max_content);
        self.depends_on_percentage |= other.depends_on_percentage;
    }

    /// Retorna o máximo elemento a elemento entre dois `MinMaxSizes`.
    #[inline]
    pub fn max(&self, other: &MinMaxSizes) -> Self {
        Self {
            min_content: self.min_content.max(other.min_content),
            max_content: self.max_content.max(other.max_content),
            depends_on_percentage: self.depends_on_percentage || other.depends_on_percentage,
        }
    }

    /// Define a dependência percentual e retorna `Self`.
    #[inline]
    pub fn with_percentage_dependency(mut self, depends: bool) -> Self {
        self.depends_on_percentage = depends;
        self
    }
}

// ---------------------------------------------------------------------------
// Operadores Aritméticos
// ---------------------------------------------------------------------------

impl Add<LayoutUnit> for MinMaxSizes {
    type Output = Self;

    #[inline]
    fn add(self, rhs: LayoutUnit) -> Self {
        Self {
            min_content: self.min_content.saturating_add(rhs),
            max_content: self.max_content.saturating_add(rhs),
            depends_on_percentage: self.depends_on_percentage,
        }
    }
}

impl AddAssign<LayoutUnit> for MinMaxSizes {
    #[inline]
    fn add_assign(&mut self, rhs: LayoutUnit) {
        self.min_content = self.min_content.saturating_add(rhs);
        self.max_content = self.max_content.saturating_add(rhs);
    }
}

impl Add<MinMaxSizes> for MinMaxSizes {
    type Output = Self;

    #[inline]
    fn add(self, rhs: MinMaxSizes) -> Self {
        Self {
            min_content: self.min_content.saturating_add(rhs.min_content),
            max_content: self.max_content.saturating_add(rhs.max_content),
            depends_on_percentage: self.depends_on_percentage || rhs.depends_on_percentage,
        }
    }
}

impl AddAssign<MinMaxSizes> for MinMaxSizes {
    #[inline]
    fn add_assign(&mut self, rhs: MinMaxSizes) {
        self.min_content = self.min_content.saturating_add(rhs.min_content);
        self.max_content = self.max_content.saturating_add(rhs.max_content);
        self.depends_on_percentage = self.depends_on_percentage || rhs.depends_on_percentage;
    }
}

/// Cache de tamanhos intrínsecos armazenado na caixa de layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MinMaxSizesCache {
    /// Os tamanhos intrínsecos computados.
    pub sizes: MinMaxSizes,
    /// Bit que indica se o cálculo depende do block-size percentual do contêiner pai.
    pub depends_on_block_constraints: bool,
    /// Bit que indica se descendentes usam porcentagens no eixo inline.
    pub depends_on_percentage_inline: bool,
}

impl MinMaxSizesCache {
    /// Cria uma nova entrada de cache para o par de tamanhos fornecido.
    #[inline]
    pub fn new(sizes: MinMaxSizes) -> Self {
        Self {
            sizes,
            depends_on_block_constraints: false,
            depends_on_percentage_inline: sizes.depends_on_percentage,
        }
    }

    /// Cria uma nova entrada de cache com flags detalhadas.
    #[inline]
    pub fn new_with_flags(
        sizes: MinMaxSizes,
        depends_on_block_constraints: bool,
        depends_on_percentage_inline: bool,
    ) -> Self {
        Self {
            sizes,
            depends_on_block_constraints,
            depends_on_percentage_inline,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_max_sizes_basic_and_clamping() {
        let m = MinMaxSizes::new(LayoutUnit::from_px(100), LayoutUnit::from_px(300), false);
        assert_eq!(m.min_content.to_f32_px(), 100.0);
        assert_eq!(m.max_content.to_f32_px(), 300.0);
        assert!(!m.depends_on_percentage);

        // Clamp tests
        assert_eq!(
            m.clamp_size(LayoutUnit::from_px(50)),
            LayoutUnit::from_px(100)
        );
        assert_eq!(
            m.clamp_size(LayoutUnit::from_px(200)),
            LayoutUnit::from_px(200)
        );
        assert_eq!(
            m.clamp_size(LayoutUnit::from_px(400)),
            LayoutUnit::from_px(300)
        );
    }

    #[test]
    fn test_min_max_sizes_arithmetic_and_accumulation() {
        let mut m1 = MinMaxSizes::new(LayoutUnit::from_px(50), LayoutUnit::from_px(150), false);
        m1 += LayoutUnit::from_px(20);
        assert_eq!(m1.min_content, LayoutUnit::from_px(70));
        assert_eq!(m1.max_content, LayoutUnit::from_px(170));

        let m2 = MinMaxSizes::new(LayoutUnit::from_px(30), LayoutUnit::from_px(80), true);
        let sum = m1 + m2;
        assert_eq!(sum.min_content, LayoutUnit::from_px(100));
        assert_eq!(sum.max_content, LayoutUnit::from_px(250));
        assert!(sum.depends_on_percentage);

        let mut max_accum = m1;
        max_accum.max_assign(&m2);
        assert_eq!(max_accum.min_content, LayoutUnit::from_px(70));
        assert_eq!(max_accum.max_content, LayoutUnit::from_px(170));
        assert!(max_accum.depends_on_percentage);
    }

    #[test]
    fn test_min_max_invariants() {
        let mut inverted =
            MinMaxSizes::new(LayoutUnit::from_px(200), LayoutUnit::from_px(100), false);
        inverted.enforce_invariants();
        assert_eq!(inverted.min_content, LayoutUnit::from_px(200));
        assert_eq!(inverted.max_content, LayoutUnit::from_px(200));
    }

    #[test]
    fn test_min_max_zero_and_from_single() {
        let zero = MinMaxSizes::zero();
        assert_eq!(zero.min_content, LayoutUnit::ZERO);
        assert_eq!(zero.max_content, LayoutUnit::ZERO);
        assert!(!zero.depends_on_percentage);

        let single = MinMaxSizes::from_single(LayoutUnit::from_px(120));
        assert_eq!(single.min_content, LayoutUnit::from_px(120));
        assert_eq!(single.max_content, LayoutUnit::from_px(120));
        assert!(!single.depends_on_percentage);

        let with_dep = single.with_percentage_dependency(true);
        assert!(with_dep.depends_on_percentage);
    }

    #[test]
    fn test_min_max_binary_max() {
        let m1 = MinMaxSizes::new(LayoutUnit::from_px(50), LayoutUnit::from_px(120), false);
        let m2 = MinMaxSizes::new(LayoutUnit::from_px(80), LayoutUnit::from_px(100), true);
        let m3 = m1.max(&m2);
        assert_eq!(m3.min_content, LayoutUnit::from_px(80));
        assert_eq!(m3.max_content, LayoutUnit::from_px(120));
        assert!(m3.depends_on_percentage);
    }

    #[test]
    fn test_min_max_cache_and_flags() {
        let sizes = MinMaxSizes::new(LayoutUnit::from_px(60), LayoutUnit::from_px(180), true);
        let cache = MinMaxSizesCache::new(sizes);
        assert_eq!(cache.sizes, sizes);
        assert!(!cache.depends_on_block_constraints);
        assert!(cache.depends_on_percentage_inline);

        let cache_explicit = MinMaxSizesCache::new_with_flags(sizes, true, false);
        assert!(cache_explicit.depends_on_block_constraints);
        assert!(!cache_explicit.depends_on_percentage_inline);

        let mut flags = MinMaxDependencyFlags::DEPENDS_ON_PERCENTAGE_INLINE;
        flags |= MinMaxDependencyFlags::DEPENDS_ON_BLOCK_CONSTRAINTS;
        assert!(flags.contains(MinMaxDependencyFlags::DEPENDS_ON_PERCENTAGE_INLINE));
        assert!(flags.contains(MinMaxDependencyFlags::DEPENDS_ON_BLOCK_CONSTRAINTS));
        assert!(!flags.contains(MinMaxDependencyFlags::DEPENDS_ON_PERCENTAGE_BLOCK));
    }
}
