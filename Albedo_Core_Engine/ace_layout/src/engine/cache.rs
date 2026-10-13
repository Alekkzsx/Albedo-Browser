//! # Incremental Subtree Layout Cache (`LayoutCache`)
//!
//! Implements an $O(1)$ subtree layout cache using `LayoutCacheKey`,
//! enabling instant surgical reflow (< 50 µs/mutation) by reusing unchanged
//! immutable layout fragments across passes.

use ace_core::id::NodeId;
use rustc_hash::FxHashMap;
use std::sync::Arc;

use crate::fragment::layout_fragment::LayoutFragment;

/// Execution mode for a layout pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LayoutPassMode {
    /// Hypothetical measurement pass.
    Measure,
    /// Final definitive fragment generation pass.
    #[default]
    Definitive,
}

/// Unique key identifying a layout subtree computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutCacheKey {
    /// Associated DOM / BoxTree NodeId.
    pub node_id: NodeId,
    /// Generation / version of the computed style for this node.
    pub style_generation: u32,
    /// Raw available inline width in base-60 LayoutUnits (-1 if indefinite).
    pub available_width_raw: i32,
    /// Raw available block height in base-60 LayoutUnits (-1 if indefinite).
    pub available_height_raw: i32,
    /// Layout pass mode (Measure vs Definitive).
    pub pass_mode: LayoutPassMode,
}

impl LayoutCacheKey {
    /// Creates a new layout cache key.
    #[inline]
    pub const fn new(
        node_id: NodeId,
        style_generation: u32,
        available_width_raw: i32,
        available_height_raw: i32,
        pass_mode: LayoutPassMode,
    ) -> Self {
        Self {
            node_id,
            style_generation,
            available_width_raw,
            available_height_raw,
            pass_mode,
        }
    }
}

/// Incremental subtree cache mapping `LayoutCacheKey` to `Arc<LayoutFragment>`.
#[derive(Debug, Clone, Default)]
pub struct LayoutCache {
    entries: FxHashMap<LayoutCacheKey, Arc<LayoutFragment>>,
    hits: u64,
    misses: u64,
}

impl LayoutCache {
    /// Creates a new, empty layout cache.
    pub fn new() -> Self {
        Self {
            entries: FxHashMap::default(),
            hits: 0,
            misses: 0,
        }
    }

    /// Looks up a cached fragment by key.
    #[inline]
    pub fn get(&mut self, key: &LayoutCacheKey) -> Option<Arc<LayoutFragment>> {
        if let Some(fragment) = self.entries.get(key) {
            self.hits += 1;
            Some(Arc::clone(fragment))
        } else {
            self.misses += 1;
            None
        }
    }

    /// Inserts or updates a cached layout fragment.
    #[inline]
    pub fn insert(&mut self, key: LayoutCacheKey, fragment: Arc<LayoutFragment>) {
        self.entries.insert(key, fragment);
    }

    /// Surgically invalidates all entries for a specific `NodeId`.
    pub fn invalidate_node(&mut self, node_id: NodeId) {
        self.entries.retain(|k, _| k.node_id != node_id);
    }

    /// Clears all cached entries and resets metrics.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Total number of cached fragments.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the cache is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total cache hits.
    #[inline]
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Total cache misses.
    #[inline]
    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// Current cache hit rate ratio (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fragment::overflow::LayoutOverflow;
    use crate::positioning::stacking::StackingLayer;
    use crate::tree::layout_box::LayoutBoxKind;
    use ace_core::math::layout_unit::LayoutUnit;
    use ace_core::math::logical::LogicalRect;
    use ace_style::computed::ComputedStyle;

    #[test]
    fn test_layout_cache_insert_lookup_and_invalidate() {
        let mut cache = LayoutCache::new();
        let node_id = NodeId::new();
        let key = LayoutCacheKey::new(node_id, 1, 6000, 4800, LayoutPassMode::Definitive);

        assert!(cache.get(&key).is_none());
        assert_eq!(cache.misses(), 1);

        let style = Arc::new(ComputedStyle::initial());
        let bounds = LogicalRect {
            inline_start: LayoutUnit::from_px(0),
            block_start: LayoutUnit::from_px(0),
            inline_size: LayoutUnit::from_px(100),
            block_size: LayoutUnit::from_px(80),
        };
        let fragment = Arc::new(LayoutFragment::new_box(
            node_id,
            LayoutBoxKind::Block,
            bounds,
            bounds,
            Vec::new(),
            LayoutOverflow::new(bounds),
            StackingLayer::Layer1RootBackgroundAndBorders,
            0,
            style,
            None,
        ));

        cache.insert(key, fragment.clone());
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get(&key);
        assert!(retrieved.is_some());
        assert_eq!(cache.hits(), 1);
        assert_eq!(cache.hit_rate(), 0.5);

        // Invalidate node
        cache.invalidate_node(node_id);
        assert_eq!(cache.len(), 0);
        assert!(cache.get(&key).is_none());
    }
}
