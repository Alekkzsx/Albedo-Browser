use crate::engine::DisplayItem;
use std::collections::HashMap;

/// Um contexto de empilhamento Z-Index (Stacking Context).
/// Cada Layer representa um painel contínuo de conteúdo que será rasterizado.
#[derive(Clone, Debug)]
pub struct Layer {
    pub id: usize,
    /// Caixa delimitadora original (viewport relative space para fixed layers, scroll relative para base)
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Itens contidos nesta camada (Display List local)
    pub display_items: Vec<DisplayItem>,
    /// Flags comportamentais para o Compositor WGPU
    pub is_fixed: bool,
    pub z_index: i32,
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub display_items: Vec<DisplayItem>,
}

impl Layer {
    pub fn new(id: usize, is_fixed: bool, z_index: i32) -> Self {
        Self {
            id,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            display_items: Vec::new(),
            is_fixed,
            z_index,
        }
    }

    pub fn add_item(&mut self, item: DisplayItem) {
        if self.display_items.is_empty() {
            self.x = item.x;
            self.y = item.y;
            self.width = item.width;
            self.height = item.height;
        } else {
            let left = self.x.min(item.x);
            let top = self.y.min(item.y);
            let right = (self.x + self.width).max(item.x + item.width);
            let bottom = (self.y + self.height).max(item.y + item.height);

            self.x = left;
            self.y = top;
            self.width = right - left;
            self.height = bottom - top;
        }
        self.display_items.push(item);
    }

    /// Divide os itens da camada em tiles (padrão 512x512)
    pub fn build_tiles(&self, tile_size: i32) -> Vec<Tile> {
        let mut tiles: HashMap<(i32, i32), Tile> = HashMap::new();

        for item in &self.display_items {
            let start_col = (item.x as i32) / tile_size;
            let end_col = ((item.x + item.width) as i32) / tile_size;
            let start_row = (item.y as i32) / tile_size;
            let end_row = ((item.y + item.height) as i32) / tile_size;

            for col in start_col..=end_col {
                for row in start_row..=end_row {
                    let tile = tiles.entry((col, row)).or_insert_with(|| Tile {
                        x: col * tile_size,
                        y: row * tile_size,
                        width: tile_size,
                        height: tile_size,
                        display_items: Vec::new(),
                    });
                    tile.display_items.push(item.clone());
                }
            }
        }

        tiles.into_values().collect()
    }
}

pub struct LayerTree {
    pub base_layer: Layer,
    pub promoted_layers: Vec<Layer>,
    next_layer_id: usize,
}

impl LayerTree {
    pub fn new() -> Self {
        Self {
            base_layer: Layer::new(0, false, 0),
            promoted_layers: Vec::new(),
            next_layer_id: 1,
        }
    }

    pub fn build(items: Vec<DisplayItem>, fixed_nodes: &[usize]) -> Self {
        let mut tree = Self::new();

        // Contextos isolados
        let mut fixed_layer = Layer::new(tree.next_layer_id, true, 1000);
        tree.next_layer_id += 1;

        for item in items {
            if fixed_nodes.contains(&item.node_idx) {
                fixed_layer.add_item(item);
            } else {
                tree.base_layer.add_item(item);
            }
        }

        if !fixed_layer.display_items.is_empty() {
            tree.promoted_layers.push(fixed_layer);
        }

        tree
    }

    pub fn get_all_layers_sorted(&self) -> Vec<&Layer> {
        let mut sorted = vec![&self.base_layer];
        for layer in &self.promoted_layers {
            sorted.push(layer);
        }
        sorted.sort_by_key(|l| l.z_index);
        sorted
    }
}
