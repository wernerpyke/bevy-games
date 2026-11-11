//! A rectangle on the ground plane that something can hit or touch. It is not the feet box that collision uses: feet
//! are small and sit at the bottom of a sprite, a hit box covers the body. Projectiles test it (`ProjectileTarget`),
//! and a game tests it for contact between two characters (`overlaps`).

use bevy::prelude::*;

/// The area of a body, relative to the entity's ground point (its `Transform` x and y).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HitBox {
    /// The centre of the box, relative to the ground point.
    pub offset: Vec2,
    /// Half the width and half the height.
    pub half_size: Vec2,
}

impl HitBox {
    /// The box in world coordinates for an entity whose ground point is `ground`.
    pub fn rect_at(&self, ground: Vec2) -> Rect {
        Rect::from_center_half_size(ground + self.offset, self.half_size)
    }
}

/// True when the two rectangles share some area. Rectangles that only touch along an edge do not overlap.
pub fn overlaps(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_needs_shared_area() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(overlaps(a, Rect::new(5.0, 5.0, 15.0, 15.0)));
        assert!(overlaps(a, Rect::new(2.0, 2.0, 4.0, 4.0)));
        assert!(!overlaps(a, Rect::new(10.0, 0.0, 20.0, 10.0)));
        assert!(!overlaps(a, Rect::new(0.0, 11.0, 10.0, 20.0)));
        assert!(!overlaps(a, Rect::new(-20.0, -20.0, -1.0, -1.0)));
    }
}
