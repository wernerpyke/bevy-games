//! Long Way's collision values. The collision engine is in `game_kit::collision`.

use bevy::prelude::*;
use game_kit::collision::Blocker;

/// Half the width of Guy's body. Guy's feet are blocked this far from a trunk's sides.
pub const GUY_HALF_WIDTH: f32 = 20.0;
/// Distance from the centre of Guy's canvas down to his feet.
pub const GUY_FEET_OFFSET: f32 = 30.0;

/// How much of the tree's bottom Guy may not enter when walking down from above, as a fraction of the tree height.
const BLOCKED_BOTTOM: f32 = 0.1;

/// A tree trunk Guy cannot walk through. All values are in world coordinates.
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
    /// Lowest feet y that is blocked: the base of the tree. Guy walking up from below stops on this line, as he
    /// does at the bottom of a rock (`adr-lw13`).
    fn y_low(&self) -> f32 {
        self.base_y
    }

    /// Highest feet y that is blocked. Above this Guy is behind the tree.
    fn y_high(&self) -> f32 {
        self.base_y + BLOCKED_BOTTOM * self.height
    }

    /// The area Guy's feet may not enter, as a rectangle.
    fn blocked_area(&self) -> Rect {
        Rect::new(
            self.left - GUY_HALF_WIDTH,
            self.y_low(),
            self.right + GUY_HALF_WIDTH,
            self.y_high(),
        )
    }

    /// The component that registers this trunk with the collision world.
    pub fn blocker(&self) -> Blocker {
        Blocker(self.blocked_area())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_kit::collision::resolve;
    use game_kit::outline::{self, Edge};

    // Base at y 0, tree 128 high, trunk x 0 to 30.
    // Blocked feet area: x -20 to 50, y 0 to 12.8.
    fn trunk() -> Trunk {
        Trunk {
            left: 0.0,
            right: 30.0,
            base_y: 0.0,
            height: 128.0,
        }
    }

    fn trunk_edges() -> Vec<Edge> {
        outline::build(&[trunk().blocked_area()])
            .iter()
            .flat_map(|c| c.edges.iter().copied())
            .collect()
    }

    fn walk(from: Vec2, to: Vec2) -> Vec2 {
        resolve(from, to, &trunk_edges()).feet
    }

    #[test]
    fn stops_at_left_side() {
        assert_eq!(walk(Vec2::new(-100.0, 5.0), Vec2::new(0.0, 5.0)).x, -20.0);
    }

    #[test]
    fn stops_at_right_side() {
        assert_eq!(walk(Vec2::new(100.0, 5.0), Vec2::new(0.0, 5.0)).x, 50.0);
    }

    #[test]
    fn walking_up_stops_at_the_base() {
        let end = walk(Vec2::new(10.0, -100.0), Vec2::new(10.0, 10.0));
        assert_eq!(end.y, 0.0);
    }

    #[test]
    fn walking_down_stops_above_the_bottom_tenth() {
        let end = walk(Vec2::new(10.0, 100.0), Vec2::new(10.0, -50.0));
        assert_eq!(end.y, 12.8);
    }

    #[test]
    fn walks_behind_the_tree_above_the_blocked_band() {
        let end = walk(Vec2::new(-100.0, 40.0), Vec2::new(100.0, 40.0));
        assert_eq!(end.x, 100.0);
    }

    #[test]
    fn walks_past_the_trunk_when_not_overlapping_it() {
        let end = walk(Vec2::new(200.0, 100.0), Vec2::new(200.0, -100.0));
        assert_eq!(end.y, -100.0);
    }
}
