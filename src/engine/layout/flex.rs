use crate::engine::layout::types::*;
use crate::engine::style::{FlexDirection, JustifyContent, AlignItems, DisplayMode};

pub fn layout_flex(lbox: &mut LayoutBox, _containing_block: Dimensions) {
    // 1. Calculate width (Block-like behavior for container)
    // We'll need access to calculate_block_width/calculate_block_position or replicate logically.
    // For now, let's assume those are helpers in mod.rs or replicated here if they are simple.
    
    // NOTE: In the original layout.rs, these were private methods on LayoutBox.
    // When moving this to a separate file, we might need them to be public or passed in.
    
    // 2. Layout Children
    let direction = lbox.style.flex_direction;
    let mut main_axis_pos = 0.0;
    let mut max_cross_size: f32 = 0.0;

    // Pass 1: Measure children
    for child in &mut lbox.children {
        child.layout(lbox.dimensions);
    }

    // Pass 2: Position children along main axis
    let total_main_size: f32 = lbox.children.iter().map(|c| {
        if direction == FlexDirection::Row { c.dimensions.content.width } else { c.dimensions.content.height }
    }).sum();

    let available_space = if direction == FlexDirection::Row { lbox.dimensions.content.width } else { lbox.dimensions.content.height }; 
    let remaining_space = available_space - total_main_size;
    
    let spacing = match lbox.style.justify_content {
        JustifyContent::FlexStart => 0.0,
        JustifyContent::Center => remaining_space / 2.0,
        JustifyContent::FlexEnd => remaining_space,
        JustifyContent::SpaceBetween => if lbox.children.len() > 1 { remaining_space / (lbox.children.len() as f32 - 1.0) } else { 0.0 },
        JustifyContent::SpaceAround => if !lbox.children.is_empty() { remaining_space / (lbox.children.len() as f32) } else { 0.0 },
    };
    
    main_axis_pos = match lbox.style.justify_content {
         JustifyContent::FlexEnd => remaining_space,
         JustifyContent::Center => remaining_space / 2.0,
         JustifyContent::SpaceAround => spacing / 2.0,
         _ => 0.0
    };
    
    let item_spacing = match lbox.style.justify_content {
         JustifyContent::SpaceBetween => spacing,
         JustifyContent::SpaceAround => spacing,
         _ => 0.0
    };

    for child in &mut lbox.children {
         let (child_main_size, child_cross_size) = if direction == FlexDirection::Row { 
             (child.dimensions.content.width, child.dimensions.content.height)
         } else {
             (child.dimensions.content.height, child.dimensions.content.width)
         };

         if direction == FlexDirection::Row {
             child.dimensions.content.x = lbox.dimensions.content.x + main_axis_pos;
             child.dimensions.content.y = lbox.dimensions.content.y; 
         } else {
             child.dimensions.content.x = lbox.dimensions.content.x;
             child.dimensions.content.y = lbox.dimensions.content.y + main_axis_pos;
         }

         if child_cross_size > max_cross_size {
             max_cross_size = child_cross_size;
         }

         main_axis_pos += child_main_size + item_spacing;
    }

    // 3. Calculate Height / Cross Size
    if direction == FlexDirection::Row {
        if lbox.dimensions.content.height == 0.0 {
            lbox.dimensions.content.height = max_cross_size;
        }
        
         for child in &mut lbox.children {
             match lbox.style.align_items {
                 AlignItems::Center => {
                     let child_h = child.dimensions.content.height;
                     let parent_h = lbox.dimensions.content.height;
                     child.dimensions.content.y = lbox.dimensions.content.y + (parent_h - child_h) / 2.0;
                 },
                 AlignItems::FlexEnd => {
                      let child_h = child.dimensions.content.height;
                     let parent_h = lbox.dimensions.content.height;
                     child.dimensions.content.y = lbox.dimensions.content.y + (parent_h - child_h);
                 },
                 AlignItems::Stretch => {
                     if child.style.display != DisplayMode::None {
                          child.dimensions.content.height = lbox.dimensions.content.height;
                     }
                 }
                  _ => {} 
             }
         }

    } else {
         if lbox.dimensions.content.height == 0.0 {
             lbox.dimensions.content.height = total_main_size; 
        }
          for child in &mut lbox.children {
             match lbox.style.align_items {
                 AlignItems::Center => {
                     let child_w = child.dimensions.content.width;
                     let parent_w = lbox.dimensions.content.width;
                     child.dimensions.content.x = lbox.dimensions.content.x + (parent_w - child_w) / 2.0;
                 },
                 AlignItems::FlexEnd => {
                      let child_w = child.dimensions.content.width;
                     let parent_w = lbox.dimensions.content.width;
                     child.dimensions.content.x = lbox.dimensions.content.x + (parent_w - child_w);
                 },
                  AlignItems::Stretch => {
                         child.dimensions.content.width = lbox.dimensions.content.width;
                 }
                  _ => {}
             }
         }
    }
}
