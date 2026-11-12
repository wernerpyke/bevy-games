use bevy::prelude::*;

/// Height of the game view in art pixels: 8 cells. The game region of the window is square, so the view is 8 by 8 cells. The Hunter is about 50 pixels tall, so about one 10th of the screen height. The camera follows the player and stops at the map edge.
pub const VIEW_HEIGHT: f32 = 512.0;

/// Size of the game window in logical pixels. The game region is the upper 576 by 576 (the view scale is 576 / `VIEW_HEIGHT` = 1.125), the HUD bar below it is 576 by 180 (`adr-lw24`).
pub const WINDOW_WIDTH: u32 = 576;
pub const WINDOW_HEIGHT: u32 = 756;

/// Height of the HUD bar as a share of the game view height: the bar is the view width wide and 5/16 as high (160 art pixels, 2.5 cells).
pub const HUD_BAR_RATIO: f32 = 0.3125;

/// Palette colour `#e78d7b`.
pub const HUD_TEXT: Color = Color::srgb(0.9059, 0.5529, 0.4824);

/// Background colour behind the world.
pub const CLEAR_COLOR: Color = Color::srgb(0.16, 0.17, 0.2);

/// Size of one grid cell in art pixels.
pub const CELL_SIZE: f32 = 64.0;

/// World position of the bottom left corner of cell (0, 0) for map files and the F2 grid. Fixed, so the cells
/// do not move when the window is resized. A map smaller than the view is centred by the camera, so the value only
/// has to stay the same between runs.
pub const MAP_ORIGIN: Vec2 = Vec2::new(-16.0 * CELL_SIZE, -9.0 * CELL_SIZE);

/// How slowly the camera moves onto a new map when the player crosses a seam, as a time constant in seconds: the
/// camera covers about two thirds of the way in this time, and most of it in three times this. Larger is slower
/// (`game_kit::map_transition`, `adr-x32`).
pub const CAMERA_EASE_SECS: f32 = 0.9;
