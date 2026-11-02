use bevy::prelude::*;

/// Height of the game view in art pixels: 15 cells (`adr-rt4`).
pub const VIEW_HEIGHT: f32 = 240.0;

/// Size of one grid cell in art pixels (`adr-rt3`).
pub const CELL_SIZE: f32 = 16.0;

/// Colour behind the world. The camera stops at the map edge, so it only shows outside a window wider than the map.
pub const CLEAR_COLOR: Color = Color::srgb(0.07, 0.09, 0.07);
