//! Geometry for area effects: a growing ring, a band around a centre and a push away from a centre. Plain functions
//! on Bevy `Rect` and `Vec2`, no game values (`dev-docs/extensions/area-effects.md`).

use bevy::math::{Rect, Vec2};

/// The point of `rect` that is nearest to `p`. `p` itself when it is inside.
pub fn closest_point(rect: Rect, p: Vec2) -> Vec2 {
    p.clamp(rect.min, rect.max)
}

/// How far `p` is from the nearest point of `rect`. 0 when `p` is inside.
pub fn distance_to_rect(p: Vec2, rect: Rect) -> f32 {
    closest_point(rect, p).distance(p)
}

/// The unit direction from `centre` to the middle of `rect`: the way an effect at `centre` pushes the body. A body
/// whose middle is exactly on the centre is pushed along +X.
pub fn away_from(centre: Vec2, rect: Rect) -> Vec2 {
    (rect.center() - centre).try_normalize().unwrap_or(Vec2::X)
}

/// The distance from `p` to the corner of `rect` that is farthest away.
fn farthest_corner_distance(rect: Rect, p: Vec2) -> f32 {
    ((p - rect.center()).abs() + rect.half_size()).length()
}

/// Ease-out: 0 at 0, 1 at 1, fast at the start and slowing to a stop (quadratic, so the start speed is twice the
/// average). The input is clamped to 0 to 1.
pub fn ease_out(u: f32) -> f32 {
    let left = 1.0 - u.clamp(0.0, 1.0);
    1.0 - left * left
}

/// Radius of a growing ring `age` seconds after it started: eased from 0 to `max` over `grow_secs`, then constant.
/// A negative age (history before the start) gives 0. With `grow_secs` 0 the ring is at `max` from age 0.
pub fn ring_radius(age: f32, grow_secs: f32, max: f32) -> f32 {
    if grow_secs <= 0.0 {
        return if age < 0.0 { 0.0 } else { max };
    }
    max * ease_out(age / grow_secs)
}

/// True when a circle touches or overlaps the rectangle.
pub fn circle_hits_rect(centre: Vec2, radius: f32, rect: Rect) -> bool {
    closest_point(rect, centre).distance_squared(centre) <= radius * radius
}

/// True when the rectangle overlaps the band between two circles around `centre` (radius `inner` to `outer`).
/// The rectangle is convex, so it overlaps when its nearest point is inside the outer circle and its farthest corner
/// is outside the inner circle.
pub fn rect_overlaps_band(centre: Vec2, inner: f32, outer: f32, rect: Rect) -> bool {
    closest_point(rect, centre).distance(centre) <= outer
        && farthest_corner_distance(rect, centre) >= inner
}

/// How far a shield around `centre` pushes a body this frame: straight away from the centre at `speed`, while any
/// part of the body is inside `radius`. Nothing once it is clear. A body exactly on the centre is pushed along +X.
pub fn push_delta(centre: Vec2, radius: f32, rect: Rect, speed: f32, dt: f32) -> Vec2 {
    if !circle_hits_rect(centre, radius, rect) {
        return Vec2::ZERO;
    }
    away_from(centre, rect) * speed * dt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(centre: Vec2, size: Vec2) -> Rect {
        Rect::from_center_size(centre, size)
    }

    fn body() -> Rect {
        rect(Vec2::new(10.0, 0.0), Vec2::new(8.0, 8.0))
    }

    #[test]
    fn ease_out_is_fast_first_and_never_goes_back() {
        assert_eq!((ease_out(0.0), ease_out(1.0)), (0.0, 1.0));
        // The input is clamped.
        assert_eq!((ease_out(-3.0), ease_out(7.0)), (0.0, 1.0));
        assert_eq!(ease_out(0.5), 0.75);
        assert!(ease_out(0.1) > 0.15);
        let mut last = 0.0;
        for i in 0..=100 {
            let now = ease_out(i as f32 / 100.0);
            assert!(now >= last);
            last = now;
        }
        // The last tenth of the time covers only a sliver of the distance.
        assert!(1.0 - ease_out(0.9) < 0.02);
    }

    #[test]
    fn ring_radius_reaches_the_maximum_exactly_and_stays() {
        assert_eq!(ring_radius(0.0, 0.8, 230.0), 0.0);
        assert_eq!(ring_radius(0.8, 0.8, 230.0), 230.0);
        assert_eq!(ring_radius(5.0, 0.8, 230.0), 230.0);
        assert_eq!(ring_radius(-0.3, 0.8, 230.0), 0.0);
    }

    #[test]
    fn a_ring_with_no_grow_time_is_at_the_maximum_at_once() {
        assert_eq!(ring_radius(0.0, 0.0, 100.0), 100.0);
        assert_eq!(ring_radius(0.5, 0.0, 100.0), 100.0);
        assert_eq!(ring_radius(-0.1, 0.0, 100.0), 0.0);
    }

    #[test]
    fn the_trailing_band_leaves_no_gap_at_20_fps_when_it_reaches_back_more_than_a_frame() {
        // The band reaches back `trail` seconds. Consecutive frames must overlap, or a body could be skipped.
        // A frame at 20 fps is 0.05 s: a trail of 0.06 s overlaps, a trail of 0.04 s leaves a gap.
        let (grow, max, frame) = (1.0, 100.0, 0.05);
        let gaps = |trail: f32| {
            let mut found = 0;
            let mut age = 0.0;
            while age < grow {
                let next = age + frame;
                if ring_radius(next - trail, grow, max) > ring_radius(age, grow, max) {
                    found += 1;
                }
                age = next;
            }
            found
        };
        assert_eq!(gaps(0.06), 0);
        assert!(gaps(0.04) > 0);
    }

    #[test]
    fn circle_inside_hits() {
        assert!(circle_hits_rect(Vec2::new(10.0, 0.0), 1.0, body()));
    }

    #[test]
    fn circle_touching_an_edge_hits() {
        assert!(circle_hits_rect(Vec2::ZERO, 6.0, body()));
    }

    #[test]
    fn circle_short_of_an_edge_misses() {
        assert!(!circle_hits_rect(Vec2::ZERO, 5.9, body()));
    }

    #[test]
    fn corner_is_measured_as_a_distance() {
        // Corner at (14, 4). The point (17, 8) is 5 away from it.
        assert!(circle_hits_rect(Vec2::new(17.0, 8.0), 5.0, body()));
        assert!(!circle_hits_rect(Vec2::new(17.0, 8.0), 4.9, body()));
    }

    #[test]
    fn a_rectangle_in_the_hole_is_not_in_the_band() {
        // Farthest corner is at (14, 4), 14.6 from the centre, inside an inner radius of 20.
        assert!(!rect_overlaps_band(Vec2::ZERO, 20.0, 30.0, body()));
    }

    #[test]
    fn a_rectangle_across_the_band_is_in_it() {
        assert!(rect_overlaps_band(Vec2::ZERO, 8.0, 12.0, body()));
        assert!(rect_overlaps_band(Vec2::ZERO, 12.0, 30.0, body()));
    }

    #[test]
    fn a_rectangle_outside_the_band_is_not_in_it() {
        // Nearest point is at x = 6.
        assert!(!rect_overlaps_band(Vec2::ZERO, 1.0, 5.9, body()));
    }

    #[test]
    fn a_rectangle_that_spans_the_whole_band_is_in_it() {
        let wide = rect(Vec2::ZERO, Vec2::new(200.0, 8.0));
        assert!(rect_overlaps_band(Vec2::ZERO, 20.0, 30.0, wide));
    }

    #[test]
    fn the_shield_pushes_a_body_straight_away_until_it_is_clear() {
        let near = rect(Vec2::new(0.0, 100.0), Vec2::new(40.0, 48.0));
        let push = push_delta(Vec2::ZERO, 110.0, near, 200.0, 0.1);
        assert!(
            push.x.abs() < 1e-4 && (push.y - 20.0).abs() < 1e-4,
            "{push}"
        );
        // The nearest edge of this body is at y = 130, outside a radius of 110.
        let clear = rect(Vec2::new(0.0, 154.0), Vec2::new(40.0, 48.0));
        assert_eq!(push_delta(Vec2::ZERO, 110.0, clear, 200.0, 0.1), Vec2::ZERO);
    }

    #[test]
    fn the_distance_to_a_rectangle_is_to_its_nearest_point_and_zero_inside() {
        let rect = Rect::new(-3.0, 5.0, 9.0, 11.0);
        // Beside an edge, beyond a corner (3-4-5), and inside.
        assert_eq!(distance_to_rect(Vec2::new(14.0, 7.0), rect), 5.0);
        assert_eq!(distance_to_rect(Vec2::new(-6.0, 15.0), rect), 5.0);
        assert_eq!(distance_to_rect(Vec2::new(0.0, 8.0), rect), 0.0);
        assert_eq!(
            closest_point(rect, Vec2::new(-6.0, 15.0)),
            Vec2::new(-3.0, 11.0)
        );
    }

    #[test]
    fn away_points_from_the_centre_to_the_middle_of_the_body() {
        let body = rect(Vec2::new(-30.0, 40.0), Vec2::new(6.0, 20.0));
        let diagonal = away_from(Vec2::ZERO, body);
        assert!(diagonal.distance(Vec2::new(-0.6, 0.8)) < 1e-6, "{diagonal}");
        assert_eq!(away_from(Vec2::new(-30.0, 10.0), body), Vec2::Y);
        assert_eq!(away_from(Vec2::new(-30.0, 40.0), body), Vec2::X);
    }

    #[test]
    fn a_body_on_the_player_is_still_pushed_somewhere() {
        let on_top = rect(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert_ne!(
            push_delta(Vec2::ZERO, 110.0, on_top, 200.0, 0.1),
            Vec2::ZERO
        );
    }
}
