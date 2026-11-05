use bevy::prelude::*;

/// Height of the game view in art pixels: 9 cells. The Hunter is about 50 pixels tall, so about one 11th of the screen height. The camera follows the player and stops at the map edge.
pub const VIEW_HEIGHT: f32 = 576.0;

/// Background colour behind the world.
pub const CLEAR_COLOR: Color = Color::srgb(0.16, 0.17, 0.2);

/// Size of one grid cell in art pixels.
pub const CELL_SIZE: f32 = 64.0;

/// World position of the bottom left corner of cell (0, 0) for map files and the F2 grid. Fixed, so the cells
/// do not move when the window is resized. It is the corner of a 16:9 view (33.3 by 18.75 cells) rounded to
/// whole cells, so in a 16:9 window the grid starts close to the bottom left of the screen.
pub const MAP_ORIGIN: Vec2 = Vec2::new(-16.0 * CELL_SIZE, -9.0 * CELL_SIZE);

/// The map area in world units. Cell (0, 0) has its bottom left corner at `MAP_ORIGIN`.
pub fn map_rect(size: IVec2) -> Rect {
    Rect::from_corners(MAP_ORIGIN, MAP_ORIGIN + size.as_vec2() * CELL_SIZE)
}
