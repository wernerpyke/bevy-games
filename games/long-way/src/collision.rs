use bevy::math::Isometry2d;
use bevy::prelude::*;

use crate::depth::DepthSort;
use crate::outline::{self, Cluster, Edge, Orientation, TOLERANCE};

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
/// The most Guy may be from an end of a face and still be slid past it, in world units. A long face of a cluster
/// would otherwise slide him a long way. One cell. To be tuned by trying it.
const SLIDE_RANGE_CAP: f32 = 64.0;
/// How far past a convex end of a face the way is probed, in world units (three cells at a cell size of 64). If it
/// is open, Guy is put exactly on the end. If a face blocks it (a notch or an overhang), he is put
/// `SLIDE_PAST_CORNER` past the end and walks on until that face stops him.
const SLIDE_CLEARANCE: f32 = 192.0;
/// A slide towards an end whose way on is blocked stops this far past the end of the face, so Guy is off the face
/// and can walk on until the next face blocks him. Ends at inward corners refuse the slide instead.
const SLIDE_PAST_CORNER: f32 = 0.5;

pub struct CollisionPlugin {
    /// When true, collision is forgiving: Guy slides along an obstacle when walking diagonally into it, and
    /// is slid past an edge when he keeps pushing straight into an obstacle close to that edge.
    /// When false, any blocked move stops Guy completely.
    pub allow_sliding: bool,
}

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShowCollision>()
            .init_resource::<CollisionWorld>()
            .insert_resource(CollisionSettings {
                allow_sliding: self.allow_sliding,
            })
            .add_systems(PostStartup, register_trunks)
            .add_systems(Update, (toggle_debug, draw_debug));
    }
}

/// Every static blocked area Guy collides with, merged into clusters with one outline. Plugins add their blocked
/// areas at startup with `add_blocked` (rocks) and this plugin adds the trunks, so a trunk that overlaps a rock
/// joins the same cluster. `resolve` and `move_feet` take its `edges`.
#[derive(Resource, Default)]
pub struct CollisionWorld {
    areas: Vec<Rect>,
    clusters: Vec<Cluster>,
    edges: Vec<Edge>,
}

impl CollisionWorld {
    /// Adds blocked areas and rebuilds every cluster and edge. Returns the number of the first added area, which
    /// is the number `is_interior` takes.
    pub fn add_blocked(&mut self, areas: &[Rect]) -> usize {
        let first = self.areas.len();
        self.areas.extend_from_slice(areas);
        self.clusters = outline::build(&self.areas);
        self.edges = self
            .clusters
            .iter()
            .flat_map(|c| c.edges.iter().copied())
            .collect();
        first
    }

    pub fn clusters(&self) -> &[Cluster] {
        &self.clusters
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// True when the area touches no empty space (`Cluster::interior`).
    pub fn is_interior(&self, area: usize) -> bool {
        self.clusters.iter().any(|c| c.interior.contains(&area))
    }
}

/// Trunks are spawned in `Startup`, so they can be read after it. Runs before anything that reads the clusters.
pub fn register_trunks(trunks: Query<&Trunk>, mut world: ResMut<CollisionWorld>) {
    let areas: Vec<Rect> = trunks.iter().map(Trunk::blocked_area).collect();
    world.add_blocked(&areas);
}

#[derive(Resource)]
pub struct CollisionSettings {
    pub allow_sliding: bool,
}

/// Tracks how long an entity has pushed straight into an obstacle. Put it on every entity that uses `move_feet`.
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

/// Result of `resolve`: where the feet ended up, and the outline edge that stopped each axis, if any.
pub struct Resolved {
    pub feet: Vec2,
    pub blocked_x: Option<Edge>,
    pub blocked_y: Option<Edge>,
}

/// Moves Guy's feet from `old` towards `new`, stopping at the first outline edge it crosses. A blocked region
/// is on one side of each edge (`blocked_positive`), so an edge only stops a move coming from the free side. Each
/// axis is resolved on its own, and a move is stopped when it crosses an edge, so a large step cannot jump over a
/// shape. An edge covers the range between its ends, including an end at an inward corner (`Edge::covers`), so
/// feet on a corner line cannot slip into the shape.
pub fn resolve(old: Vec2, new: Vec2, edges: &[Edge]) -> Resolved {
    let mut x = new.x;
    let mut blocked_x = None;
    for edge in edges
        .iter()
        .filter(|e| e.orientation == Orientation::Vertical)
    {
        if edge.covers(old.y) && crosses(edge.blocked_positive, old.x, x, edge.at) {
            x = edge.at;
            blocked_x = Some(*edge);
        }
    }

    let mut y = new.y;
    let mut blocked_y = None;
    for edge in edges
        .iter()
        .filter(|e| e.orientation == Orientation::Horizontal)
    {
        if edge.covers(x) && crosses(edge.blocked_positive, old.y, y, edge.at) {
            y = edge.at;
            blocked_y = Some(*edge);
        }
    }

    Resolved {
        feet: Vec2::new(x, y),
        blocked_x,
        blocked_y,
    }
}

/// True when a move from `old` to `new` along one axis reaches an edge at `at` from its free side. A start
/// within `TOLERANCE` on the blocked side is still treated as on the free side: that is rounding noise after a
/// stop exactly on the edge.
fn crosses(blocked_positive: bool, old: f32, new: f32, at: f32) -> bool {
    if blocked_positive {
        old <= at + TOLERANCE && new > at
    } else {
        old >= at - TOLERANCE && new < at
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
pub fn move_feet(
    settings: &CollisionSettings,
    assist: &mut SlideAssist,
    old: Vec2,
    step: Vec2,
    dt: f32,
    edges: &[Edge],
) -> Vec2 {
    let desired = old + step;
    let moved = resolve(old, desired, edges);

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
    match slide_to_nearest_edge(face, moved.feet, straight_up_or_down, direction, dt, edges) {
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
                    "collision: held {:.2}s at feet {:?} but no usable edge in range of {:?}",
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

/// True when walking from `from` along `probe` is not stopped by any edge.
fn clear_beyond(from: Vec2, probe: Vec2, edges: &[Edge]) -> bool {
    let moved = resolve(from, from + probe, edges);
    (moved.feet - from).length() >= probe.length() - TOLERANCE
}

/// Moves `feet` along the face towards the nearer end. Returns `None` when the nearer end is out of range (the
/// smaller of `SLIDE_RANGE_FRACTION` of the face and `SLIDE_RANGE_CAP`). When the way past the end is blocked
/// (a notch shallower than `SLIDE_CLEARANCE`), Guy still slides to just past the end and walks on
/// until the next face blocks him.
/// `pushing_vertically` is true when Guy pushes up or down, so the face is horizontal and he slides along x.
/// `push` is the push direction as signs.
fn slide_to_nearest_edge(
    face: Edge,
    feet: Vec2,
    pushing_vertically: bool,
    push: Vec2,
    dt: f32,
    edges: &[Edge],
) -> Option<Vec2> {
    let position = if pushing_vertically { feet.x } else { feet.y };
    let (min, max) = (face.from, face.to);
    let to_min = position - min;
    let to_max = max - position;
    let range = (SLIDE_RANGE_FRACTION * (max - min)).min(SLIDE_RANGE_CAP);
    if to_min.min(to_max) > range {
        return None;
    }
    let towards_min = to_min <= to_max;
    let end = if towards_min { min } else { max };
    let concave = if towards_min {
        face.from_concave
    } else {
        face.to_concave
    };
    let end_point = if pushing_vertically {
        Vec2::new(end, feet.y)
    } else {
        Vec2::new(feet.x, end)
    };
    // Past an inward corner there is only solid ground, so Guy is not slid there. Past a convex end that leads into
    // a notch, he still slides to just past the end and walks on until the next face stops him.
    if concave {
        return None;
    }
    let pocket = !clear_beyond(end_point, push * SLIDE_CLEARANCE, edges);

    let travel = SLIDE_SPEED * dt;
    let past = if pocket { SLIDE_PAST_CORNER } else { 0.0 };
    let slid = if towards_min {
        (position - travel).max(min - past)
    } else {
        (position + travel).min(max + past)
    };
    let target = if pushing_vertically {
        Vec2::new(slid, feet.y)
    } else {
        Vec2::new(feet.x, slid)
    };
    // The slide may not end inside another shape, for example one whose wall is closer than `SLIDE_PAST_CORNER`.
    Some(resolve(feet, target, edges).feet)
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

    /// The outline edges of the test trunk, as the game builds them.
    fn trunk_edges() -> Vec<Edge> {
        outline::build(&[trunk().blocked_area()])
            .iter()
            .flat_map(|c| c.edges.iter().copied())
            .collect()
    }

    fn walk(from: Vec2, to: Vec2) -> Vec2 {
        resolve(from, to, &trunk_edges()).feet
    }

    const DT: f32 = 1.0 / 60.0;
    /// 300 world units per second at 60 frames per second.
    const STEP: f32 = 5.0;

    /// Walks `frames` frames with `step` per frame, returning the final feet position.
    fn simulate(settings: &CollisionSettings, start: Vec2, step: Vec2, frames: usize) -> Vec2 {
        let mut assist = SlideAssist::default();
        let mut feet = start;
        for _ in 0..frames {
            feet = move_feet(settings, &mut assist, feet, step, DT, &trunk_edges());
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
            &trunk_edges(),
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
            &trunk_edges(),
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
            feet = move_feet(&SLIDING, &mut assist, feet, up, DT, &trunk_edges());
        }
        assist.reset();
        for _ in 0..10 {
            feet = move_feet(&SLIDING, &mut assist, feet, up, DT, &trunk_edges());
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
            let next = move_feet(&SLIDING, &mut assist, feet, step, dt, &trunk_edges());
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

    fn edges_of(rects: &[Rect]) -> Vec<Edge> {
        outline::build(rects)
            .iter()
            .flat_map(|c| c.edges.iter().copied())
            .collect()
    }

    /// A U shape: base 0..40 by 0..10 and pillars 0..10 and 30..40 up to y 40. The notch is x 10..30 above y 10.
    fn notch() -> Vec<Edge> {
        edges_of(&[
            Rect::new(0.0, 0.0, 40.0, 10.0),
            Rect::new(0.0, 10.0, 10.0, 40.0),
            Rect::new(30.0, 10.0, 40.0, 40.0),
        ])
    }

    #[test]
    fn walking_along_a_wall_cannot_slip_into_the_shape_at_an_inward_corner() {
        // Two rocks, the upper one shifted left. The wall x 40 (y 0 to 20) ends at an inward corner at (40, 20),
        // where the upper rock's bottom face (x 0 to 40, y 20) starts. Guy stopped on the wall line pushes up.
        let edges = edges_of(&[
            Rect::new(40.0, 0.0, 100.0, 20.0),
            Rect::new(0.0, 20.0, 80.0, 60.0),
        ]);
        let end = resolve(Vec2::new(40.0, 5.0), Vec2::new(40.0, 100.0), &edges).feet;
        assert_eq!(
            end,
            Vec2::new(40.0, 20.0),
            "stopped in the corner, not walked into the shape"
        );
    }

    #[test]
    fn walking_along_a_floor_cannot_slip_into_the_shape_at_an_inward_corner() {
        // The same shape as above turned on its side: the face y 40 (x 0 to 20) ends at an inward corner at
        // (20, 40), where the vertical wall x 20 (y 40 to 80) starts. Guy stopped on the line pushes right.
        let edges = edges_of(&[
            Rect::new(0.0, 40.0, 20.0, 100.0),
            Rect::new(20.0, 0.0, 60.0, 80.0),
        ]);
        let end = resolve(Vec2::new(5.0, 40.0), Vec2::new(100.0, 40.0), &edges).feet;
        assert_eq!(end, Vec2::new(20.0, 40.0));
    }

    #[test]
    fn rounding_noise_on_the_stop_line_does_not_let_guy_into_a_shape() {
        let edges = edges_of(&[Rect::new(0.0, 0.0, 10.0, 10.0)]);
        // Guy was stopped at y 0 below the rectangle, and the recomputed feet came out a hair high.
        let end = resolve(Vec2::new(5.0, 0.000_005), Vec2::new(5.0, 3.0), &edges).feet;
        assert_eq!(end.y, 0.0);
        let end = resolve(Vec2::new(-0.000_005, 5.0), Vec2::new(3.0, 5.0), &edges).feet;
        assert_eq!(end.x, 0.0);
    }

    #[test]
    fn rounding_noise_at_a_corner_line_does_not_catch_or_leak() {
        // Convex corner: feet a hair inside the end of a face still walk past it.
        let edges = edges_of(&[Rect::new(0.0, 0.0, 10.0, 10.0)]);
        let end = resolve(
            Vec2::new(-5.0, 0.000_005),
            Vec2::new(30.0, 0.000_005),
            &edges,
        )
        .feet;
        assert_eq!(end.x, 30.0);
        // Inward corner: feet a hair past the end still stop in the corner.
        let edges = edges_of(&[
            Rect::new(40.0, 0.0, 100.0, 20.0),
            Rect::new(0.0, 20.0, 80.0, 60.0),
        ]);
        let end = resolve(Vec2::new(40.0, 5.0), Vec2::new(40.000_005, 100.0), &edges).feet;
        assert_eq!(end.y, 20.0);
    }

    #[test]
    fn feet_on_the_line_of_an_outward_corner_can_walk_past_it() {
        let edges = edges_of(&[Rect::new(0.0, 0.0, 10.0, 10.0)]);
        // Along the top line y 10, past the whole rectangle.
        let end = resolve(Vec2::new(-5.0, 10.0), Vec2::new(30.0, 10.0), &edges).feet;
        assert_eq!(end.x, 30.0);
    }

    #[test]
    fn the_notch_walls_and_floor_stop_guy_inside_it() {
        let edges = notch();
        let from = Vec2::new(20.0, 25.0);
        assert_eq!(resolve(from, Vec2::new(-50.0, 25.0), &edges).feet.x, 10.0);
        assert_eq!(resolve(from, Vec2::new(90.0, 25.0), &edges).feet.x, 30.0);
        assert_eq!(resolve(from, Vec2::new(20.0, -50.0), &edges).feet.y, 10.0);
        // Straight up and out of the open top of the notch is free.
        assert_eq!(resolve(from, Vec2::new(20.0, 90.0), &edges).feet.y, 90.0);
    }

    #[test]
    fn a_cluster_stops_guy_from_every_side_and_a_big_step_cannot_jump_it() {
        let edges = notch();
        let at = |from: Vec2, to: Vec2| resolve(from, to, &edges).feet;
        assert_eq!(at(Vec2::new(-100.0, 20.0), Vec2::new(500.0, 20.0)).x, 0.0);
        assert_eq!(at(Vec2::new(500.0, 20.0), Vec2::new(-100.0, 20.0)).x, 40.0);
        assert_eq!(at(Vec2::new(20.0, -100.0), Vec2::new(20.0, 500.0)).y, 0.0);
        assert_eq!(at(Vec2::new(5.0, 100.0), Vec2::new(5.0, -100.0)).y, 40.0);
    }

    #[test]
    fn walking_along_the_side_of_a_cluster_is_free() {
        let edges = notch();
        // Feet exactly on the line of the left side, walking up past the whole cluster.
        let end = resolve(Vec2::new(0.0, -10.0), Vec2::new(0.0, 100.0), &edges).feet;
        assert_eq!(end, Vec2::new(0.0, 100.0));
    }

    /// Two touching rocks side by side. Their shared face is one merged edge 0 to 120 long.
    fn two_rocks() -> Vec<Edge> {
        edges_of(&[
            Rect::new(0.0, 0.0, 60.0, 20.0),
            Rect::new(60.0, 0.0, 120.0, 20.0),
        ])
    }

    fn simulate_edges(start: Vec2, step: Vec2, frames: usize, edges: &[Edge]) -> Vec2 {
        let mut assist = SlideAssist::default();
        let mut feet = start;
        for _ in 0..frames {
            feet = move_feet(&SLIDING, &mut assist, feet, step, DT, edges);
        }
        feet
    }

    #[test]
    fn sliding_goes_to_the_end_of_the_merged_face_not_the_end_of_one_rock() {
        // 35% of the 120 long face is 42. Feet at x 20 are 20 from the left end: slid to x 0, then walk on up.
        let end = simulate_edges(
            Vec2::new(20.0, -10.0),
            Vec2::new(0.0, STEP),
            60,
            &two_rocks(),
        );
        assert_eq!(end.x, 0.0);
        assert!(end.y > 20.0, "walked up past the rocks, got {end}");
    }

    #[test]
    fn guy_is_not_slid_into_the_seam_between_two_touching_rocks() {
        // Feet at x 50 are 10 from the seam at x 60, but the seam is not an edge. The nearest end is 50 away,
        // out of range, so Guy stays stopped. Per rock he would have been slid to x 60 and walked up the seam.
        let end = simulate_edges(
            Vec2::new(50.0, -10.0),
            Vec2::new(0.0, STEP),
            90,
            &two_rocks(),
        );
        assert_eq!(end, Vec2::new(50.0, 0.0));
    }

    /// Two rocks as in the test map: a bottom one and one above it shifted left, so there is a pocket under the
    /// upper rock between x 376.8 and 440.8 (64 wide, bounded on the right by the bottom rock).
    fn pocket() -> Vec<Edge> {
        edges_of(&[
            Rect::new(440.8, 12.8, 647.2, 115.2),
            Rect::new(376.8, 76.8, 583.2, 179.2),
        ])
    }

    #[test]
    fn the_slide_range_is_capped_on_a_long_face() {
        let edges = edges_of(&[Rect::new(0.0, 0.0, 400.0, 20.0)]);
        // 80 from the left end is inside 35% of the 400 long face, but beyond the cap: Guy stays stopped.
        let end = simulate_edges(Vec2::new(80.0, -10.0), Vec2::new(0.0, STEP), 90, &edges);
        assert_eq!(end, Vec2::new(80.0, 0.0));
        // 50 from the end is inside the cap: slid to the end, then walks on up.
        let end = simulate_edges(Vec2::new(50.0, -10.0), Vec2::new(0.0, STEP), 90, &edges);
        assert_eq!(end.x, 0.0);
        assert!(end.y > 20.0, "walked up past the rock, got {end}");
    }

    #[test]
    fn guy_slides_into_a_pocket_under_an_overhang_and_stops_at_its_back() {
        let edges = pocket();
        // Pushing right into the left face at y 100: the near end (76.8) leads into the pocket. He slides to just
        // below the overhang and walks on until the bottom rock stops him. The inward corner there never slides him.
        let end = simulate_edges(Vec2::new(300.0, 100.0), Vec2::new(STEP, 0.0), 120, &edges);
        assert_eq!(end, Vec2::new(440.8, 76.3));
        // Pushing up into the bottom face near its left end: he slides left past the end, walks up the wall and is
        // stopped under the overhang.
        let end = simulate_edges(Vec2::new(460.0, -50.0), Vec2::new(0.0, STEP), 120, &edges);
        assert_eq!(end, Vec2::new(440.3, 76.8));
    }

    #[test]
    fn guy_slides_into_a_shallow_notch_between_rocks() {
        // The right side of the test cluster: a bottom rock, a middle rock further left and an upper rock shifted
        // right. Under the upper rock is a notch 25.6 high and 128 deep, closed at the back by the middle rock.
        let edges = edges_of(&[
            Rect::new(440.8, 12.8, 647.2, 115.2),
            Rect::new(376.8, 76.8, 583.2, 179.2),
            Rect::new(583.2, 140.8, 711.2, 243.2),
        ]);
        // Pushing up under the bottom rock's right end: he slides right past it and walks up until the upper rock stops him.
        let end = simulate_edges(Vec2::new(630.0, -50.0), Vec2::new(0.0, STEP), 120, &edges);
        assert_eq!(end, Vec2::new(647.7, 140.8));
        let end = simulate_edges(Vec2::new(900.0, 160.0), Vec2::new(-STEP, 0.0), 120, &edges);
        // Pushing left into the upper rock near its bottom end: he slides into the notch and walks to its back.
        assert_eq!(end, Vec2::new(583.2, 140.3));
    }

    #[test]
    fn guy_is_slid_past_a_free_corner_of_a_cluster() {
        let edges = edges_of(&[Rect::new(440.8, 12.8, 647.2, 115.2)]);
        let end = simulate_edges(Vec2::new(630.0, -50.0), Vec2::new(0.0, STEP), 120, &edges);
        assert_eq!(end.x, 647.2);
        assert!(end.y > 115.2, "walked up past the rock, got {end}");
    }

    #[test]
    fn clear_beyond_checks_the_path_past_the_end() {
        let edges = edges_of(&[Rect::new(0.0, 0.0, 10.0, 10.0)]);
        assert!(clear_beyond(
            Vec2::new(20.0, 5.0),
            Vec2::new(50.0, 0.0),
            &edges
        ));
        assert!(!clear_beyond(
            Vec2::new(-20.0, 5.0),
            Vec2::new(50.0, 0.0),
            &edges
        ));
        // Along the side line of the rectangle is free.
        assert!(clear_beyond(
            Vec2::new(10.0, -20.0),
            Vec2::new(0.0, 50.0),
            &edges
        ));
    }

    /// An L shape: a base 0..100 by 0..10 and a pillar 0..10 by 10..50. The inside corner is at (10, 10).
    fn l_shape() -> Vec<Edge> {
        edges_of(&[
            Rect::new(0.0, 0.0, 100.0, 10.0),
            Rect::new(0.0, 10.0, 10.0, 50.0),
        ])
    }

    #[test]
    fn an_end_at_an_inward_corner_refuses_the_slide() {
        let edges = l_shape();
        let face = resolve(Vec2::new(20.0, 30.0), Vec2::new(20.0, 0.0), &edges)
            .blocked_y
            .expect("the top of the base blocks a move down");
        assert!(face.from_concave);
        // 10 from the inward end and in range, but a slide there only runs into the pillar.
        let slid = slide_to_nearest_edge(
            face,
            Vec2::new(20.0, 10.0),
            true,
            Vec2::new(0.0, -1.0),
            DT,
            &edges,
        );
        assert_eq!(slid, None);
    }

    #[test]
    fn changing_the_push_direction_restarts_the_slide_timer() {
        let edges = l_shape();
        let mut assist = SlideAssist::default();
        let corner = Vec2::new(10.0, 10.0);
        // Pushing down into the base for 10 frames (0.17 s), just under the delay.
        for _ in 0..10 {
            move_feet(
                &SLIDING,
                &mut assist,
                corner,
                Vec2::new(0.0, -STEP),
                DT,
                &edges,
            );
        }
        assert!(assist.held > 0.16 && assist.held < SLIDE_DELAY);
        // Pushing left into the pillar, still blocked: the timer starts again from this frame.
        move_feet(
            &SLIDING,
            &mut assist,
            corner,
            Vec2::new(-STEP, 0.0),
            DT,
            &edges,
        );
        assert_eq!(assist.held, DT);
    }

    #[test]
    fn a_second_slide_can_follow_the_first() {
        // Under the base, near its left end. After sliding left past the base, Guy walks up to the underside of a
        // second rectangle and slides again, this time to its right end.
        let edges = edges_of(&[
            Rect::new(0.0, 0.0, 100.0, 10.0),
            Rect::new(-50.0, 30.0, 5.0, 60.0),
        ]);
        let end = simulate_edges(Vec2::new(10.0, -10.0), Vec2::new(0.0, STEP), 200, &edges);
        assert_eq!(end.x, 5.0);
        assert!(
            end.y > 60.0,
            "walked on past the second rectangle, got {end}"
        );
    }

    #[test]
    fn a_move_stops_at_the_first_of_two_edges_in_either_order() {
        let mut edges = edges_of(&[
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(0.0, 20.0, 10.0, 30.0),
        ]);
        for _ in 0..2 {
            let up = resolve(Vec2::new(5.0, -5.0), Vec2::new(5.0, 40.0), &edges).feet;
            assert_eq!(up, Vec2::new(5.0, 0.0));
            let down = resolve(Vec2::new(5.0, 50.0), Vec2::new(5.0, -5.0), &edges).feet;
            assert_eq!(down, Vec2::new(5.0, 30.0));
            edges.reverse();
        }
    }

    #[test]
    fn rectangles_touching_only_at_a_corner_leave_no_gap_to_walk_through() {
        let edges = edges_of(&[
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(10.0, 10.0, 20.0, 20.0),
        ]);
        let end = resolve(Vec2::new(10.0, -5.0), Vec2::new(10.0, 30.0), &edges).feet;
        // Walking up the line between the first rectangle and the corner of the second stops at that corner.
        assert_eq!(end, Vec2::new(10.0, 10.0));
    }

    #[test]
    fn a_slide_never_ends_inside_another_shape() {
        // The base's left end is at x 0. A second shape's wall is at x -0.2, closer than `SLIDE_PAST_CORNER`.
        let edges = edges_of(&[
            Rect::new(0.0, 0.0, 100.0, 10.0),
            Rect::new(-50.0, 10.0, -0.2, 60.0),
            Rect::new(-50.0, -60.0, -0.2, 0.0),
        ]);
        let end = simulate_edges(Vec2::new(10.0, -10.0), Vec2::new(0.0, STEP), 120, &edges);
        assert!(end.x >= -0.2, "ended inside the shape at {end}");
    }

    #[test]
    fn blocked_areas_added_in_separate_calls_merge_into_one_cluster() {
        let mut world = CollisionWorld::default();
        let first = world.add_blocked(&[Rect::new(0.0, 0.0, 100.0, 10.0)]);
        let second = world.add_blocked(&[Rect::new(90.0, 0.0, 120.0, 60.0)]);
        assert_eq!((first, second), (0, 1));
        assert_eq!(world.clusters().len(), 1);
        assert_eq!(world.edges().len(), 6, "an L shape has six edges");
        assert!(!world.is_interior(0));
    }
}
