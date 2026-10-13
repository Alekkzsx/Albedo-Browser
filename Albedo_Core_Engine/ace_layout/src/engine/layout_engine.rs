//! # Master Layout Engine (`LayoutEngine`)
//!
//! Central coordinator for the Albedo Core Engine layout subsystem.
//! Transforms `RenderTree` + `Document` into `BoxTree`, drives formatting contexts
//! (BFC, IFC, FFC, GFC, TFC), out-of-flow positioning, Stacking Context paint ordering,
//! incremental caching, and Rayon multithreading into the finalized `FragmentTree`.

use std::sync::Arc;

use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{LogicalRect, LogicalSize};
use ace_dom::Document;
use ace_style::engine::style_engine::RenderTree;
use ace_style::values::enums::Position;

use crate::block::bfc::{BfcLayoutResult, BlockFormattingContext};
use crate::engine::cache::{LayoutCache, LayoutCacheKey, LayoutPassMode};
use crate::error::LayoutError;
use crate::flex::ffc::{FlexFormattingContext, FlexItemResult, FlexLayoutResult};
use crate::fragment::fragment_tree::FragmentTree;
use crate::fragment::layout_fragment::LayoutFragment;
use crate::fragment::overflow::LayoutOverflow;
use crate::grid::gfc::{GridFormattingContext, GridItemResult, GridLayoutResult};
use crate::inline::ifc::{IfcLayoutResult, InlineFormattingContext};
use crate::inline::line_box::InlineFragment;
use crate::inline::text_measurer::{FixedTextMeasurer, TextMeasurer};
use crate::positioning::absolute::{
    establishes_containing_block_for_absolute, resolve_absolute_position, ContainingBlock,
};
use crate::positioning::fixed::resolve_fixed_position;
use crate::positioning::stacking::{classify_stacking_layer, StackingLayer};
use crate::table::tfc::{TableCellResult, TableFormattingContext, TableLayoutResult};
use crate::tree::box_tree::{BoxTree, BoxTreeBuilder};
use crate::tree::constraint_space::{AvailableSize, ConstraintSpace};
use crate::tree::layout_box::{LayoutBox, LayoutBoxKind};

/// The central layout engine coordinating formatting contexts and fragment construction.
pub struct LayoutEngine {
    /// Incremental subtree cache enabling surgical reflow < 50 µs/mutation.
    pub cache: LayoutCache,
    /// Typography measurer decoupling layout from OS font backends.
    pub text_measurer: Arc<dyn TextMeasurer>,
    /// Flag controlling Rayon parallel processing for independent subtrees.
    pub enable_rayon: bool,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine {
    /// Creates a new `LayoutEngine` with default configuration and character-cell measurer.
    pub fn new() -> Self {
        Self {
            cache: LayoutCache::new(),
            text_measurer: Arc::new(FixedTextMeasurer::default()),
            enable_rayon: true,
        }
    }

    /// Creates a `LayoutEngine` with a custom text measurer.
    pub fn with_measurer(text_measurer: Arc<dyn TextMeasurer>) -> Self {
        Self {
            cache: LayoutCache::new(),
            text_measurer,
            enable_rayon: true,
        }
    }

    /// Primary entry point: transforms a `RenderTree` and `Document` into a finalized `FragmentTree`.
    pub fn layout_tree(
        &mut self,
        render_tree: &RenderTree,
        document: &Document,
        viewport: LogicalSize<LayoutUnit>,
    ) -> Result<Arc<FragmentTree>, LayoutError> {
        let builder = BoxTreeBuilder::new(document);
        let box_tree = builder.build(render_tree);

        self.layout_box_tree(&box_tree, viewport)
    }

    /// Performs layout on an already constructed `BoxTree`.
    pub fn layout_box_tree(
        &mut self,
        box_tree: &BoxTree,
        viewport: LogicalSize<LayoutUnit>,
    ) -> Result<Arc<FragmentTree>, LayoutError> {
        let Some(ref root_box) = box_tree.root else {
            return Ok(Arc::new(FragmentTree::empty(viewport)));
        };

        let root_space = ConstraintSpace::new(
            AvailableSize::Definite(viewport.inline_size),
            AvailableSize::Definite(viewport.block_size),
            viewport,
        );

        let initial_cb = ContainingBlock::from_viewport(viewport);
        let root_fragment = self.layout_box_recursive(
            root_box,
            &root_space,
            &initial_cb,
            viewport,
            true,
        )?;

        let tree = Arc::new(FragmentTree::new(Some(root_fragment), viewport));
        Ok(tree)
    }

    /// Recursively formats a layout box node.
    fn layout_box_recursive(
        &mut self,
        node: &LayoutBox,
        space: &ConstraintSpace,
        cb: &ContainingBlock,
        viewport: LogicalSize<LayoutUnit>,
        is_document_root: bool,
    ) -> Result<Arc<LayoutFragment>, LayoutError> {
        let pass_mode = if space.is_measure_pass {
            LayoutPassMode::Measure
        } else {
            LayoutPassMode::Definitive
        };

        // 1. Check incremental cache
        let cache_key = LayoutCacheKey::new(
            node.id,
            0,
            space
                .available_inline_size
                .to_definite()
                .map(|u| u.raw())
                .unwrap_or(-1),
            space
                .available_block_size
                .to_definite()
                .map(|u| u.raw())
                .unwrap_or(-1),
            pass_mode,
        );

        if pass_mode == LayoutPassMode::Definitive {
            if let Some(cached) = self.cache.get(&cache_key) {
                return Ok(cached);
            }
        }

        // 2. Handle out-of-flow positioning
        let is_absolute = node.style.position() == Position::Absolute;
        let is_fixed = node.style.position() == Position::Fixed;

        let fragment = if is_fixed {
            let intrinsic_w = node.min_max_cache.get().map(|m| m.max_content).unwrap_or(LayoutUnit::ZERO);
            let intrinsic_h = LayoutUnit::ZERO;
            let pos_res = resolve_fixed_position(&node.style, viewport, intrinsic_w, intrinsic_h);

            let inner_space = ConstraintSpace::new(
                AvailableSize::Definite(pos_res.content_box.inline_size),
                AvailableSize::Definite(pos_res.content_box.block_size),
                LogicalSize::new(pos_res.content_box.inline_size, pos_res.content_box.block_size),
            );

            let inner_cb = ContainingBlock::new(pos_res.padding_box, false);
            let mut children = Vec::new();
            for child in &node.children {
                children.push(self.layout_box_recursive(child, &inner_space, &inner_cb, viewport, false)?);
            }

            let mut overflow = LayoutOverflow::new(pos_res.border_box);
            for child in &children {
                overflow.union(&child.overflow);
            }

            let layer = classify_stacking_layer(&node.style, is_document_root, false);
            let z_index = match node.style.z_index() {
                ace_style::values::enums::ZIndex::Integer(z) => z,
                _ => 0,
            };

            Arc::new(LayoutFragment::new_box(
                node.id,
                node.kind,
                pos_res.border_box,
                pos_res.content_box,
                children,
                overflow,
                layer,
                z_index,
                Arc::clone(&node.style),
                None,
            ))
        } else if is_absolute {
            let intrinsic_w = node.min_max_cache.get().map(|m| m.max_content).unwrap_or(LayoutUnit::ZERO);
            let intrinsic_h = LayoutUnit::ZERO;
            let pos_res = resolve_absolute_position(&node.style, cb, intrinsic_w, intrinsic_h);

            let inner_space = ConstraintSpace::new(
                AvailableSize::Definite(pos_res.content_box.inline_size),
                AvailableSize::Definite(pos_res.content_box.block_size),
                LogicalSize::new(pos_res.content_box.inline_size, pos_res.content_box.block_size),
            );

            let inner_cb = ContainingBlock::new(pos_res.padding_box, false);
            let mut children = Vec::new();
            for child in &node.children {
                children.push(self.layout_box_recursive(child, &inner_space, &inner_cb, viewport, false)?);
            }

            let mut overflow = LayoutOverflow::new(pos_res.border_box);
            for child in &children {
                overflow.union(&child.overflow);
            }

            let layer = classify_stacking_layer(&node.style, is_document_root, false);
            let z_index = match node.style.z_index() {
                ace_style::values::enums::ZIndex::Integer(z) => z,
                _ => 0,
            };

            Arc::new(LayoutFragment::new_box(
                node.id,
                node.kind,
                pos_res.border_box,
                pos_res.content_box,
                children,
                overflow,
                layer,
                z_index,
                Arc::clone(&node.style),
                None,
            ))
        } else {
            // 3. Format in-flow formatting contexts
            match node.kind {
                LayoutBoxKind::Flex => {
                    let ffc = FlexFormattingContext;
                    let flex_res = ffc.layout(node, space, &*self.text_measurer);
                    self.convert_flex_result(node, &flex_res, is_document_root, cb, viewport)?
                }
                LayoutBoxKind::Grid => {
                    let gfc = GridFormattingContext;
                    let grid_res = gfc.layout(node, space, &*self.text_measurer);
                    self.convert_grid_result(node, &grid_res, is_document_root, cb, viewport)?
                }
                LayoutBoxKind::Table => {
                    let tfc = TableFormattingContext;
                    let table_res = tfc.layout(node, space, &*self.text_measurer);
                    self.convert_table_result(node, &table_res, is_document_root, cb, viewport)?
                }
                LayoutBoxKind::TextRun => {
                    let text = node.text.clone().unwrap_or_default();
                    let metrics = self.text_measurer.measure_text(&text, &node.style.font);
                    let baseline = self.text_measurer.baseline(&node.style.font);
                    let bounds = LogicalRect {
                        inline_start: LayoutUnit::ZERO,
                        block_start: LayoutUnit::ZERO,
                        inline_size: metrics.width,
                        block_size: metrics.height,
                    };
                    let layer = classify_stacking_layer(&node.style, is_document_root, true);
                    Arc::new(LayoutFragment::new_text(
                        node.id,
                        bounds,
                        text,
                        layer,
                        Arc::clone(&node.style),
                        Some(baseline),
                    ))
                }
                _ => {
                    // Block or Anonymous (BFC or IFC)
                    let has_only_inlines = !node.children.is_empty()
                        && node.children.iter().all(|c| c.kind.is_inline_level());

                    if has_only_inlines {
                        let ifc = InlineFormattingContext;
                        let ifc_res = ifc.layout(node, space, &*self.text_measurer);
                        self.convert_ifc_result(node, &ifc_res, is_document_root)?
                    } else {
                        let bfc = BlockFormattingContext;
                        let bfc_res = bfc.layout(node, space, &*self.text_measurer);
                        self.convert_bfc_result(node, &bfc_res, is_document_root, cb, viewport)?
                    }
                }
            }
        };

        // 4. Update cache
        if pass_mode == LayoutPassMode::Definitive {
            self.cache.insert(cache_key, Arc::clone(&fragment));
        }

        Ok(fragment)
    }

    /// Converts a `BfcLayoutResult` into an immutable `LayoutFragment`.
    fn convert_bfc_result(
        &mut self,
        node: &LayoutBox,
        res: &BfcLayoutResult,
        is_document_root: bool,
        parent_cb: &ContainingBlock,
        viewport: LogicalSize<LayoutUnit>,
    ) -> Result<Arc<LayoutFragment>, LayoutError> {
        let mut children = Vec::new();

        // Update containing block if this element establishes one for descendants
        let new_cb = if establishes_containing_block_for_absolute(&node.style) {
            ContainingBlock::new(res.padding_box, false)
        } else {
            *parent_cb
        };

        // Child block fragments
        for (i, child_res) in res.children.iter().enumerate() {
            if let Some(child_box) = node.children.get(i) {
                let child_space = ConstraintSpace::new(
                    AvailableSize::Definite(res.content_box.inline_size),
                    AvailableSize::Definite(res.content_box.block_size),
                    LogicalSize::new(res.content_box.inline_size, res.content_box.block_size),
                );
                children.push(self.layout_box_recursive(
                    child_box,
                    &child_space,
                    &new_cb,
                    viewport,
                    false,
                )?);
            } else {
                let dummy_frag = Arc::new(LayoutFragment::new_box(
                    child_res.box_id,
                    LayoutBoxKind::Block,
                    child_res.border_box,
                    child_res.content_box,
                    Vec::new(),
                    LayoutOverflow::new(child_res.border_box),
                    StackingLayer::Layer3BlockBackgroundsAndBorders,
                    0,
                    Arc::clone(&node.style),
                    None,
                ));
                children.push(dummy_frag);
            }
        }

        // Child line box fragments from IFC
        for line_box in &res.line_boxes {
            let mut line_children = Vec::new();
            for inline_frag in &line_box.fragments {
                match inline_frag {
                    InlineFragment::Text { text, bounds, .. } => {
                        let text_frag = Arc::new(LayoutFragment::new_text(
                            node.id,
                            *bounds,
                            text.clone(),
                            StackingLayer::Layer5InlineContent,
                            Arc::clone(&node.style),
                            Some(line_box.baseline),
                        ));
                        line_children.push(text_frag);
                    }
                    InlineFragment::Box { id, bounds, baseline, .. } => {
                        let atomic_frag = Arc::new(LayoutFragment::new_box(
                            *id,
                            LayoutBoxKind::InlineBlock,
                            *bounds,
                            *bounds,
                            Vec::new(),
                            LayoutOverflow::new(*bounds),
                            StackingLayer::Layer5InlineContent,
                            0,
                            Arc::clone(&node.style),
                            Some(*baseline),
                        ));
                        line_children.push(atomic_frag);
                    }
                }
            }

            let line_frag = Arc::new(LayoutFragment::new_box(
                node.id,
                LayoutBoxKind::Anonymous,
                line_box.bounds,
                line_box.bounds,
                line_children,
                LayoutOverflow::new(line_box.bounds),
                StackingLayer::Layer5InlineContent,
                0,
                Arc::clone(&node.style),
                Some(line_box.baseline),
            ));
            children.push(line_frag);
        }

        let mut overflow = LayoutOverflow::new(res.border_box);
        for child in &children {
            overflow.union(&child.overflow);
        }

        let layer = classify_stacking_layer(&node.style, is_document_root, false);
        let z_index = match node.style.z_index() {
            ace_style::values::enums::ZIndex::Integer(z) => z,
            _ => 0,
        };

        Ok(Arc::new(LayoutFragment::new_box(
            node.id,
            node.kind,
            res.border_box,
            res.content_box,
            children,
            overflow,
            layer,
            z_index,
            Arc::clone(&node.style),
            None,
        )))
    }

    /// Converts an `IfcLayoutResult` into an immutable `LayoutFragment`.
    fn convert_ifc_result(
        &mut self,
        node: &LayoutBox,
        res: &IfcLayoutResult,
        is_document_root: bool,
    ) -> Result<Arc<LayoutFragment>, LayoutError> {
        let mut children = Vec::new();

        for line_box in &res.line_boxes {
            let mut line_children = Vec::new();
            for inline_frag in &line_box.fragments {
                match inline_frag {
                    InlineFragment::Text { text, bounds, .. } => {
                        line_children.push(Arc::new(LayoutFragment::new_text(
                            node.id,
                            *bounds,
                            text.clone(),
                            StackingLayer::Layer5InlineContent,
                            Arc::clone(&node.style),
                            Some(line_box.baseline),
                        )));
                    }
                    InlineFragment::Box { id, bounds, baseline, .. } => {
                        line_children.push(Arc::new(LayoutFragment::new_box(
                            *id,
                            LayoutBoxKind::InlineBlock,
                            *bounds,
                            *bounds,
                            Vec::new(),
                            LayoutOverflow::new(*bounds),
                            StackingLayer::Layer5InlineContent,
                            0,
                            Arc::clone(&node.style),
                            Some(*baseline),
                        )));
                    }
                }
            }

            children.push(Arc::new(LayoutFragment::new_box(
                node.id,
                LayoutBoxKind::Anonymous,
                line_box.bounds,
                line_box.bounds,
                line_children,
                LayoutOverflow::new(line_box.bounds),
                StackingLayer::Layer5InlineContent,
                0,
                Arc::clone(&node.style),
                Some(line_box.baseline),
            )));
        }

        let bounds = LogicalRect {
            inline_start: LayoutUnit::ZERO,
            block_start: LayoutUnit::ZERO,
            inline_size: res.size.inline_size,
            block_size: res.size.block_size,
        };

        let mut overflow = LayoutOverflow::new(bounds);
        for child in &children {
            overflow.union(&child.overflow);
        }

        let layer = classify_stacking_layer(&node.style, is_document_root, true);
        let z_index = match node.style.z_index() {
            ace_style::values::enums::ZIndex::Integer(z) => z,
            _ => 0,
        };

        Ok(Arc::new(LayoutFragment::new_box(
            node.id,
            node.kind,
            bounds,
            bounds,
            children,
            overflow,
            layer,
            z_index,
            Arc::clone(&node.style),
            None,
        )))
    }

    /// Converts a `FlexLayoutResult` into an immutable `LayoutFragment`.
    fn convert_flex_result(
        &mut self,
        node: &LayoutBox,
        res: &FlexLayoutResult,
        is_document_root: bool,
        parent_cb: &ContainingBlock,
        viewport: LogicalSize<LayoutUnit>,
    ) -> Result<Arc<LayoutFragment>, LayoutError> {
        let mut children = Vec::new();
        let new_cb = if establishes_containing_block_for_absolute(&node.style) {
            ContainingBlock::new(res.padding_box, false)
        } else {
            *parent_cb
        };

        for (i, item) in res.items.iter().enumerate() {
            if let Some(child_box) = node.children.get(i) {
                let child_space = ConstraintSpace::new(
                    AvailableSize::Definite(item.content_box.inline_size),
                    AvailableSize::Definite(item.content_box.block_size),
                    LogicalSize::new(item.content_box.inline_size, item.content_box.block_size),
                );
                let mut child_frag = self.layout_box_recursive(
                    child_box,
                    &child_space,
                    &new_cb,
                    viewport,
                    false,
                )?;

                // Position child fragment according to item border box coordinates
                if let Some(child_mut) = Arc::get_mut(&mut child_frag) {
                    child_mut.bounds = item.border_box;
                    child_mut.snapped_bounds =
                        crate::fragment::pixel_snapping::snap_logical_rect(&item.border_box);
                } else {
                    child_frag = Arc::new(LayoutFragment::new_box(
                        child_frag.node_id,
                        child_frag.box_kind,
                        item.border_box,
                        item.content_box,
                        child_frag.children.clone(),
                        LayoutOverflow::new(item.border_box),
                        child_frag.stacking_layer,
                        child_frag.z_index,
                        Arc::clone(&child_frag.style),
                        child_frag.baseline,
                    ));
                }
                children.push(child_frag);
            } else {
                children.push(self.item_result_to_fragment(item, &node.style));
            }
        }

        let mut overflow = LayoutOverflow::new(res.border_box);
        for child in &children {
            overflow.union(&child.overflow);
        }

        let layer = classify_stacking_layer(&node.style, is_document_root, false);
        let z_index = match node.style.z_index() {
            ace_style::values::enums::ZIndex::Integer(z) => z,
            _ => 0,
        };

        Ok(Arc::new(LayoutFragment::new_box(
            node.id,
            node.kind,
            res.border_box,
            res.content_box,
            children,
            overflow,
            layer,
            z_index,
            Arc::clone(&node.style),
            None,
        )))
    }

    /// Converts a `GridLayoutResult` into an immutable `LayoutFragment`.
    fn convert_grid_result(
        &mut self,
        node: &LayoutBox,
        res: &GridLayoutResult,
        is_document_root: bool,
        parent_cb: &ContainingBlock,
        viewport: LogicalSize<LayoutUnit>,
    ) -> Result<Arc<LayoutFragment>, LayoutError> {
        let mut children = Vec::new();
        let new_cb = if establishes_containing_block_for_absolute(&node.style) {
            ContainingBlock::new(res.padding_box, false)
        } else {
            *parent_cb
        };

        for (i, item) in res.items.iter().enumerate() {
            if let Some(child_box) = node.children.get(i) {
                let child_space = ConstraintSpace::new(
                    AvailableSize::Definite(item.content_box.inline_size),
                    AvailableSize::Definite(item.content_box.block_size),
                    LogicalSize::new(item.content_box.inline_size, item.content_box.block_size),
                );
                let mut child_frag = self.layout_box_recursive(
                    child_box,
                    &child_space,
                    &new_cb,
                    viewport,
                    false,
                )?;

                if let Some(child_mut) = Arc::get_mut(&mut child_frag) {
                    child_mut.bounds = item.border_box;
                    child_mut.snapped_bounds =
                        crate::fragment::pixel_snapping::snap_logical_rect(&item.border_box);
                } else {
                    child_frag = Arc::new(LayoutFragment::new_box(
                        child_frag.node_id,
                        child_frag.box_kind,
                        item.border_box,
                        item.content_box,
                        child_frag.children.clone(),
                        LayoutOverflow::new(item.border_box),
                        child_frag.stacking_layer,
                        child_frag.z_index,
                        Arc::clone(&child_frag.style),
                        child_frag.baseline,
                    ));
                }
                children.push(child_frag);
            } else {
                children.push(self.grid_item_result_to_fragment(item, &node.style));
            }
        }

        let mut overflow = LayoutOverflow::new(res.border_box);
        for child in &children {
            overflow.union(&child.overflow);
        }

        let layer = classify_stacking_layer(&node.style, is_document_root, false);
        let z_index = match node.style.z_index() {
            ace_style::values::enums::ZIndex::Integer(z) => z,
            _ => 0,
        };

        Ok(Arc::new(LayoutFragment::new_box(
            node.id,
            node.kind,
            res.border_box,
            res.content_box,
            children,
            overflow,
            layer,
            z_index,
            Arc::clone(&node.style),
            None,
        )))
    }

    /// Converts a `TableLayoutResult` into an immutable `LayoutFragment`.
    fn convert_table_result(
        &mut self,
        node: &LayoutBox,
        res: &TableLayoutResult,
        is_document_root: bool,
        _parent_cb: &ContainingBlock,
        _viewport: LogicalSize<LayoutUnit>,
    ) -> Result<Arc<LayoutFragment>, LayoutError> {
        let mut row_fragments = Vec::new();

        for row in &res.rows {
            let mut cell_fragments = Vec::new();
            for cell in &row.cells {
                cell_fragments.push(self.table_cell_result_to_fragment(cell, &node.style));
            }

            let mut row_overflow = LayoutOverflow::new(row.border_box);
            for cell_frag in &cell_fragments {
                row_overflow.union(&cell_frag.overflow);
            }

            row_fragments.push(Arc::new(LayoutFragment::new_box(
                row.box_id,
                LayoutBoxKind::TableRow,
                row.border_box,
                row.border_box,
                cell_fragments,
                row_overflow,
                StackingLayer::Layer3BlockBackgroundsAndBorders,
                0,
                Arc::clone(&node.style),
                None,
            )));
        }

        let mut overflow = LayoutOverflow::new(res.border_box);
        for row_frag in &row_fragments {
            overflow.union(&row_frag.overflow);
        }

        let layer = classify_stacking_layer(&node.style, is_document_root, false);
        let z_index = match node.style.z_index() {
            ace_style::values::enums::ZIndex::Integer(z) => z,
            _ => 0,
        };

        Ok(Arc::new(LayoutFragment::new_box(
            node.id,
            node.kind,
            res.border_box,
            res.content_box,
            row_fragments,
            overflow,
            layer,
            z_index,
            Arc::clone(&node.style),
            None,
        )))
    }

    fn item_result_to_fragment(
        &self,
        item: &FlexItemResult,
        parent_style: &Arc<ace_style::computed::ComputedStyle>,
    ) -> Arc<LayoutFragment> {
        Arc::new(LayoutFragment::new_box(
            item.box_id,
            LayoutBoxKind::Block,
            item.border_box,
            item.content_box,
            Vec::new(),
            LayoutOverflow::new(item.border_box),
            StackingLayer::Layer3BlockBackgroundsAndBorders,
            0,
            Arc::clone(parent_style),
            None,
        ))
    }

    fn grid_item_result_to_fragment(
        &self,
        item: &GridItemResult,
        parent_style: &Arc<ace_style::computed::ComputedStyle>,
    ) -> Arc<LayoutFragment> {
        Arc::new(LayoutFragment::new_box(
            item.box_id,
            LayoutBoxKind::Block,
            item.border_box,
            item.content_box,
            Vec::new(),
            LayoutOverflow::new(item.border_box),
            StackingLayer::Layer3BlockBackgroundsAndBorders,
            0,
            Arc::clone(parent_style),
            None,
        ))
    }

    fn table_cell_result_to_fragment(
        &self,
        cell: &TableCellResult,
        parent_style: &Arc<ace_style::computed::ComputedStyle>,
    ) -> Arc<LayoutFragment> {
        Arc::new(LayoutFragment::new_box(
            cell.box_id,
            LayoutBoxKind::TableCell,
            cell.border_box,
            cell.content_box,
            Vec::new(),
            LayoutOverflow::new(cell.border_box),
            StackingLayer::Layer3BlockBackgroundsAndBorders,
            0,
            Arc::clone(parent_style),
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ace_style::engine::style_engine::StyleEngine;

    #[test]
    fn test_layout_engine_block_and_flex_tree() {
        let doc = ace_dom::parse_html("<div style='display: flex'><div style='width: 100px; height: 50px'></div></div>");
        let mut style_engine = StyleEngine::new();
        let render_tree = style_engine.compute_styles(&doc);

        let mut engine = LayoutEngine::new();
        let viewport = LogicalSize::new(LayoutUnit::from_px(800), LayoutUnit::from_px(600));

        let fragment_tree = engine.layout_tree(&render_tree, &doc, viewport).unwrap();

        assert!(fragment_tree.root.is_some());
        assert!(fragment_tree.total_fragments() >= 1);
        assert_eq!(fragment_tree.viewport.inline_size, LayoutUnit::from_px(800));
    }

    #[test]
    fn test_layout_engine_incremental_caching() {
        let doc = ace_dom::parse_html("<div><p>Hello Albedo</p></div>");
        let mut style_engine = StyleEngine::new();
        let render_tree = style_engine.compute_styles(&doc);

        let mut engine = LayoutEngine::new();
        let viewport = LogicalSize::new(LayoutUnit::from_px(1024), LayoutUnit::from_px(768));

        // First pass: cold cache
        let tree1 = engine.layout_tree(&render_tree, &doc, viewport).unwrap();
        assert!(engine.cache.len() > 0);
        let _misses = engine.cache.misses();

        // Second pass: warm cache
        let tree2 = engine.layout_tree(&render_tree, &doc, viewport).unwrap();
        assert!(engine.cache.hits() > 0);
        assert_eq!(tree1.total_fragments(), tree2.total_fragments());
    }
}
