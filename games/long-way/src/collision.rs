//! Long Way's collision values. The collision engine is in `game_kit::collision`.

use bevy::prelude::*;
use game_kit::collision::Blocker;

/// Half the width of the player's body. The feet are blocked this far from a trunk's sides.
/// The Hunter's cloak is about 20 px wide and the hat 30. Keep this above 12.8 (half of the 25.6 gap between two rocks two cells apart), or those rocks stop forming one cluster.
pub const PLAYER_HALF_WIDTH: f32 = 14.0;
/// Distance from the centre of the player's canvas down to the feet (feet at y 120 of 128 in `Hunter.aseprite`).
pub const PLAYER_FEET_OFFSET: f32 = 56.0;
/// Height of the Hunter's body above the feet (hat top at about y 70 of 128 in `Hunter.aseprite`). The feet stop this far below the top of the map, so the body is not cut off where the camera stops.
pub const PLAYER_BODY_HEIGHT: f32 = 50.0;

/// How much of the tree's bottom the player may not enter when walking down from above, as a fraction of the tree height.
const BLOCKED_BOTTOM: f32 = 0.1;

/// A tree trunk the player cannot walk through. All values are in world coordinates.
#[derive(Clone, Copy)]
pub struct Trunk {
    pub left: f32,
    pub right: f32,
    /// World y of the bottom of the tree.
    pub base_y: f32,
    /// Height of the whole tree sprite.
    pub height: f32,
}

impl Trunk {
    /// Lowest feet y that is blocked: the base of the tree. The player walking up from below stops on this line, as it
    /// does at the bottom of a rock (`adr-lw13`).
    fn y_low(&self) -> f32 {
        self.base_y
    }

    /// Highest feet y that is blocked. Above this the player is behind the tree.
    fn y_high(&self) -> f32 {
        self.base_y + BLOCKED_BOTTOM * self.height
    }

    /// The area the player's feet may not enter, as a rectangle.
    fn blocked_area(&self) -> Rect {
        Rect::new(
            self.left - PLAYER_HALF_WIDTH,
            self.y_low(),
            self.right + PLAYER_HALF_WIDTH,
            self.y_high(),
        )
    }

    /// The component that registers this trunk with the collision world.
    pub fn blocker(&self) -> Blocker {
        Blocker(self.blocked_area())
    }
}
