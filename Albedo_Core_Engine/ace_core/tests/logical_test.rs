use ace_core::math::geometry::{point2, rect, size2};
use ace_core::math::layout_unit::LayoutUnit;
use ace_core::math::logical::{
    Direction, LayoutEdgeInsets, LayoutPoint, LayoutRect, LayoutSize, LogicalLayoutPoint,
    LogicalLayoutRect, LogicalLayoutSides, LogicalLayoutSize, LogicalPoint, LogicalRect,
    LogicalSides, LogicalSize, WritingMode,
};
use ace_core::math::units::{CssPixel, LayoutPixel};
use euclid::{Point2D, Rect, SideOffsets2D, Size2D};

#[test]
fn test_logical_point_and_size_creation() {
    let pt: LogicalPoint<f32> = LogicalPoint::new(10.0, 20.0);
    assert_eq!(pt.inline, 10.0);
    assert_eq!(pt.block, 20.0);

    let sz: LogicalSize<f32> = LogicalSize::new(100.0, 50.0);
    assert_eq!(sz.inline_size, 100.0);
    assert_eq!(sz.block_size, 50.0);
}

#[test]
fn test_logical_rect_origin_and_size() {
    let rect = LogicalRect::new(5.0, 10.0, 200.0, 100.0);
    assert_eq!(rect.origin(), LogicalPoint::new(5.0, 10.0));
    assert_eq!(rect.size(), LogicalSize::new(200.0, 100.0));
}

#[test]
fn test_horizontal_tb_ltr_conversion() {
    let container_size: Size2D<f32, CssPixel> = size2(1000.0, 800.0);
    let logical = LogicalRect::new(10.0, 20.0, 300.0, 150.0);

    let physical = logical.to_physical(WritingMode::HorizontalTb, Direction::Ltr, container_size);
    assert_eq!(physical.origin.x, 10.0);
    assert_eq!(physical.origin.y, 20.0);
    assert_eq!(physical.size.width, 300.0);
    assert_eq!(physical.size.height, 150.0);

    let back_to_logical = LogicalRect::from_physical(physical, WritingMode::HorizontalTb, Direction::Ltr, container_size);
    assert_eq!(back_to_logical, logical);
}

#[test]
fn test_horizontal_tb_rtl_conversion() {
    let container_size: Size2D<f32, CssPixel> = size2(1000.0, 800.0);
    let logical = LogicalRect::new(10.0, 20.0, 300.0, 150.0);

    let physical = logical.to_physical(WritingMode::HorizontalTb, Direction::Rtl, container_size);
    // Em RTL: x = container_w - inline_start - inline_size = 1000 - 10 - 300 = 690
    assert_eq!(physical.origin.x, 690.0);
    assert_eq!(physical.origin.y, 20.0);
    assert_eq!(physical.size.width, 300.0);
    assert_eq!(physical.size.height, 150.0);

    let back_to_logical = LogicalRect::from_physical(physical, WritingMode::HorizontalTb, Direction::Rtl, container_size);
    assert_eq!(back_to_logical, logical);
}

#[test]
fn test_vertical_rl_conversion() {
    let container_size: Size2D<f32, CssPixel> = size2(1000.0, 800.0);
    let logical = LogicalRect::new(15.0, 25.0, 400.0, 200.0);

    let physical = logical.to_physical(WritingMode::VerticalRl, Direction::Ltr, container_size);
    // Em Vertical-RL LTR:
    // x = container_w - block_start - block_size = 1000 - 25 - 200 = 775
    // y = inline_start = 15
    // width = block_size = 200
    // height = inline_size = 400
    assert_eq!(physical.origin.x, 775.0);
    assert_eq!(physical.origin.y, 15.0);
    assert_eq!(physical.size.width, 200.0);
    assert_eq!(physical.size.height, 400.0);

    let back_to_logical = LogicalRect::from_physical(physical, WritingMode::VerticalRl, Direction::Ltr, container_size);
    assert_eq!(back_to_logical, logical);
}

#[test]
fn test_vertical_lr_conversion() {
    let container_size: Size2D<f32, CssPixel> = size2(1000.0, 800.0);
    let logical = LogicalRect::new(15.0, 25.0, 400.0, 200.0);

    let physical = logical.to_physical(WritingMode::VerticalLr, Direction::Ltr, container_size);
    // Em Vertical-LR LTR:
    // x = block_start = 25
    // y = inline_start = 15
    // width = block_size = 200
    // height = inline_size = 400
    assert_eq!(physical.origin.x, 25.0);
    assert_eq!(physical.origin.y, 15.0);
    assert_eq!(physical.size.width, 200.0);
    assert_eq!(physical.size.height, 400.0);

    let back_to_logical = LogicalRect::from_physical(physical, WritingMode::VerticalLr, Direction::Ltr, container_size);
    assert_eq!(back_to_logical, logical);
}

#[test]
fn test_logical_sides_to_physical() {
    let sides: LogicalSides<f32> = LogicalSides::new(10.0, 20.0, 30.0, 40.0);

    let insets_htb_ltr: SideOffsets2D<f32, CssPixel> = sides.to_physical(WritingMode::HorizontalTb, Direction::Ltr);
    assert_eq!(insets_htb_ltr.top, 30.0); // block_start
    assert_eq!(insets_htb_ltr.right, 20.0); // inline_end
    assert_eq!(insets_htb_ltr.bottom, 40.0); // block_end
    assert_eq!(insets_htb_ltr.left, 10.0); // inline_start

    let insets_htb_rtl: SideOffsets2D<f32, CssPixel> = sides.to_physical(WritingMode::HorizontalTb, Direction::Rtl);
    assert_eq!(insets_htb_rtl.left, 20.0); // inline_end vira left
    assert_eq!(insets_htb_rtl.right, 10.0); // inline_start vira right
}

#[test]
fn test_layout_unit_fixed_point_logical_rect() {
    let container_size: LayoutSize = Size2D::new(LayoutUnit::from_pixels(100), LayoutUnit::from_pixels(100));
    let logical = LogicalLayoutRect::new(
        LayoutUnit::from_pixels(10),
        LayoutUnit::from_pixels(20),
        LayoutUnit::from_pixels(50),
        LayoutUnit::from_pixels(30),
    );

    let physical: LayoutRect = logical.to_physical(WritingMode::HorizontalTb, Direction::Ltr, container_size);
    assert_eq!(physical.origin.x, LayoutUnit::from_pixels(10));
    assert_eq!(physical.origin.y, LayoutUnit::from_pixels(20));
    assert_eq!(physical.size.width, LayoutUnit::from_pixels(50));
    assert_eq!(physical.size.height, LayoutUnit::from_pixels(30));
}
