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

/// True when two rectangles overlap (touching edges do not count).
pub fn rects_overlap(a: Rect2, b: Rect2) -> bool {
    let reach = (a.size + b.size) / 2.0;
    let gap = (a.centre - b.centre).abs();
    gap.x < reach.x && gap.y < reach.y
}

/// Moves `rect` by `step`, one axis at a time, so it slides along a blocker. An axis move is refused when it would
/// put the rectangle into a blocker that it does not overlap yet (a body that starts inside can walk out).
pub fn move_rect(rect: Rect2, step: Vec2, blockers: &[Rect2]) -> Vec2 {
    let mut centre = rect.centre;
    for axis in [Vec2::X, Vec2::Y] {
        let next = Rect2::new(centre + step * axis, rect.size);
        let blocked = blockers
            .iter()
            .any(|b| rects_overlap(next, *b) && !rects_overlap(Rect2::new(centre, rect.size), *b));
        if !blocked {
            centre = next.centre;
        }
    }
    centre
}

/// How far a shield around `centre` pushes a body this frame: straight away from the centre at `speed`, while any part of the
/// body is inside `radius`. Nothing once it is clear.
pub fn push_delta(centre: Vec2, radius: f32, rect: Rect2, speed: f32, dt: f32) -> Vec2 {
    if !circle_hits_rect(centre, radius, rect) {
        return Vec2::ZERO;
    }
    (rect.centre - centre).try_normalize().unwrap_or(Vec2::X) * speed * dt
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
    fn touching_rectangles_do_not_overlap() {
        let beside = Rect2::new(Vec2::new(18.0, 0.0), Vec2::new(8.0, 8.0));
        assert!(!rects_overlap(RECT, beside));
        assert!(rects_overlap(
            RECT,
            Rect2::new(Vec2::new(17.0, 0.0), Vec2::new(8.0, 8.0))
        ));
    }

    #[test]
    fn a_move_into_a_blocker_stops_and_slides() {
        let wall = Rect2::new(Vec2::new(0.0, 20.0), Vec2::new(100.0, 10.0));
        let body = Rect2::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        // Moving up into the wall is refused, moving sideways along it is not.
        let after = move_rect(body, Vec2::new(3.0, 12.0), &[wall]);
        assert_eq!(after, Vec2::new(3.0, 0.0));
    }

    #[test]
    fn a_body_inside_a_blocker_can_walk_out() {
        let wall = Rect2::new(Vec2::ZERO, Vec2::new(100.0, 100.0));
        let body = Rect2::new(Vec2::new(45.0, 0.0), Vec2::new(10.0, 10.0));
        assert_eq!(
            move_rect(body, Vec2::new(2.0, 0.0), &[wall]),
            Vec2::new(47.0, 0.0)
        );
    }

    #[test]
    fn the_shield_pushes_a_body_straight_away_until_it_is_clear() {
        let near = Rect2::new(Vec2::new(0.0, 100.0), Vec2::new(40.0, 48.0));
        let push = push_delta(Vec2::ZERO, 110.0, near, 200.0, 0.1);
        assert!(
            push.x.abs() < 1e-4 && (push.y - 20.0).abs() < 1e-4,
            "{push}"
        );
        // The nearest edge of this body is at y = 130, outside a radius of 110.
        let clear = Rect2::new(Vec2::new(0.0, 154.0), Vec2::new(40.0, 48.0));
        assert_eq!(push_delta(Vec2::ZERO, 110.0, clear, 200.0, 0.1), Vec2::ZERO);
    }

    #[test]
    fn a_body_on_the_player_is_still_pushed_somewhere() {
        let on_top = Rect2::new(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert_ne!(
            push_delta(Vec2::ZERO, 110.0, on_top, 200.0, 0.1),
            Vec2::ZERO
        );
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
