use bevy::math::Isometry2d;
use bevy::prelude::*;

use crate::depth::DepthSort;

/// Half the width of Guy's body. Guy's feet are blocked this far from a trunk's sides.
pub const GUY_HALF_WIDTH: f32 = 20.0;
/// Height of Guy's 60x60 canvas.
pub const GUY_HEIGHT: f32 = 60.0;
/// Distance from the centre of Guy's canvas down to his feet.
pub const GUY_FEET_OFFSET: f32 = 30.0;

/// How far up the trunk Guy may overlap it when walking up from below, as a fraction of his height.
const OVERLAP_FROM_BELOW: f32 = 0.5;
/// How much of the tree's bottom Guy may not enter when walking down from above, as a fraction of the tree height.
const BLOCKED_BOTTOM: f32 = 0.1;

/// Seconds Guy must keep pushing into a trunk before he is slid past its edge.
const SLIDE_DELAY: f32 = 0.2;
/// Speed of the slide past a trunk edge, in world units per second.
const SLIDE_SPEED: f32 = 120.0;
/// How close to an edge Guy must be to get slid past it, as a fraction of the length of the face he is pushing into.
const SLIDE_RANGE_FRACTION: f32 = 0.35;

pub struct CollisionPlugin {
    /// When true, collision is forgiving: Guy slides along a trunk when walking diagonally into it, and
    /// is slid past an edge when he keeps pushing straight into a trunk close to that edge.
    /// When false, any blocked move stops Guy completely.
    pub allow_sliding: bool,
}

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShowCollision>()
            .insert_resource(CollisionSettings {
                allow_sliding: self.allow_sliding,
            })
            .add_systems(Update, (toggle_debug, draw_debug));
    }
}

#[derive(Resource)]
pub struct CollisionSettings {
    pub allow_sliding: bool,
}

/// Tracks how long an entity has pushed straight into a trunk. Put it on every entity that uses `move_feet`.
#[derive(Component, Default)]
pub struct SlideAssist {
    direction: Vec2,
    held: f32,
    /// Only used to log changes once instead of every frame.
    phase: Phase,
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
enum Phase {
    #[default]
    Free,
    Blocked,
    Sliding,
    /// Pushed for long enough, but too far from an edge to be slid.
    OutOfRange,
}

impl SlideAssist {
    /// Call when the entity is not trying to move.
    pub fn reset(&mut self) {
        if self.phase != Phase::Free {
            debug!("collision: no longer pushing (was {:?})", self.phase);
        }
        *self = Self::default();
    }
}

/// A tree trunk Guy cannot walk through. All values are in world coordinates.
#[derive(Component, Clone, Copy)]
pub struct Trunk {
    pub left: f32,
    pub right: f32,
    /// World y of the bottom of the tree.
    pub base_y: f32,
    /// Height of the whole tree sprite.
    pub height: f32,
}

impl Trunk {
    /// Lowest feet y that is blocked. Below this Guy is in front of the trunk and may walk up to here,
    /// which puts the top half of his body over the trunk base.
    fn y_low(&self) -> f32 {
        self.base_y - OVERLAP_FROM_BELOW * GUY_HEIGHT
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
}

/// Result of `resolve`: where the feet ended up, and the blocked area that stopped each axis, if any.
pub struct Resolved {
    pub feet: Vec2,
    pub blocked_x: Option<Rect>,
    pub blocked_y: Option<Rect>,
}

/// Moves Guy's feet from `old` towards `new`, stopping at the edge of any trunk's blocked area.
/// Each axis is resolved on its own. A move is stopped when it crosses an edge, so a large step
/// cannot jump over a trunk.
pub fn resolve<'a>(
    old: Vec2,
    new: Vec2,
    trunks: impl Iterator<Item = &'a Trunk> + Clone,
) -> Resolved {
    let mut x = new.x;
    let mut blocked_x = None;
    for trunk in trunks.clone() {
        let area = trunk.blocked_area();
        if old.y > area.min.y && old.y < area.max.y {
            if old.x <= area.min.x && x > area.min.x {
                x = area.min.x;
                blocked_x = Some(area);
            } else if old.x >= area.max.x && x < area.max.x {
                x = area.max.x;
                blocked_x = Some(area);
            }
        }
    }

    let mut y = new.y;
    let mut blocked_y = None;
    for trunk in trunks {
        let area = trunk.blocked_area();
        if x > area.min.x && x < area.max.x {
            if old.y <= area.min.y && y > area.min.y {
                y = area.min.y;
                blocked_y = Some(area);
            } else if old.y >= area.max.y && y < area.max.y {
                y = area.max.y;
                blocked_y = Some(area);
            }
        }
    }

    Resolved {
        feet: Vec2::new(x, y),
        blocked_x,
        blocked_y,
    }
}

/// Moves Guy's feet by `step` for one frame of `dt` seconds, applying the sliding setting.
///
/// With `allow_sliding` off, a blocked move leaves Guy where he was.
///
/// With it on:
/// 1. A diagonal move into a trunk slides along it (each axis is resolved separately).
/// 2. A move straight into a face stops Guy, which shows the collision.
/// 3. If Guy keeps pushing for `SLIDE_DELAY` seconds and is close to an edge of that face, he is slid
///    sideways to that edge, then continues past the trunk.
pub fn move_feet<'a>(
    settings: &CollisionSettings,
    assist: &mut SlideAssist,
    old: Vec2,
    step: Vec2,
    dt: f32,
    trunks: impl Iterator<Item = &'a Trunk> + Clone,
) -> Vec2 {
    let desired = old + step;
    let moved = resolve(old, desired, trunks);

    if !settings.allow_sliding {
        return if moved.feet == desired { desired } else { old };
    }

    let straight_up_or_down = step.x == 0.0 && step.y != 0.0;
    let straight_sideways = step.y == 0.0 && step.x != 0.0;
    let face = if straight_up_or_down {
        moved.blocked_y
    } else if straight_sideways {
        moved.blocked_x
    } else {
        None
    };
    let Some(face) = face else {
        assist.reset();
        return moved.feet;
    };

    let direction = push_direction(step);
    if assist.direction != direction {
        assist.direction = direction;
        assist.held = 0.0;
        assist.phase = Phase::Free;
    }
    if assist.phase == Phase::Free {
        assist.phase = Phase::Blocked;
        debug!(
            "collision: blocked at feet {:?} pushing {:?}",
            moved.feet, direction
        );
    }
    assist.held += dt;
    if assist.held < SLIDE_DELAY {
        return moved.feet;
    }
    match slide_to_nearest_edge(face, moved.feet, straight_up_or_down, dt) {
        Some(slid) => {
            if assist.phase != Phase::Sliding {
                assist.phase = Phase::Sliding;
                debug!("collision: sliding from feet {:?}", moved.feet);
            }
            slid
        }
        None => {
            if assist.phase != Phase::OutOfRange {
                assist.phase = Phase::OutOfRange;
                debug!(
                    "collision: held {:.2}s at feet {:?} but too far from an edge of {:?}",
                    assist.held, moved.feet, face
                );
            }
            moved.feet
        }
    }
}

/// The direction of a step as signs only, for example (0, 1) for up. Comparing exact normalised floats
/// would count tiny rounding differences between frames as a change of direction.
fn push_direction(step: Vec2) -> Vec2 {
    let sign = |value: f32| {
        if value > 0.0 {
            1.0
        } else if value < 0.0 {
            -1.0
        } else {
            0.0
        }
    };
    Vec2::new(sign(step.x), sign(step.y))
}

/// Moves `feet` along the face towards the nearer edge. Returns `None` if neither edge is within range.
/// `pushing_vertically` is true when Guy pushes up or down, so the face is horizontal and he slides along x.
fn slide_to_nearest_edge(
    face: Rect,
    feet: Vec2,
    pushing_vertically: bool,
    dt: f32,
) -> Option<Vec2> {
    let (position, min, max) = if pushing_vertically {
        (feet.x, face.min.x, face.max.x)
    } else {
        (feet.y, face.min.y, face.max.y)
    };
    let to_min = position - min;
    let to_max = max - position;
    if to_min.min(to_max) > SLIDE_RANGE_FRACTION * (max - min) {
        return None;
    }

    let travel = SLIDE_SPEED * dt;
    let slid = if to_min <= to_max {
        (position - travel).max(min)
    } else {
        (position + travel).min(max)
    };
    Some(if pushing_vertically {
        Vec2::new(slid, feet.y)
    } else {
        Vec2::new(feet.x, slid)
    })
}

/// Debug overlay switch. Press F1 to toggle.
#[derive(Resource, Default)]
struct ShowCollision(bool);

fn toggle_debug(input: Res<ButtonInput<KeyCode>>, mut show: ResMut<ShowCollision>) {
    if input.just_pressed(KeyCode::F1) {
        show.0 = !show.0;
    }
}

/// Red: where feet may not go. Green circle: the feet of every depth-sorted entity.
fn draw_debug(
    show: Res<ShowCollision>,
    trunks: Query<&Trunk>,
    feet: Query<(&Transform, &DepthSort)>,
    mut gizmos: Gizmos,
) {
    if !show.0 {
        return;
    }
    for trunk in &trunks {
        let area = trunk.blocked_area();
        gizmos.rect_2d(
            Isometry2d::from_translation(area.center()),
            area.size(),
            Color::srgb(1.0, 0.1, 0.1),
        );
    }
    for (transform, sort) in &feet {
        let point = Vec2::new(
            transform.translation.x,
            sort.feet_y(transform.translation.y),
        );
        gizmos.circle_2d(
            Isometry2d::from_translation(point),
            4.0,
            Color::srgb(0.1, 1.0, 0.1),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Base at y 0, tree 128 high, trunk x 0 to 30.
    // Blocked feet area: x -20 to 50, y -30 to 12.8.
    fn trunk() -> Trunk {
        Trunk {
            left: 0.0,
            right: 30.0,
            base_y: 0.0,
            height: 128.0,
        }
    }

    fn walk(from: Vec2, to: Vec2) -> Vec2 {
        resolve(from, to, [trunk()].iter()).feet
    }

    const DT: f32 = 1.0 / 60.0;
    /// 300 world units per second at 60 frames per second.
    const STEP: f32 = 5.0;

    /// Walks `frames` frames with `step` per frame, returning the final feet position.
    fn simulate(settings: &CollisionSettings, start: Vec2, step: Vec2, frames: usize) -> Vec2 {
        let mut assist = SlideAssist::default();
        let mut feet = start;
        for _ in 0..frames {
            feet = move_feet(settings, &mut assist, feet, step, DT, [trunk()].iter());
        }
        feet
    }

    const SLIDING: CollisionSettings = CollisionSettings {
        allow_sliding: true,
    };
    const NOT_SLIDING: CollisionSettings = CollisionSettings {
        allow_sliding: false,
    };

    #[test]
    fn stops_at_left_side() {
        assert_eq!(walk(Vec2::new(-100.0, 0.0), Vec2::new(0.0, 0.0)).x, -20.0);
    }

    #[test]
    fn stops_at_right_side() {
        assert_eq!(walk(Vec2::new(100.0, 0.0), Vec2::new(0.0, 0.0)).x, 50.0);
    }

    #[test]
    fn walking_up_overlaps_half_of_guy() {
        let end = walk(Vec2::new(10.0, -100.0), Vec2::new(10.0, 0.0));
        assert_eq!(end.y, -30.0);
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

    #[test]
    fn stops_below_the_trunk_when_walking_diagonally_up() {
        let end = walk(Vec2::new(-20.0, -40.0), Vec2::new(-10.0, -20.0));
        assert_eq!(end, Vec2::new(-10.0, -30.0));
    }

    #[test]
    fn slides_along_the_trunk() {
        let end = walk(Vec2::new(-20.0, -10.0), Vec2::new(-10.0, 5.0));
        assert_eq!(end, Vec2::new(-20.0, 5.0));
    }

    #[test]
    fn near_the_left_edge_guy_stops_first_then_slides_past_after_the_delay() {
        let start = Vec2::new(-10.0, -100.0);
        let up = Vec2::new(0.0, STEP);

        // About 0.17 s after reaching the trunk (14 frames to get there, then 10 frames of pushing):
        // still stopped, not yet sliding.
        let stopped = simulate(&SLIDING, start, up, 14 + 10);
        assert_eq!(stopped, Vec2::new(-10.0, -30.0));

        // Later: slid to the left edge and walked on past the trunk.
        let past = simulate(&SLIDING, start, up, 200);
        assert_eq!(past.x, -20.0);
        assert!(past.y > 12.8);
    }

    #[test]
    fn near_the_right_edge_guy_slides_to_the_right() {
        let past = simulate(&SLIDING, Vec2::new(40.0, -100.0), Vec2::new(0.0, STEP), 200);
        assert_eq!(past.x, 50.0);
        assert!(past.y > 12.8);
    }

    #[test]
    fn in_the_middle_of_the_trunk_guy_stays_stopped() {
        let end = simulate(&SLIDING, Vec2::new(15.0, -100.0), Vec2::new(0.0, STEP), 300);
        assert_eq!(end, Vec2::new(15.0, -30.0));
    }

    #[test]
    fn without_sliding_guy_stays_stopped_even_near_the_edge() {
        let end = simulate(
            &NOT_SLIDING,
            Vec2::new(-10.0, -100.0),
            Vec2::new(0.0, STEP),
            300,
        );
        assert_eq!(end, Vec2::new(-10.0, -30.0));
    }

    #[test]
    fn without_sliding_a_diagonal_move_into_a_trunk_stops_completely() {
        let mut assist = SlideAssist::default();
        let old = Vec2::new(-20.0, -10.0);
        let end = move_feet(
            &NOT_SLIDING,
            &mut assist,
            old,
            Vec2::new(5.0, 5.0),
            DT,
            [trunk()].iter(),
        );
        assert_eq!(end, old);
    }

    #[test]
    fn with_sliding_a_diagonal_move_into_a_trunk_slides_along_it() {
        let mut assist = SlideAssist::default();
        let end = move_feet(
            &SLIDING,
            &mut assist,
            Vec2::new(-20.0, -10.0),
            Vec2::new(5.0, 5.0),
            DT,
            [trunk()].iter(),
        );
        assert_eq!(end, Vec2::new(-20.0, -5.0));
    }

    #[test]
    fn letting_go_resets_the_delay() {
        let mut assist = SlideAssist::default();
        let up = Vec2::new(0.0, STEP);
        let mut feet = Vec2::new(-10.0, -30.0);
        // 10 frames is 0.17 s, under the delay. Letting go in between restarts the count.
        for _ in 0..10 {
            feet = move_feet(&SLIDING, &mut assist, feet, up, DT, [trunk()].iter());
        }
        assist.reset();
        for _ in 0..10 {
            feet = move_feet(&SLIDING, &mut assist, feet, up, DT, [trunk()].iter());
        }
        assert_eq!(feet, Vec2::new(-10.0, -30.0));
    }

    /// Frame times that vary a little from frame to frame, like a real display.
    fn frame_times() -> impl Iterator<Item = f32> {
        let mut seed: u32 = 12345;
        std::iter::from_fn(move || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            Some(0.0155 + (seed >> 16) as f32 / 65536.0 * 0.002)
        })
    }

    #[test]
    fn the_slide_delay_survives_frame_time_noise() {
        let mut assist = SlideAssist::default();
        let mut feet = Vec2::new(-10.0, -30.0);
        let mut pushed = 0.0;
        for dt in frame_times().take(300) {
            // The step length depends on the frame time, as it does in the game (300 units per second).
            let step = Vec2::new(0.0, 300.0 * dt);
            let next = move_feet(&SLIDING, &mut assist, feet, step, dt, [trunk()].iter());
            if next.x != feet.x {
                // The slide may start on the frame that brings the push time to the delay.
                let held = pushed + dt;
                assert!(
                    held < SLIDE_DELAY + 0.05,
                    "slide started after {held} s instead of about {SLIDE_DELAY} s"
                );
                assert!(held >= SLIDE_DELAY, "slide started too early at {held} s");
                return;
            }
            feet = next;
            pushed += dt;
        }
        panic!("never slid in {pushed} s");
    }
}
