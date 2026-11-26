use bevy::prelude::*;

/// Height of the game region in art pixels. The HUD bar of 28 art pixels is above it (`adr-jt2`).
pub const VIEW_HEIGHT: f32 = 512.0;

/// Size of the window in logical pixels: 2x on a 1080p screen at scale factor 2 (`adr-jt2`).
pub const WINDOW_WIDTH: u32 = 960;
pub const WINDOW_HEIGHT: u32 = 540;

/// Black ground: the sprites are white line art on black (`adr-jt5`).
pub const CLEAR_COLOR: Color = Color::BLACK;
