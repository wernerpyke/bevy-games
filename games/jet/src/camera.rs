use bevy::prelude::*;

/// Height of the game region in art pixels: the map, 32 cells (`adr-jt2`).
pub const VIEW_HEIGHT: f32 = 512.0;

/// Height of the HUD bar above the game region, in art pixels (`adr-jt2`).
pub const HUD_BAR_HEIGHT: f32 = 28.0;

/// Size of the window in logical pixels: 2x on a 1080p screen at scale factor 2 (`adr-jt2`).
pub const WINDOW_WIDTH: u32 = 960;
pub const WINDOW_HEIGHT: u32 = 540;

/// Size of one grid cell in art pixels: an 8 px Pyxel tile at 2x (`adr-jt2`).
pub const CELL_SIZE: f32 = 16.0;

/// World position of the bottom left corner of map cell (0, 0). The camera is fixed at the world origin, so the
/// 60 x 32 map is centred on it.
pub const MAP_ORIGIN: Vec2 = Vec2::new(-480.0, -256.0);

/// Black ground: the sprites are white line art on black (`adr-jt5`).
pub const CLEAR_COLOR: Color = Color::BLACK;

/// The F3 grid: palette teal-green, so it stands out from the white art (`art-tags.md`).
pub const GRID_COLOR: Color = Color::srgb_u8(0x19, 0x95, 0x9c);
