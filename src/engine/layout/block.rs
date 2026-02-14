use crate::engine::layout::types::*;

impl LayoutBox {
    pub fn layout_block(&mut self, containing_block: Dimensions) {
        self.calculate_block_width(containing_block);
        self.calculate_block_position(containing_block);
        self.layout_block_children();
        self.calculate_block_height();
    }

    pub fn calculate_block_width(&mut self, containing_block: Dimensions) {
        self.dimensions.content.width = containing_block.content.width;
    }

    pub fn calculate_block_position(&mut self, containing_block: Dimensions) {
        self.dimensions.content.x = containing_block.content.x;
        self.dimensions.content.y = containing_block.content.y + containing_block.content.height;
    }

    pub fn layout_block_children(&mut self) {
        let mut total_height = 0.0;
        for child in &mut self.children {
            child.layout(self.dimensions.clone());
            child.dimensions.content.y = self.dimensions.content.y + total_height;
            total_height += child.dimensions.content.height;
        }
        self.dimensions.content.height = total_height;
    }

    pub fn calculate_block_height(&mut self) {
        if self.dimensions.content.height == 0.0 {
             self.dimensions.content.height = 20.0;
        }
    }
}
