use bevy::prelude::*;

use crate::outline::{self, Cluster, Edge, Orientation, TOLERANCE};

/// How the forgiving slide behaves. Lengths are in world units, so they depend on the art scale of the game.
/// `for_cell_size` gives the values tuned for a cell size of 64 (Long Way), scaled to another cell size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlideTuning {
    /// Seconds the actor must keep pushing into an obstacle before it is slid past its edge.
    pub delay: f32,
    /// Speed of the slide past an obstacle edge, in world units per second.
    pub speed: f32,
    /// How close to an edge the actor must be to get slid past it, as a fraction of the length of the face it is
    /// pushing into.
    pub range_fraction: f32,
    /// The most the actor may be from an end of a face and still be slid past it. A long face of a cluster would
    /// otherwise slide it a long way. One cell. To be tuned by trying it.
    pub range_cap: f32,
    /// How far past a convex end of a face the way is probed (three cells). If it is open, the actor is put exactly
    /// on the end. If a face blocks it (a notch or an overhang), it is put `past_corner` past the end and walks on
    /// until that face stops it.
    pub clearance: f32,
    /// A slide towards an end whose way on is blocked stops this far past the end of the face, so the actor is off
    /// the face and can walk on until the next face blocks it. Ends at inward corners refuse the slide instead.
    pub past_corner: f32,
}

impl SlideTuning {
    pub const fn for_cell_size(cell_size: f32) -> Self {
        let cell = cell_size / 64.0;
        SlideTuning {
            delay: 0.2,
            speed: 120.0 * cell,
            range_fraction: 0.35,
            range_cap: 64.0 * cell,
            clearance: 192.0 * cell,
            past_corner: 0.5 * cell,
        }
    }
}

pub struct CollisionPlugin {
    /// When true, collision is forgiving: the actor slides along an obstacle when walking diagonally into it, and
    /// is slid past an edge when it keeps pushing straight into an obstacle close to that edge.
    /// When false, any blocked move stops the actor completely.
    pub allow_sliding: bool,
    /// Size of one grid cell in world units. The slide values scale with it (`SlideTuning::for_cell_size`).
    pub cell_size: f32,
}

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionWorld>()
            .insert_resource(CollisionSettings {
                allow_sliding: self.allow_sliding,
                slide: SlideTuning::for_cell_size(self.cell_size),
            })
            .add_systems(PostStartup, register_blockers);
    }
}

/// Every static blocked area the actor collides with, merged into clusters with one outline. Plugins add their blocked
/// areas at startup with `add_blocked` (rocks) and this plugin adds the trunks, so a trunk that overlaps a rock
/// joins the same cluster. `resolve` and `move_feet` take its `edges`.
#[derive(Resource, Default)]
pub struct CollisionWorld {
    areas: Vec<Rect>,
    /// Blocked areas that exist for a while, such as a wall made by a spell (`add_temporary`).
    temporary: Vec<(TemporaryBlocker, Rect)>,
    next_temporary: u32,
    /// `areas` then `temporary`, the list the clusters were built from. Area numbers index into it.
    built: Vec<Rect>,
    clusters: Vec<Cluster>,
    edges: Vec<Edge>,
}

/// Names one temporary blocked area, so it can be removed again (`CollisionWorld::remove_temporary`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporaryBlocker(u32);

impl CollisionWorld {
    /// Removes every blocked area, temporary ones too, for a game that reloads its map. Add the new areas with
    /// `add_blocked`.
    pub fn clear(&mut self) {
        self.areas.clear();
        self.temporary.clear();
        self.built.clear();
        self.clusters.clear();
        self.edges.clear();
    }

    /// Adds blocked areas and rebuilds every cluster and edge. Returns the number of the first added area, which
    /// is the number `is_interior` takes.
    pub fn add_blocked(&mut self, areas: &[Rect]) -> usize {
        let first = self.areas.len();
        self.areas.extend_from_slice(areas);
        self.rebuild();
        first
    }

    /// Adds a blocked area that is meant to go away again, and rebuilds every cluster and edge. Permanent area
    /// numbers do not change, because temporary areas come after them. A body that stands inside the area when
    /// it appears is not trapped: `resolve` lets a move that starts inside walk out.
    pub fn add_temporary(&mut self, area: Rect) -> TemporaryBlocker {
        let id = TemporaryBlocker(self.next_temporary);
        self.next_temporary += 1;
        self.temporary.push((id, area));
        self.rebuild();
        id
    }

    /// Removes a temporary area and rebuilds. Returns false when it is not there (already removed, or the world was
    /// cleared by a map reload), and then changes nothing.
    pub fn remove_temporary(&mut self, id: TemporaryBlocker) -> bool {
        let before = self.temporary.len();
        self.temporary.retain(|(other, _)| *other != id);
        let removed = self.temporary.len() != before;
        if removed {
            self.rebuild();
        }
        removed
    }

    fn rebuild(&mut self) {
        self.built.clear();
        self.built.extend_from_slice(&self.areas);
        self.built
            .extend(self.temporary.iter().map(|(_, area)| *area));
        self.clusters = outline::build(&self.built);
        self.edges = self
            .clusters
            .iter()
            .flat_map(|c| c.edges.iter().copied())
            .collect();
    }

    /// The areas that touch no empty space, for the debug overlay.
    pub fn interior_areas(&self) -> impl Iterator<Item = Rect> + '_ {
        self.clusters
            .iter()
            .flat_map(|c| c.interior.iter().map(|&i| self.built[i]))
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

/// A static area the actor's feet may not enter, as a world rectangle. The game grows it by the actor's size
/// before it adds this component. `register_blockers` adds every `Blocker` to the `CollisionWorld`.
#[derive(Component, Clone, Copy)]
pub struct Blocker(pub Rect);

/// Blockers are spawned in `Startup`, so they can be read after it. Runs before anything that reads the clusters.
pub fn register_blockers(blockers: Query<&Blocker>, mut world: ResMut<CollisionWorld>) {
    let areas: Vec<Rect> = blockers.iter().map(|blocker| blocker.0).collect();
    world.add_blocked(&areas);
}

#[derive(Resource)]
pub struct CollisionSettings {
    pub allow_sliding: bool,
    pub slide: SlideTuning,
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

/// Result of `resolve`: where the feet ended up, and the outline edge that stopped each axis, if any.
pub struct Resolved {
    pub feet: Vec2,
    pub blocked_x: Option<Edge>,
    pub blocked_y: Option<Edge>,
}

/// Moves the player's feet from `old` towards `new`, stopping at the first outline edge it crosses. A blocked region
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

/// How far a ray from `from` in the unit direction `dir` goes before it reaches an outline edge from the edge's free
/// side, at most `max`. It follows the rules of `resolve`: an edge stops only a ray that moves towards its blocked
/// side, and the ends of an edge count as in `Edge::covers`. A ray that starts on an edge and leaves it is not stopped.
/// For example the reach of a spell that stops at the first tree or rock.
pub fn ray_distance(from: Vec2, dir: Vec2, max: f32, edges: &[Edge]) -> f32 {
    let mut nearest = max;
    for edge in edges {
        let (start, step) = match edge.orientation {
            Orientation::Vertical => (from.x, dir.x),
            Orientation::Horizontal => (from.y, dir.y),
        };
        let towards_blocked = if edge.blocked_positive {
            step > 0.0
        } else {
            step < 0.0
        };
        if !towards_blocked {
            continue;
        }
        let distance = (edge.at - start) / step;
        if distance < 0.0 || distance >= nearest {
            continue;
        }
        let along = match edge.orientation {
            Orientation::Vertical => from.y + dir.y * distance,
            Orientation::Horizontal => from.x + dir.x * distance,
        };
        if edge.covers(along) {
            nearest = distance;
        }
    }
    nearest
}

/// Moves the player's feet by `step` for one frame of `dt` seconds, applying the sliding setting.
///
/// With `allow_sliding` off, a blocked move leaves the player where it was.
///
/// With it on:
/// 1. A diagonal move into a trunk slides along it (each axis is resolved separately).
/// 2. A move straight into a face stops the player, which shows the collision.
/// 3. If the player keeps pushing for `SlideTuning::delay` seconds and is close to an edge of that face, it is slid
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
    if assist.held < settings.slide.delay {
        return moved.feet;
    }
    match slide_to_nearest_edge(
        &settings.slide,
        face,
        moved.feet,
        straight_up_or_down,
        direction,
        dt,
        edges,
    ) {
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
/// smaller of `range_fraction` of the face and `range_cap`). When the way past the end is blocked
/// (a notch shallower than `clearance`), the player still slides to just past the end and walks on
/// until the next face blocks it.
/// `pushing_vertically` is true when the player pushes up or down, so the face is horizontal and it slides along x.
/// `push` is the push direction as signs.
fn slide_to_nearest_edge(
    tuning: &SlideTuning,
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
    let range = (tuning.range_fraction * (max - min)).min(tuning.range_cap);
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
    // Past an inward corner there is only solid ground, so the player is not slid there. Past a convex end that leads into
    // a notch, it still slides to just past the end and walks on until the next face stops it.
    if concave {
        return None;
    }
    let pocket = !clear_beyond(end_point, push * tuning.clearance, edges);

    let travel = tuning.speed * dt;
    let past = if pocket { tuning.past_corner } else { 0.0 };
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
    // The slide may not end inside another shape, for example one whose wall is closer than `past_corner`.
    Some(resolve(feet, target, edges).feet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;
    use crate::test_util::{DT, edges_of};

    /// The blocked feet area of the test obstacle: x -20 to 50, y -30 to 12.8.
    fn block() -> Rect {
        Rect::new(-20.0, -30.0, 50.0, 12.8)
    }

    /// The outline edges of the test obstacle.
    fn trunk_edges() -> Vec<Edge> {
        edges_of(&[block()])
    }

    fn walk(from: Vec2, to: Vec2) -> Vec2 {
        resolve(from, to, &trunk_edges()).feet
    }

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
        slide: SlideTuning::for_cell_size(64.0),
    };
    const NOT_SLIDING: CollisionSettings = CollisionSettings {
        allow_sliding: false,
        slide: SlideTuning::for_cell_size(64.0),
    };

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
    fn near_the_left_edge_player_stops_first_then_slides_past_after_the_delay() {
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
    fn in_the_middle_of_the_trunk_player_stays_stopped() {
        let end = simulate(&SLIDING, Vec2::new(15.0, -100.0), Vec2::new(0.0, STEP), 300);
        assert_eq!(end, Vec2::new(15.0, -30.0));
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
        let mut rng = Rng::new(12345);
        std::iter::from_fn(move || Some(rng.range(0.0155, 0.0175)))
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
                    held < SLIDING.slide.delay + 0.05,
                    "slide started after {held} s instead of about {} s",
                    SLIDING.slide.delay
                );
                assert!(
                    held >= SLIDING.slide.delay,
                    "slide started too early at {held} s"
                );
                return;
            }
            feet = next;
            pushed += dt;
        }
        panic!("never slid in {pushed} s");
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
    fn walking_along_a_floor_cannot_slip_into_the_shape_at_an_inward_corner() {
        // The same shape as above turned on its side: the face y 40 (x 0 to 20) ends at an inward corner at
        // (20, 40), where the vertical wall x 20 (y 40 to 80) starts. The player stopped on the line pushes right.
        let edges = edges_of(&[
            Rect::new(0.0, 40.0, 20.0, 100.0),
            Rect::new(20.0, 0.0, 60.0, 80.0),
        ]);
        let end = resolve(Vec2::new(5.0, 40.0), Vec2::new(100.0, 40.0), &edges).feet;
        assert_eq!(end, Vec2::new(20.0, 40.0));
    }

    #[test]
    fn rounding_noise_on_the_stop_line_does_not_let_player_into_a_shape() {
        let edges = edges_of(&[Rect::new(0.0, 0.0, 10.0, 10.0)]);
        // The player was stopped at y 0 below the rectangle, and the recomputed feet came out a hair high.
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
        // Inward corner: two rocks, the upper one shifted left. The wall x 40 (y 0 to 20) ends at an inward corner
        // at (40, 20), where the upper rock's bottom face (x 0 to 40, y 20) starts. Feet stopped on the wall line
        // push up, a hair past the end of that face, and still stop in the corner.
        let edges = edges_of(&[
            Rect::new(40.0, 0.0, 100.0, 20.0),
            Rect::new(0.0, 20.0, 80.0, 60.0),
        ]);
        let end = resolve(Vec2::new(40.0, 5.0), Vec2::new(40.000_005, 100.0), &edges).feet;
        assert_eq!(
            end,
            Vec2::new(40.0, 20.0),
            "stopped in the corner, not walked into the shape"
        );
    }

    #[test]
    fn feet_on_the_line_of_an_outward_corner_can_walk_past_it() {
        let edges = edges_of(&[Rect::new(0.0, 0.0, 10.0, 10.0)]);
        // Along the top line y 10, past the whole rectangle.
        let end = resolve(Vec2::new(-5.0, 10.0), Vec2::new(30.0, 10.0), &edges).feet;
        assert_eq!(end.x, 30.0);
    }

    #[test]
    fn the_notch_walls_and_floor_stop_player_inside_it() {
        let edges = notch();
        let from = Vec2::new(20.0, 25.0);
        assert_eq!(resolve(from, Vec2::new(-50.0, 25.0), &edges).feet.x, 10.0);
        assert_eq!(resolve(from, Vec2::new(90.0, 25.0), &edges).feet.x, 30.0);
        assert_eq!(resolve(from, Vec2::new(20.0, -50.0), &edges).feet.y, 10.0);
        // Straight up and out of the open top of the notch is free.
        assert_eq!(resolve(from, Vec2::new(20.0, 90.0), &edges).feet.y, 90.0);
    }

    #[test]
    fn a_cluster_stops_player_from_every_side_and_a_big_step_cannot_jump_it() {
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
    fn player_is_not_slid_into_the_seam_between_two_touching_rocks() {
        // Feet at x 50 are 10 from the seam at x 60, but the seam is not an edge. The nearest end is 50 away,
        // out of range, so the player stays stopped. Per rock it would have been slid to x 60 and walked up the seam.
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
        // 80 from the left end is inside 35% of the 400 long face, but beyond the cap: the player stays stopped.
        let end = simulate_edges(Vec2::new(80.0, -10.0), Vec2::new(0.0, STEP), 90, &edges);
        assert_eq!(end, Vec2::new(80.0, 0.0));
        // 50 from the end is inside the cap: slid to the end, then walks on up.
        let end = simulate_edges(Vec2::new(50.0, -10.0), Vec2::new(0.0, STEP), 90, &edges);
        assert_eq!(end.x, 0.0);
        assert!(end.y > 20.0, "walked up past the rock, got {end}");
    }

    #[test]
    fn player_slides_into_a_pocket_under_an_overhang_and_stops_at_its_back() {
        let edges = pocket();
        // Pushing right into the left face at y 100: the near end (76.8) leads into the pocket. It slides to just
        // below the overhang and walks on until the bottom rock stops it. The inward corner there never slides it.
        let end = simulate_edges(Vec2::new(300.0, 100.0), Vec2::new(STEP, 0.0), 120, &edges);
        assert_near(end, Vec2::new(440.8, 76.8 - SLIDING.slide.past_corner));
        // Pushing up into the bottom face near its left end: it slides left past the end, walks up the wall and is
        // stopped under the overhang.
        let end = simulate_edges(Vec2::new(460.0, -50.0), Vec2::new(0.0, STEP), 120, &edges);
        assert_near(end, Vec2::new(440.8 - SLIDING.slide.past_corner, 76.8));
    }

    /// The slide end points are sums of f32 coordinates near 600, which round by about 1e-4. The positions the
    /// slide tests tell apart differ by `past_corner` (0.5) or more.
    fn assert_near(end: Vec2, expected: Vec2) {
        assert!(end.distance(expected) < 1e-3, "{end} is not {expected}");
    }

    #[test]
    fn player_slides_into_a_shallow_notch_between_rocks() {
        // The right side of the test cluster: a bottom rock, a middle rock further left and an upper rock shifted
        // right. Under the upper rock is a notch 25.6 high and 128 deep, closed at the back by the middle rock.
        let edges = edges_of(&[
            Rect::new(440.8, 12.8, 647.2, 115.2),
            Rect::new(376.8, 76.8, 583.2, 179.2),
            Rect::new(583.2, 140.8, 711.2, 243.2),
        ]);
        // Pushing up under the bottom rock's right end: it slides right past it and walks up until the upper rock stops it.
        let end = simulate_edges(Vec2::new(630.0, -50.0), Vec2::new(0.0, STEP), 120, &edges);
        assert_near(end, Vec2::new(647.2 + SLIDING.slide.past_corner, 140.8));
        let end = simulate_edges(Vec2::new(900.0, 160.0), Vec2::new(-STEP, 0.0), 120, &edges);
        // Pushing left into the upper rock near its bottom end: it slides into the notch and walks to its back.
        assert_near(end, Vec2::new(583.2, 140.8 - SLIDING.slide.past_corner));
    }

    #[test]
    fn player_is_slid_past_a_free_corner_of_a_cluster() {
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
        // The L shape with the pillar on the left (inward end at the start of the base's top face, x 10) and its
        // mirror with the pillar on the right (inward end at the face's end, x 90).
        let mirrored = edges_of(&[
            Rect::new(0.0, 0.0, 100.0, 10.0),
            Rect::new(90.0, 10.0, 100.0, 50.0),
        ]);
        for (edges, x, at_start) in [(l_shape(), 20.0, true), (mirrored, 80.0, false)] {
            let face = resolve(Vec2::new(x, 30.0), Vec2::new(x, 0.0), &edges)
                .blocked_y
                .expect("the top of the base blocks a move down");
            assert_eq!((face.from_concave, face.to_concave), (at_start, !at_start));
            // 10 from the inward end and in range, but a slide there only runs into the pillar.
            let slid = slide_to_nearest_edge(
                &SLIDING.slide,
                face,
                Vec2::new(x, 10.0),
                true,
                Vec2::new(0.0, -1.0),
                DT,
                &edges,
            );
            assert_eq!(slid, None, "pillar end at the start: {at_start}");
        }
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
        assert!(assist.held > 0.16 && assist.held < SLIDING.slide.delay);
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
        // Under the base, near its left end. After sliding left past the base, the player walks up to the underside of a
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
        // Walking up the line between the first rectangle and the corner of the second stops just short of the corner.
        assert_eq!(end, Vec2::new(10.0, 10.0 - outline::FILLER_HALF.y));
    }

    #[test]
    fn feet_stopped_at_a_corner_contact_can_back_away_but_not_cross() {
        let edges = edges_of(&[
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(10.0, 10.0, 20.0, 20.0),
        ]);
        let stop = Vec2::new(10.0, 10.0 - outline::FILLER_HALF.y);
        // It can go back down and out to the right, the side it came from.
        assert_eq!(resolve(stop, Vec2::new(10.0, -5.0), &edges).feet.y, -5.0);
        assert_eq!(resolve(stop, Vec2::new(25.0, stop.y), &edges).feet.x, 25.0);
        // It cannot go up or left, into the other quadrant.
        assert_eq!(resolve(stop, Vec2::new(10.0, 30.0), &edges).feet, stop);
        assert_eq!(resolve(stop, Vec2::new(-5.0, stop.y), &edges).feet, stop);
    }

    #[test]
    fn a_diagonal_move_through_a_corner_contact_stops_on_the_side_it_came_from() {
        let edges = edges_of(&[
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(10.0, 10.0, 20.0, 20.0),
        ]);
        let from_below = resolve(Vec2::new(14.0, 6.0), Vec2::new(6.0, 14.0), &edges).feet;
        assert_eq!(from_below, Vec2::new(10.0, 10.0 - outline::FILLER_HALF.y));
        let from_above = resolve(Vec2::new(6.0, 14.0), Vec2::new(14.0, 6.0), &edges).feet;
        assert_eq!(from_above, Vec2::new(10.0, 10.0 + outline::FILLER_HALF.y));
    }

    #[test]
    fn a_slide_never_ends_inside_another_shape() {
        // The base's left end is at x 0. A second shape's wall is at x -0.2, closer than `past_corner`.
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

#[cfg(test)]
mod ray_tests {
    use super::*;

    /// A block x 100 to 110, y 0 to 200.
    fn world() -> CollisionWorld {
        let mut world = CollisionWorld::default();
        world.add_blocked(&[Rect::new(100.0, 0.0, 110.0, 200.0)]);
        world
    }

    #[test]
    fn a_ray_stops_at_the_face_it_meets_from_either_side() {
        let world = world();
        assert_eq!(
            ray_distance(Vec2::new(30.0, 50.0), Vec2::X, 500.0, world.edges()),
            70.0
        );
        assert_eq!(
            ray_distance(Vec2::new(170.0, 120.0), Vec2::NEG_X, 500.0, world.edges()),
            60.0
        );
        // From below, the bottom face (y 0) stops it.
        assert_eq!(
            ray_distance(Vec2::new(105.0, -40.0), Vec2::Y, 500.0, world.edges()),
            40.0
        );
    }

    #[test]
    fn a_slanted_ray_stops_where_it_crosses_the_face() {
        let world = world();
        let dir = Vec2::new(3.0, 4.0).normalize();
        // From (40, 20) the ray reaches x 100 after 60 / 0.6 = 100, at y 100.
        let distance = ray_distance(Vec2::new(40.0, 20.0), dir, 500.0, world.edges());
        assert!((distance - 100.0).abs() < 1e-3, "{distance}");
    }

    #[test]
    fn a_ray_that_misses_or_ends_short_or_leaves_a_face_goes_its_full_length() {
        let world = world();
        // Passes above the block.
        assert_eq!(
            ray_distance(Vec2::new(30.0, 250.0), Vec2::X, 500.0, world.edges()),
            500.0
        );
        // Ends before the block.
        assert_eq!(
            ray_distance(Vec2::new(30.0, 50.0), Vec2::X, 60.0, world.edges()),
            60.0
        );
        // Starts on the right face and moves away from it.
        assert_eq!(
            ray_distance(Vec2::new(110.0, 50.0), Vec2::X, 80.0, world.edges()),
            80.0
        );
        // Moves away from the block.
        assert_eq!(
            ray_distance(Vec2::new(30.0, 50.0), Vec2::NEG_X, 90.0, world.edges()),
            90.0
        );
    }

    #[test]
    fn a_ray_stops_at_the_nearer_of_two_blocks_in_either_order() {
        let near = Rect::new(60.0, 0.0, 70.0, 100.0);
        let far = Rect::new(150.0, 0.0, 160.0, 100.0);
        for areas in [[near, far], [far, near]] {
            let mut world = CollisionWorld::default();
            world.add_blocked(&areas);
            assert_eq!(
                ray_distance(Vec2::new(0.0, 50.0), Vec2::X, 500.0, world.edges()),
                60.0
            );
        }
    }
}

#[cfg(test)]
mod temporary_tests {
    use super::*;
    use crate::nav::NavGrid;
    use crate::test_util::map_rect;

    /// A tree trunk and a vertical wall (x 100 to 110, y 40 to 160) that is added and removed during a test.
    fn world_with_trunk() -> CollisionWorld {
        let mut world = CollisionWorld::default();
        world.add_blocked(&[Rect::new(20.0, 20.0, 30.0, 24.0)]);
        world
    }

    fn wall() -> Rect {
        Rect::new(100.0, 40.0, 110.0, 160.0)
    }

    fn walk_right(world: &CollisionWorld, from: Vec2, to_x: f32) -> Vec2 {
        resolve(from, Vec2::new(to_x, from.y), world.edges()).feet
    }

    #[test]
    fn a_body_stops_at_a_wall_that_appears_and_walks_on_when_it_is_removed() {
        let mut world = world_with_trunk();
        let from = Vec2::new(80.0, 100.0);
        assert_eq!(walk_right(&world, from, 140.0).x, 140.0);
        let id = world.add_temporary(wall());
        assert_eq!(walk_right(&world, from, 140.0).x, 100.0);
        assert!(world.remove_temporary(id));
        assert_eq!(walk_right(&world, from, 140.0).x, 140.0);
    }

    #[test]
    fn a_body_inside_a_new_wall_can_walk_out() {
        let mut world = world_with_trunk();
        world.add_temporary(wall());
        let inside = Vec2::new(105.0, 100.0);
        assert_eq!(walk_right(&world, inside, 130.0).x, 130.0);
    }

    /// Smoke test that a temporary wall reaches a user of `edges()`. Projectiles and movement call `resolve` on the
    /// same edges, which the walk tests above check.
    #[test]
    fn a_path_goes_round_a_wall_and_straight_again_when_it_is_removed() {
        let mut world = world_with_trunk();
        let id = world.add_temporary(wall());
        let (from, to) = (Vec2::new(50.0, 100.0), Vec2::new(150.0, 100.0));
        let grid = NavGrid::build(world.edges(), map_rect(), 10.0);
        let path = grid.find_path(from, to).expect("a way round");
        assert!(path.len() > 1, "{path:?}");
        assert!(path.iter().any(|p| p.y <= 40.0 || p.y >= 160.0), "{path:?}");
        world.remove_temporary(id);
        let grid = NavGrid::build(world.edges(), map_rect(), 10.0);
        assert_eq!(grid.find_path(from, to), Some(vec![to]));
    }

    #[test]
    fn a_wall_that_touches_a_trunk_joins_its_cluster_and_leaves_it_again() {
        let mut world = world_with_trunk();
        let clusters = world.clusters().len();
        let id = world.add_temporary(Rect::new(30.0, 20.0, 40.0, 60.0));
        assert_eq!(world.clusters().len(), clusters);
        world.remove_temporary(id);
        assert_eq!(world.clusters().len(), clusters);
        assert_eq!(world.edges().len(), 4);
    }

    #[test]
    fn removing_twice_or_after_a_clear_does_nothing() {
        let mut world = world_with_trunk();
        let id = world.add_temporary(wall());
        assert!(world.remove_temporary(id));
        assert!(!world.remove_temporary(id));
        let id = world.add_temporary(wall());
        world.clear();
        assert!(!world.remove_temporary(id));
        assert!(world.edges().is_empty());
    }

    #[test]
    fn two_walls_are_removed_one_at_a_time() {
        let mut world = world_with_trunk();
        let a = world.add_temporary(wall());
        let b = world.add_temporary(Rect::new(140.0, 40.0, 150.0, 160.0));
        world.remove_temporary(a);
        assert_eq!(walk_right(&world, Vec2::new(80.0, 100.0), 135.0).x, 135.0);
        assert_eq!(walk_right(&world, Vec2::new(120.0, 100.0), 160.0).x, 140.0);
        world.remove_temporary(b);
        assert_eq!(world.edges().len(), 4);
    }

    #[test]
    fn permanent_area_numbers_stay_when_a_wall_is_added_after_them() {
        let mut world = CollisionWorld::default();
        let first = world.add_blocked(&[Rect::new(0.0, 0.0, 10.0, 10.0)]);
        world.add_temporary(wall());
        let second = world.add_blocked(&[Rect::new(50.0, 50.0, 60.0, 60.0)]);
        assert_eq!((first, second), (0, 1));
        assert!(!world.is_interior(first));
    }

    #[test]
    fn a_temporary_area_can_be_an_interior_area() {
        // Eight permanent squares round a hole, which a temporary square fills. Its area number (8) is past the
        // permanent areas, so `interior_areas` must look it up among all the areas the clusters were built from.
        let mut world = CollisionWorld::default();
        let ring: Vec<Rect> = (0..9)
            .filter(|&i| i != 4)
            .map(|i| {
                let (x, y) = ((i % 3) as f32 * 10.0, (i / 3) as f32 * 10.0);
                Rect::new(x, y, x + 10.0, y + 10.0)
            })
            .collect();
        world.add_blocked(&ring);
        assert_eq!(world.interior_areas().count(), 0);
        let middle = Rect::new(10.0, 10.0, 20.0, 20.0);
        let id = world.add_temporary(middle);
        assert_eq!(world.interior_areas().collect::<Vec<_>>(), vec![middle]);
        world.remove_temporary(id);
        assert_eq!(world.interior_areas().count(), 0);
    }

    /// Run with `--ignored --nocapture` to read the cost of one rebuild with many blocked areas.
    #[test]
    #[ignore]
    fn cost_of_one_wall_with_many_areas() {
        let mut world = CollisionWorld::default();
        let areas: Vec<Rect> = (0..40)
            .flat_map(|x| {
                (0..25).map(move |y| {
                    let (x, y) = (x as f32 * 40.0, y as f32 * 40.0);
                    Rect::new(x, y, x + 12.0, y + 6.0)
                })
            })
            .collect();
        world.add_blocked(&areas);
        let bounds = Rect::new(0.0, 0.0, 1600.0, 1000.0);
        let started = std::time::Instant::now();
        let id = world.add_temporary(Rect::new(500.0, 300.0, 510.0, 420.0));
        let outline = started.elapsed();
        let started = std::time::Instant::now();
        let _ = NavGrid::build(world.edges(), bounds, 16.0);
        let nav = started.elapsed();
        world.remove_temporary(id);
        println!(
            "{} areas: outline rebuild {outline:?}, nav grid {nav:?}",
            areas.len()
        );
    }
}
