//! Game rules without Bevy, so they are unit-testable. Shared by the effect steps of the spike.

use bevy::math::Vec2;

/// Axis-aligned rectangle in art pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect2 {
    pub centre: Vec2,
    pub size: Vec2,
}

impl Rect2 {
    pub const fn new(centre: Vec2, size: Vec2) -> Self {
        Self { centre, size }
    }

    /// The distance from `p` to the corner of the rectangle that is farthest away.
    pub fn farthest_corner_distance(&self, p: Vec2) -> f32 {
        let half = self.size / 2.0;
        let d = (p - self.centre).abs() + half;
        d.length()
    }

    /// The point of the rectangle that is nearest to `p`.
    pub fn closest_point(&self, p: Vec2) -> Vec2 {
        let half = self.size / 2.0;
        p.clamp(self.centre - half, self.centre + half)
    }
}

/// True when a circle touches or overlaps the rectangle.
pub fn circle_hits_rect(centre: Vec2, radius: f32, rect: Rect2) -> bool {
    rect.closest_point(centre).distance_squared(centre) <= radius * radius
}

/// True when the rectangle overlaps the band between two circles around `centre` (radius `inner` to `outer`).
/// The rectangle is convex, so it overlaps when its nearest point is inside the outer circle and its farthest corner
/// is outside the inner circle.
pub fn rect_overlaps_band(centre: Vec2, inner: f32, outer: f32, rect: Rect2) -> bool {
    rect.closest_point(centre).distance(centre) <= outer
        && rect.farthest_corner_distance(centre) >= inner
}

/// Moves a circle from `from` to `to` in small steps and returns the first share of the move (0 to 1) at which
/// it touches the rectangle. The step is half the radius, so a fast circle cannot jump over a thin rectangle.
pub fn sweep_circle(from: Vec2, to: Vec2, radius: f32, rect: Rect2) -> Option<f32> {
    let step = (radius * 0.5).max(0.1);
    let steps = ((to - from).length() / step).ceil().max(1.0) as u32;
    (0..=steps)
        .map(|i| i as f32 / steps as f32)
        .find(|t| circle_hits_rect(from.lerp(to, *t), radius, rect))
}

/// The rectangle a moving circle hits first: its index in `rects` and the share of the move at the hit.
pub fn first_hit(from: Vec2, to: Vec2, radius: f32, rects: &[Rect2]) -> Option<(usize, f32)> {
    rects
        .iter()
        .enumerate()
        .filter_map(|(index, rect)| Some((index, sweep_circle(from, to, radius, *rect)?)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: Rect2 = Rect2::new(Vec2::new(10.0, 0.0), Vec2::new(8.0, 8.0));

    #[test]
    fn circle_inside_hits() {
        assert!(circle_hits_rect(Vec2::new(10.0, 0.0), 1.0, RECT));
    }

    #[test]
    fn circle_touching_an_edge_hits() {
        assert!(circle_hits_rect(Vec2::new(0.0, 0.0), 6.0, RECT));
    }

    #[test]
    fn circle_short_of_an_edge_misses() {
        assert!(!circle_hits_rect(Vec2::new(0.0, 0.0), 5.9, RECT));
    }

    #[test]
    fn corner_is_measured_as_a_distance() {
        // Corner at (14, 4). The point (17, 8) is 5 away from it.
        assert!(circle_hits_rect(Vec2::new(17.0, 8.0), 5.0, RECT));
        assert!(!circle_hits_rect(Vec2::new(17.0, 8.0), 4.9, RECT));
    }

    #[test]
    fn a_rectangle_in_the_hole_is_not_in_the_band() {
        // Farthest corner is at (14, 4), 14.6 from the centre, inside an inner radius of 20.
        assert!(!rect_overlaps_band(Vec2::ZERO, 20.0, 30.0, RECT));
    }

    #[test]
    fn a_rectangle_across_the_band_is_in_it() {
        assert!(rect_overlaps_band(Vec2::ZERO, 8.0, 12.0, RECT));
        assert!(rect_overlaps_band(Vec2::ZERO, 12.0, 30.0, RECT));
    }

    #[test]
    fn a_rectangle_outside_the_band_is_not_in_it() {
        // Nearest point is at x = 6.
        assert!(!rect_overlaps_band(Vec2::ZERO, 1.0, 5.9, RECT));
    }

    #[test]
    fn a_rectangle_that_spans_the_whole_band_is_in_it() {
        let wide = Rect2::new(Vec2::ZERO, Vec2::new(200.0, 8.0));
        assert!(rect_overlaps_band(Vec2::ZERO, 20.0, 30.0, wide));
    }

    #[test]
    fn a_long_step_cannot_jump_over_a_thin_rectangle() {
        let thin = Rect2::new(Vec2::new(50.0, 0.0), Vec2::new(2.0, 20.0));
        let t = sweep_circle(Vec2::ZERO, Vec2::new(100.0, 0.0), 4.0, thin).unwrap();
        // The circle touches when its edge reaches x = 49, so its centre is at 45.
        assert!((t * 100.0 - 45.0).abs() < 2.1, "t = {t}");
    }

    #[test]
    fn a_move_beside_the_rectangle_misses() {
        assert_eq!(
            sweep_circle(Vec2::new(0.0, 20.0), Vec2::new(100.0, 20.0), 4.0, RECT),
            None
        );
    }

    #[test]
    fn a_move_that_starts_inside_hits_at_once() {
        assert_eq!(
            sweep_circle(Vec2::new(10.0, 0.0), Vec2::new(30.0, 0.0), 4.0, RECT),
            Some(0.0)
        );
    }

    #[test]
    fn the_nearer_rectangle_is_hit_first() {
        let near = Rect2::new(Vec2::new(30.0, 0.0), Vec2::new(8.0, 8.0));
        let far = Rect2::new(Vec2::new(60.0, 0.0), Vec2::new(8.0, 8.0));
        let (index, _) = first_hit(Vec2::ZERO, Vec2::new(100.0, 0.0), 4.0, &[far, near]).unwrap();
        assert_eq!(index, 1);
    }
}
