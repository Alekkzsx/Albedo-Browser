//! # Test Harness & Mock Measurer Utilities (`tests/common/mod.rs`)
//!
//! Provides reusable, deterministic opaque-box test infrastructure for Phase 7 (`ace_layout`):
//! - **`MockFontMeasurer`**: Character-cell typography engine with fixed glyph advance in base-60 `LayoutUnit`.
//! - **Synthetic Tree Builders**: Fluent builders for `ComputedStyle`, `RenderTreeNode`, and `RenderTree`.
//! - **DOM Integration**: Helpers for compiling HTML directly into `(Document, RenderTree)`.
//! - **Subpixel Snapping Verifiers**: Exact continuous edge verification guaranteeing zero 1px gaps or overlaps.
//! - **Telemetry & Benchmarks**: High-resolution latency and throughput profiler for performance SLAs.

#![allow(dead_code)]

use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ace_core::id::NodeId;
use ace_core::math::layout_unit::{snap_box, LayoutUnit};
use ace_dom::Document;
use ace_style::computed::structs::StyleFont;
use ace_style::computed::style::ComputedStyle;
use ace_style::engine::style_engine::{RenderNodeKind, RenderTree, RenderTreeNode, StyleEngine};
use ace_style::values::enums::{
    AlignItems, BoxSizing, Direction, Display, FlexDirection, FlexWrap, GridAutoFlow,
    JustifyContent, Overflow, OverflowWrap, Position, RectSides, TextAlign, WhiteSpace, WordBreak,
    WritingMode, ZIndex,
};

// =============================================================================
// 1. Character-Cell Typography & Mock Deterministic Font Measurer
// =============================================================================

/// Canonical vertical font metrics in fixed-point `LayoutUnit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontMetrics {
    /// Distance from baseline to the top of the glyph box.
    pub ascent: LayoutUnit,
    /// Distance from baseline to the bottom of the glyph box.
    pub descent: LayoutUnit,
    /// Recommended interline gap.
    pub line_gap: LayoutUnit,
}

impl FontMetrics {
    /// Creates a new font metrics descriptor.
    #[inline]
    pub const fn new(ascent: LayoutUnit, descent: LayoutUnit, line_gap: LayoutUnit) -> Self {
        Self {
            ascent,
            descent,
            line_gap,
        }
    }

    /// Total line height derived from vertical metrics: `ascent + descent + line_gap`.
    #[inline]
    pub fn line_height(&self) -> LayoutUnit {
        self.ascent
            .saturating_add(self.descent)
            .saturating_add(self.line_gap)
    }
}

/// Abstract contract for font measurement used across tests.
pub trait TestTextMeasurer: Send + Sync {
    /// Measures the spatial dimensions (width, height) of a text run under a given font style.
    fn measure_text(&self, text: &str, font: &StyleFont) -> (LayoutUnit, LayoutUnit);

    /// Returns the fundamental vertical metrics for the font.
    fn font_metrics(&self, font: &StyleFont) -> FontMetrics;
}

/// Deterministic character-cell typography measurer.
///
/// In character-cell typography (fixed-pitch monospaced rendering):
/// - Every character advances by an exact fixed scalar in base-60 `LayoutUnit` (default: 8px = 480 raw units).
/// - Default baseline metrics: `ascent = 12px`, `descent = 4px`, `line_gap = 2px` (line height = 18px).
/// - Enables 100% deterministic layout testing without GPU or OS font rasterizer dependencies.
#[derive(Debug, Clone)]
pub struct MockFontMeasurer {
    pub glyph_advance: LayoutUnit,
    pub ascent: LayoutUnit,
    pub descent: LayoutUnit,
    pub line_gap: LayoutUnit,
}

impl Default for MockFontMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockFontMeasurer {
    /// Creates a default monospaced measurer: 8px advance, 12px ascent, 4px descent, 2px line gap.
    pub fn new() -> Self {
        Self {
            glyph_advance: LayoutUnit::from_px(8),
            ascent: LayoutUnit::from_px(12),
            descent: LayoutUnit::from_px(4),
            line_gap: LayoutUnit::from_px(2),
        }
    }

    /// Creates a measurer with a customized glyph advance.
    pub fn with_glyph_advance(glyph_advance: LayoutUnit) -> Self {
        Self {
            glyph_advance,
            ..Self::new()
        }
    }

    /// Creates a measurer with fully specified metrics.
    pub fn with_metrics(
        glyph_advance: LayoutUnit,
        ascent: LayoutUnit,
        descent: LayoutUnit,
        line_gap: LayoutUnit,
    ) -> Self {
        Self {
            glyph_advance,
            ascent,
            descent,
            line_gap,
        }
    }

    /// Measures the exact inline advance and line height of the text string.
    pub fn measure_text(&self, text: &str, font: &StyleFont) -> (LayoutUnit, LayoutUnit) {
        let count = text.chars().count() as i32;
        let width = LayoutUnit::from_raw(self.glyph_advance.raw().saturating_mul(count));
        let height = font
            .line_height
            .unwrap_or_else(|| self.font_metrics(font).line_height());
        (width, height)
    }

    /// Returns the vertical font metrics for the given font style.
    pub fn font_metrics(&self, _font: &StyleFont) -> FontMetrics {
        FontMetrics::new(self.ascent, self.descent, self.line_gap)
    }

    /// Returns the single glyph advance in `LayoutUnit`.
    #[inline]
    pub fn glyph_advance(&self) -> LayoutUnit {
        self.glyph_advance
    }
}

impl TestTextMeasurer for MockFontMeasurer {
    #[inline]
    fn measure_text(&self, text: &str, font: &StyleFont) -> (LayoutUnit, LayoutUnit) {
        self.measure_text(text, font)
    }

    #[inline]
    fn font_metrics(&self, font: &StyleFont) -> FontMetrics {
        self.font_metrics(font)
    }
}

// =============================================================================
// 2. Synthetic Tree Builders (Opaque-Box Construction)
// =============================================================================

/// Fluent builder for constructing `Arc<ComputedStyle>` for layout tests.
#[derive(Debug, Clone)]
pub struct StyleBuilder {
    style: ComputedStyle,
}

impl Default for StyleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleBuilder {
    /// Initializes with CSS initial default styles.
    pub fn new() -> Self {
        Self {
            style: ComputedStyle::initial(),
        }
    }

    /// Sets `display`.
    pub fn display(mut self, display: Display) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.display = display;
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets `position`.
    pub fn position(mut self, position: Position) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.position = position;
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets `box_sizing`.
    pub fn box_sizing(mut self, box_sizing: BoxSizing) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.box_sizing = box_sizing;
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets explicit `width` in pixels.
    pub fn width_px(mut self, px: i32) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.width = Some(LayoutUnit::from_px(px));
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets explicit `height` in pixels.
    pub fn height_px(mut self, px: i32) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.height = Some(LayoutUnit::from_px(px));
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets `min_width` in pixels.
    pub fn min_width_px(mut self, px: i32) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.min_width = Some(LayoutUnit::from_px(px));
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets `max_width` in pixels.
    pub fn max_width_px(mut self, px: i32) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.max_width = Some(LayoutUnit::from_px(px));
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets `min_height` in pixels.
    pub fn min_height_px(mut self, px: i32) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.min_height = Some(LayoutUnit::from_px(px));
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets `max_height` in pixels.
    pub fn max_height_px(mut self, px: i32) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.max_height = Some(LayoutUnit::from_px(px));
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets out-of-flow insets `(top, right, bottom, left)` in pixels.
    pub fn insets_px(
        mut self,
        top: Option<i32>,
        right: Option<i32>,
        bottom: Option<i32>,
        left: Option<i32>,
    ) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.top = top.map(LayoutUnit::from_px);
        b.right = right.map(LayoutUnit::from_px);
        b.bottom = bottom.map(LayoutUnit::from_px);
        b.left = left.map(LayoutUnit::from_px);
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets `overflow_x` and `overflow_y`.
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.overflow_x = overflow;
        b.overflow_y = overflow;
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets `z_index`.
    pub fn z_index(mut self, z: i32) -> Self {
        let mut b = (*self.style.box_style).clone();
        b.z_index = ZIndex::Integer(z);
        self.style.box_style = Arc::new(b);
        self
    }

    /// Sets surrounding margins `(top, right, bottom, left)` in pixels.
    pub fn margin_px(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        let mut s = (*self.style.surrounding).clone();
        s.margin = RectSides {
            top: Some(LayoutUnit::from_px(top)),
            right: Some(LayoutUnit::from_px(right)),
            bottom: Some(LayoutUnit::from_px(bottom)),
            left: Some(LayoutUnit::from_px(left)),
        };
        self.style.surrounding = Arc::new(s);
        self
    }

    /// Sets horizontal margins to auto (`margin: 0 auto`).
    pub fn margin_auto_horizontal(mut self) -> Self {
        let mut s = (*self.style.surrounding).clone();
        s.margin = RectSides {
            top: Some(LayoutUnit::ZERO),
            right: None,
            bottom: Some(LayoutUnit::ZERO),
            left: None,
        };
        self.style.surrounding = Arc::new(s);
        self
    }

    /// Sets top and bottom margins in pixels.
    pub fn vertical_margin_px(mut self, top: i32, bottom: i32) -> Self {
        let mut s = (*self.style.surrounding).clone();
        s.margin.top = Some(LayoutUnit::from_px(top));
        s.margin.bottom = Some(LayoutUnit::from_px(bottom));
        self.style.surrounding = Arc::new(s);
        self
    }

    /// Sets surrounding padding `(top, right, bottom, left)` in pixels.
    pub fn padding_px(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        let mut s = (*self.style.surrounding).clone();
        s.padding = RectSides {
            top: LayoutUnit::from_px(top),
            right: LayoutUnit::from_px(right),
            bottom: LayoutUnit::from_px(bottom),
            left: LayoutUnit::from_px(left),
        };
        self.style.surrounding = Arc::new(s);
        self
    }

    /// Sets surrounding borders `(top, right, bottom, left)` in pixels.
    pub fn border_px(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        let mut s = (*self.style.surrounding).clone();
        s.border_width = RectSides {
            top: LayoutUnit::from_px(top),
            right: LayoutUnit::from_px(right),
            bottom: LayoutUnit::from_px(bottom),
            left: LayoutUnit::from_px(left),
        };
        self.style.surrounding = Arc::new(s);
        self
    }

    /// Sets flex container properties.
    pub fn flex_container(
        mut self,
        direction: FlexDirection,
        wrap: FlexWrap,
        justify: JustifyContent,
        align_items: AlignItems,
    ) -> Self {
        self = self.display(Display::Flex);
        let mut f = (*self.style.flex).clone();
        f.flex_direction = direction;
        f.flex_wrap = wrap;
        f.justify_content = justify;
        f.align_items = align_items;
        self.style.flex = Arc::new(f);
        self
    }

    /// Sets flex item parameters: `flex_grow`, `flex_shrink`, `flex_basis`.
    pub fn flex_item(mut self, grow: f32, shrink: f32, basis_px: Option<i32>) -> Self {
        let mut f = (*self.style.flex).clone();
        f.flex_grow = grow;
        f.flex_shrink = shrink;
        f.flex_basis = basis_px.map(LayoutUnit::from_px);
        self.style.flex = Arc::new(f);
        self
    }

    /// Sets flex gaps in pixels.
    pub fn flex_gap_px(mut self, row_gap: i32, col_gap: i32) -> Self {
        let mut f = (*self.style.flex).clone();
        f.row_gap = Some(LayoutUnit::from_px(row_gap));
        f.column_gap = Some(LayoutUnit::from_px(col_gap));
        self.style.flex = Arc::new(f);
        self
    }

    /// Sets CSS Grid auto-flow.
    pub fn grid_container(mut self, auto_flow: GridAutoFlow) -> Self {
        self = self.display(Display::Grid);
        let mut g = (*self.style.grid).clone();
        g.grid_auto_flow = auto_flow;
        self.style.grid = Arc::new(g);
        self
    }

    /// Sets font size and line height in pixels.
    pub fn font_size_px(mut self, size_px: i32, line_height_px: Option<i32>) -> Self {
        let mut f = (*self.style.font).clone();
        f.size = LayoutUnit::from_px(size_px);
        f.line_height = line_height_px.map(LayoutUnit::from_px);
        self.style.font = Arc::new(f);
        self
    }

    /// Sets text layout parameters: `text_align`, `white_space`, `word_break`, `overflow_wrap`.
    pub fn text_props(
        mut self,
        text_align: TextAlign,
        white_space: WhiteSpace,
        word_break: WordBreak,
        overflow_wrap: OverflowWrap,
    ) -> Self {
        let mut t = (*self.style.text).clone();
        t.text_align = text_align;
        t.white_space = white_space;
        t.word_break = word_break;
        t.overflow_wrap = overflow_wrap;
        self.style.text = Arc::new(t);
        self
    }

    /// Sets reading direction: `Ltr` or `Rtl`.
    pub fn direction(mut self, direction: Direction) -> Self {
        let mut t = (*self.style.text).clone();
        t.direction = direction;
        self.style.text = Arc::new(t);
        self
    }

    /// Sets writing mode: `HorizontalTb`, `VerticalRl`, `VerticalLr`.
    pub fn writing_mode(mut self, writing_mode: WritingMode) -> Self {
        let mut t = (*self.style.text).clone();
        t.writing_mode = writing_mode;
        self.style.text = Arc::new(t);
        self
    }

    /// Sets element opacity.
    pub fn opacity(mut self, opacity: f32) -> Self {
        let mut c = (*self.style.color).clone();
        c.opacity = opacity;
        self.style.color = Arc::new(c);
        self
    }

    /// Finalizes and wraps into an `Arc<ComputedStyle>`.
    pub fn build(self) -> Arc<ComputedStyle> {
        Arc::new(self.style)
    }
}

/// Helper for constructing synthetic `RenderTreeNode` and `RenderTree` instances.
pub struct RenderTreeBuilder;

impl RenderTreeBuilder {
    /// Constructs a standard element node with an explicit style and children.
    pub fn element(
        id: u64,
        style: Arc<ComputedStyle>,
        children: Vec<RenderTreeNode>,
    ) -> RenderTreeNode {
        let node_id = NodeId::from_raw(id).unwrap_or_default();
        RenderTreeNode::new_element(node_id, style, children)
    }

    /// Constructs a block container node.
    pub fn block(
        id: u64,
        width_px: Option<i32>,
        height_px: Option<i32>,
        children: Vec<RenderTreeNode>,
    ) -> RenderTreeNode {
        let mut sb = StyleBuilder::new().display(Display::Block);
        if let Some(w) = width_px {
            sb = sb.width_px(w);
        }
        if let Some(h) = height_px {
            sb = sb.height_px(h);
        }
        Self::element(id, sb.build(), children)
    }

    /// Constructs an inline container node.
    pub fn inline(id: u64, children: Vec<RenderTreeNode>) -> RenderTreeNode {
        let style = StyleBuilder::new().display(Display::Inline).build();
        Self::element(id, style, children)
    }

    /// Constructs an inline-block container node.
    pub fn inline_block(
        id: u64,
        width_px: Option<i32>,
        height_px: Option<i32>,
        children: Vec<RenderTreeNode>,
    ) -> RenderTreeNode {
        let mut sb = StyleBuilder::new().display(Display::InlineBlock);
        if let Some(w) = width_px {
            sb = sb.width_px(w);
        }
        if let Some(h) = height_px {
            sb = sb.height_px(h);
        }
        Self::element(id, sb.build(), children)
    }

    /// Constructs a leaf text node.
    pub fn text_node(id: u64, style: Arc<ComputedStyle>) -> RenderTreeNode {
        let node_id = NodeId::from_raw(id).unwrap_or_default();
        RenderTreeNode {
            node_id,
            kind: RenderNodeKind::Text,
            style,
            children: Vec::new(),
        }
    }

    /// Constructs a Flexbox container node.
    pub fn flex_container(
        id: u64,
        direction: FlexDirection,
        wrap: FlexWrap,
        children: Vec<RenderTreeNode>,
    ) -> RenderTreeNode {
        let style = StyleBuilder::new()
            .flex_container(
                direction,
                wrap,
                JustifyContent::FlexStart,
                AlignItems::Stretch,
            )
            .build();
        Self::element(id, style, children)
    }

    /// Constructs a CSS Grid container node.
    pub fn grid_container(
        id: u64,
        auto_flow: GridAutoFlow,
        children: Vec<RenderTreeNode>,
    ) -> RenderTreeNode {
        let style = StyleBuilder::new().grid_container(auto_flow).build();
        Self::element(id, style, children)
    }

    /// Constructs a Table container node.
    pub fn table(id: u64, children: Vec<RenderTreeNode>) -> RenderTreeNode {
        let style = StyleBuilder::new().display(Display::Table).build();
        Self::element(id, style, children)
    }

    /// Constructs a TableRow node.
    pub fn table_row(id: u64, children: Vec<RenderTreeNode>) -> RenderTreeNode {
        let style = StyleBuilder::new().display(Display::TableRow).build();
        Self::element(id, style, children)
    }

    /// Constructs a TableCell node.
    pub fn table_cell(id: u64, children: Vec<RenderTreeNode>) -> RenderTreeNode {
        let style = StyleBuilder::new().display(Display::TableCell).build();
        Self::element(id, style, children)
    }

    /// Assembles a complete `RenderTree` by recursively collecting styles into `node_styles`.
    pub fn build_render_tree(root: RenderTreeNode) -> RenderTree {
        let mut node_styles = FxHashMap::default();
        Self::collect_styles(&root, &mut node_styles);
        RenderTree {
            root: Some(root),
            node_styles,
        }
    }

    fn collect_styles(node: &RenderTreeNode, map: &mut FxHashMap<NodeId, Arc<ComputedStyle>>) {
        map.insert(node.node_id, Arc::clone(&node.style));
        for child in &node.children {
            Self::collect_styles(child, map);
        }
    }
}

/// Helper for constructing both a DOM `Document` and its resolved `RenderTree` from raw HTML.
pub fn html_to_dom_and_render_tree(html: &str) -> (Document, RenderTree) {
    let doc = ace_dom::parse_html(html);
    let mut engine = StyleEngine::new();
    let render_tree = engine.compute_styles(&doc);
    (doc, render_tree)
}

// =============================================================================
// 3. Subpixel Snapping Verifiers Checking Zero 1px Gaps
// =============================================================================

/// Diagnostic details describing any 1px gap or overlap violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnappingViolation {
    pub index: usize,
    pub left_edge_snapped: i32,
    pub right_edge_snapped: i32,
    pub difference: i32,
    pub message: String,
}

impl std::fmt::Display for SnappingViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SnappingViolation at index {}: left_edge={}, right_edge={}, diff={} ({})",
            self.index,
            self.left_edge_snapped,
            self.right_edge_snapped,
            self.difference,
            self.message
        )
    }
}

impl std::error::Error for SnappingViolation {}

/// Verifies that a 1D sequence of contiguous intervals exhibits zero pixel cracking and zero 1px gaps.
///
/// Mathematical Theorem:
/// For every adjacent pair $i$ and $i+1$, where `origin[i+1] == origin[i] + size[i]`:
/// $$\text{snapped\_end}[i] == \text{snapped\_origin}[i+1]$$
/// And the sum of snapped sizes equals the snapped total span:
/// $$\sum \text{snapped\_size}[i] == \text{snapped\_total\_size}$$
pub fn verify_zero_pixel_cracking_1d(
    slices: &[(LayoutUnit, LayoutUnit)],
) -> Result<(), SnappingViolation> {
    if slices.is_empty() {
        return Ok(());
    }

    let mut current_expected_origin = slices[0].0;
    let total_start_origin = slices[0].0;
    let mut total_accumulated_size = LayoutUnit::ZERO;

    let mut previous_snapped_end: Option<i32> = None;
    let mut sum_snapped_sizes: i64 = 0;

    for (i, &(orig, sz)) in slices.iter().enumerate() {
        // Invariant: slices must be contiguous in LayoutUnits
        if orig != current_expected_origin {
            return Err(SnappingViolation {
                index: i,
                left_edge_snapped: orig.round_px(),
                right_edge_snapped: current_expected_origin.round_px(),
                difference: orig.round_px() - current_expected_origin.round_px(),
                message: format!(
                    "Non-contiguous input intervals at index {}: expected {:?}, found {:?}",
                    i, current_expected_origin, orig
                ),
            });
        }

        let (snapped_orig, snapped_size) = snap_box(orig, sz);
        let snapped_end = snapped_orig + snapped_size;
        sum_snapped_sizes += snapped_size as i64;

        if let Some(prev_end) = previous_snapped_end {
            if prev_end != snapped_orig {
                let diff = snapped_orig - prev_end;
                return Err(SnappingViolation {
                    index: i,
                    left_edge_snapped: prev_end,
                    right_edge_snapped: snapped_orig,
                    difference: diff,
                    message: format!(
                        "Pixel cracking detected at boundary between slice {} and {}: prev_end={}, next_orig={}, gap/overlap={}px",
                        i - 1, i, prev_end, snapped_orig, diff
                    ),
                });
            }
        }

        previous_snapped_end = Some(snapped_end);
        current_expected_origin = orig + sz;
        total_accumulated_size += sz;
    }

    // Invariant: Total extent check
    let (_, total_snapped_size) = snap_box(total_start_origin, total_accumulated_size);
    if sum_snapped_sizes != total_snapped_size as i64 {
        return Err(SnappingViolation {
            index: slices.len(),
            left_edge_snapped: sum_snapped_sizes as i32,
            right_edge_snapped: total_snapped_size,
            difference: (sum_snapped_sizes as i32) - total_snapped_size,
            message: format!(
                "Cumulative size mismatch: sum(snapped_sizes)={}, total_snapped_size={}",
                sum_snapped_sizes, total_snapped_size
            ),
        });
    }

    Ok(())
}

/// Asserts zero pixel cracking across contiguous slices, panicking with diagnostic details on failure.
#[inline]
pub fn assert_zero_gaps_1d(slices: &[(LayoutUnit, LayoutUnit)]) {
    if let Err(err) = verify_zero_pixel_cracking_1d(slices) {
        panic!("FATAL: Subpixel snapping accuracy violation: {}", err);
    }
}

/// Asserts zero pixel cracking between two adjacent boxes.
#[inline]
pub fn assert_zero_gaps_adjacent(origin: LayoutUnit, size1: LayoutUnit, size2: LayoutUnit) {
    let slices = [(origin, size1), (origin + size1, size2)];
    assert_zero_gaps_1d(&slices);
}

// =============================================================================
// 4. Performance Benchmark & Telemetry Helpers
// =============================================================================

/// Performance measurement timer for stress tests.
#[derive(Debug, Clone)]
pub struct BenchmarkTimer {
    start: Instant,
}

impl BenchmarkTimer {
    /// Starts the benchmark timer.
    #[inline]
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Elapsed wall-clock duration.
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Elapsed microseconds.
    #[inline]
    pub fn elapsed_micros(&self) -> u128 {
        self.start.elapsed().as_micros()
    }

    /// Produces a telemetry report for `total_ops` iterations.
    pub fn report(&self, total_ops: usize, operation_name: &str) -> BenchmarkReport {
        let elapsed = self.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();
        let micros_per_op = if total_ops > 0 {
            (elapsed.as_micros() as f64) / (total_ops as f64)
        } else {
            0.0
        };
        let ops_per_sec = if elapsed_secs > 0.0 {
            (total_ops as f64) / elapsed_secs
        } else {
            0.0
        };

        BenchmarkReport {
            operation_name: operation_name.to_string(),
            total_ops,
            elapsed_secs,
            micros_per_op,
            ops_per_second: ops_per_sec,
        }
    }
}

/// Performance telemetry report.
#[derive(Debug, Clone)]
pub struct BenchmarkReport {
    pub operation_name: String,
    pub total_ops: usize,
    pub elapsed_secs: f64,
    pub micros_per_op: f64,
    pub ops_per_second: f64,
}

impl BenchmarkReport {
    /// Asserts that average latency per operation does not exceed the target threshold in microseconds.
    pub fn assert_max_latency_us(&self, max_us: f64, context: &str) {
        println!(
            "[{}] {} ops in {:.4}s ({:.2} µs/op, {:.0} ops/s)",
            self.operation_name,
            self.total_ops,
            self.elapsed_secs,
            self.micros_per_op,
            self.ops_per_second
        );
        assert!(
            self.micros_per_op <= max_us,
            "SLA violation in {}: average latency {:.2} µs/op exceeded threshold {:.2} µs/op (total ops: {})",
            context, self.micros_per_op, max_us, self.total_ops
        );
    }
}

// =============================================================================
// 5. Unit Tests for Test Harness Utilities
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_font_measurer_deterministic() {
        let measurer = MockFontMeasurer::new();
        let font = StyleFont::initial();

        // 8px per glyph = 480 au per char
        let (w, h) = measurer.measure_text("hello", &font);
        assert_eq!(w, LayoutUnit::from_px(40)); // 5 * 8px = 40px
        assert_eq!(h, LayoutUnit::from_px(18)); // 12 + 4 + 2 = 18px

        let metrics = measurer.font_metrics(&font);
        assert_eq!(metrics.ascent, LayoutUnit::from_px(12));
        assert_eq!(metrics.descent, LayoutUnit::from_px(4));
        assert_eq!(metrics.line_gap, LayoutUnit::from_px(2));
        assert_eq!(metrics.line_height(), LayoutUnit::from_px(18));
    }

    #[test]
    fn test_style_builder_fluent_api() {
        let style = StyleBuilder::new()
            .display(Display::Flex)
            .width_px(300)
            .height_px(200)
            .margin_px(10, 20, 30, 40)
            .padding_px(5, 5, 5, 5)
            .border_px(1, 1, 1, 1)
            .flex_container(
                FlexDirection::Row,
                FlexWrap::Wrap,
                JustifyContent::SpaceBetween,
                AlignItems::Center,
            )
            .z_index(42)
            .build();

        assert_eq!(style.display(), Display::Flex);
        assert_eq!(style.width(), Some(LayoutUnit::from_px(300)));
        assert_eq!(style.height(), Some(LayoutUnit::from_px(200)));
        assert_eq!(style.margin_top(), Some(LayoutUnit::from_px(10)));
        assert_eq!(style.margin_right(), Some(LayoutUnit::from_px(20)));
        assert_eq!(style.margin_bottom(), Some(LayoutUnit::from_px(30)));
        assert_eq!(style.margin_left(), Some(LayoutUnit::from_px(40)));
        assert_eq!(style.z_index(), ZIndex::Integer(42));
    }

    #[test]
    fn test_render_tree_builder_hierarchy() {
        let child1 = RenderTreeBuilder::block(2, Some(100), Some(50), Vec::new());
        let child2 = RenderTreeBuilder::block(3, Some(200), Some(50), Vec::new());
        let root = RenderTreeBuilder::block(1, Some(400), Some(100), vec![child1, child2]);

        let tree = RenderTreeBuilder::build_render_tree(root);
        assert!(tree.root.is_some());
        assert_eq!(tree.node_styles.len(), 3);
    }

    #[test]
    fn test_subpixel_snapping_accuracy_thirds() {
        // Three consecutive 33.333px boxes summing to 100px: 2000 au each
        let origin = LayoutUnit::ZERO;
        let third = LayoutUnit::from_raw(2000); // 33.333px
        let slices = [
            (origin, third),
            (origin + third, third),
            (origin + third + third, third),
        ];

        let result = verify_zero_pixel_cracking_1d(&slices);
        assert!(
            result.is_ok(),
            "Snapping thirds must not crack: {:?}",
            result
        );

        // Individual snapped boxes:
        let (s0_orig, s0_size) = snap_box(slices[0].0, slices[0].1);
        let (s1_orig, s1_size) = snap_box(slices[1].0, slices[1].1);
        let (s2_orig, s2_size) = snap_box(slices[2].0, slices[2].1);

        assert_eq!(s0_orig, 0);
        assert_eq!(s0_size, 33);
        assert_eq!(s1_orig, 33);
        assert_eq!(s1_size, 34);
        assert_eq!(s2_orig, 67);
        assert_eq!(s2_size, 33);
        assert_eq!(s0_size + s1_size + s2_size, 100);
    }

    #[test]
    fn test_subpixel_snapping_accuracy_sevenths() {
        // Seven boxes of 14.2857px each in a 100px container
        let total_size = LayoutUnit::from_px(100);
        let base_raw = total_size.raw() / 7; // 6000 / 7 = 857
        let remainder = total_size.raw() % 7; // 1

        let mut slices = Vec::new();
        let mut curr_orig = LayoutUnit::ZERO;

        for i in 0..7 {
            let raw = if i < remainder {
                base_raw + 1
            } else {
                base_raw
            };
            let size = LayoutUnit::from_raw(raw);
            slices.push((curr_orig, size));
            curr_orig += size;
        }

        assert_zero_gaps_1d(&slices);
    }

    #[test]
    fn test_snapping_violation_detection() {
        // Artificially broken contiguous sequence
        let slices = [
            (LayoutUnit::from_px(0), LayoutUnit::from_px(10)),
            (LayoutUnit::from_px(12), LayoutUnit::from_px(10)), // 2px gap in input!
        ];

        let result = verify_zero_pixel_cracking_1d(&slices);
        assert!(result.is_err());
    }

    #[test]
    fn test_benchmark_timer() {
        let timer = BenchmarkTimer::start();
        std::thread::sleep(Duration::from_millis(5));
        let report = timer.report(100, "sample_op");
        assert!(report.elapsed_secs > 0.003);
        assert!(report.micros_per_op > 0.0);
    }
}
