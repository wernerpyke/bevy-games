//! Projectiles that fly in a straight line over the ground and disappear when they hit something blocked or leave
//! the map. See `dev-docs/extensions/projectiles.md`.
//!
//! A projectile is a point on the ground, like the feet of a character, and uses the same collision edges
//! (`collision::segment_hit`). `step` and `PathShape` are plain Rust with no Bevy world, so they are unit tested without
//! a running app.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

use crate::area::ease_out;
use crate::camera::CameraBounds;
use crate::collision::{CollisionWorld, segment_hit};
use crate::depth::DepthSort;
use crate::hitbox::HitBox;
use crate::outline::Edge;

/// Moves every `Projectile`, removes the ones that hit something and sends `ProjectileHit`. Needs `CollisionPlugin`
/// (the blocked areas). The map rectangle is `CameraBounds`: without it a projectile never leaves the map.
pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ProjectileHit>()
            .add_systems(
                Update,
                (follow_paths, move_projectiles, orient_sprites)
                    .chain()
                    .in_set(ProjectileSet),
            )
            .add_systems(PostUpdate, despawn_spent);
    }
}

/// The kit's projectile systems. A system that reads `ProjectileHit` and queries the removed projectile (for example
/// to check its game marker) runs `.after(ProjectileSet)`: the projectile is despawned only in `PostUpdate`.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ProjectileSet;

/// A projectile in flight. The entity's `Transform` x and y are the point that collides: its ground point, with the
/// sprite on a child entity lifted above it, or with a `ProjectilePath` the point where it is drawn. It requires a
/// `DepthSort`, so it is drawn in the world layer by that point (`adr-x2`, `adr-x56`). The game spawns the entity and
/// its sprite, the kit only moves it.
#[derive(Component, Clone, Copy, Debug)]
#[require(DepthSort)]
pub struct Projectile {
    /// Ground velocity in world units per second.
    pub velocity: Vec2,
    /// How high above the ground point the projectile flies, as the sprite child is lifted. A `HitBox` covers a body
    /// above the feet of its owner, so a target is tested at this height. Blocked areas are tested on the ground.
    pub height: f32,
    /// False when the projectile hits no `ProjectileTarget` (`without_targets`).
    pub hits_targets: bool,
    /// A blocked area the projectile flies through (`ignoring`).
    pub ignored: Option<Rect>,
}

impl Projectile {
    /// A projectile that flies along `direction` (any length except zero) at `speed`.
    pub fn new(direction: Vec2, speed: f32) -> Self {
        Self {
            velocity: direction.normalize_or_zero() * speed,
            height: 0.0,
            hits_targets: true,
            ignored: None,
        }
    }

    /// A projectile that flies through the blocked area `area`, for example the base of the object that shoots it.
    /// It is still stopped by every other blocked area. An outline edge that lies on `area` (its border included) does
    /// not stop it.
    pub fn ignoring(mut self, area: Rect) -> Self {
        self.ignored = Some(area);
        self
    }

    /// The height at which the projectile flies and hits targets.
    pub fn at_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// A projectile that passes through every `ProjectileTarget`. Blocked areas and the map edge still stop it.
    pub fn without_targets(mut self) -> Self {
        self.hits_targets = false;
        self
    }
}

/// On a projectile that was removed this frame. It no longer moves, and `despawn_spent` despawns it in `PostUpdate`.
#[derive(Component)]
struct Spent;

/// The shape of a thrown path: the ground distance and the height above the ground over time. The ground distance
/// eases out, so the projectile slows down to a stop. In the air it falls in arcs under one gravity: a first arc from
/// `start_height` up to `peak` and down to the ground, then `bounces` arcs, each `bounce_ratio` as high as the one
/// before (so also shorter), then it rolls on the ground for the last `roll_share` of the time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathShape {
    /// Ground distance from the start to the end.
    pub distance: f32,
    /// Seconds from the start to the end.
    pub duration: f32,
    /// Height at the start, for a projectile thrown from above the ground.
    pub start_height: f32,
    /// Top of the first arc. A value below `start_height` counts as `start_height`.
    pub peak: f32,
    pub bounces: u32,
    /// Height of each bounce as a share of the arc before it, 0 to 1.
    pub bounce_ratio: f32,
    /// Share of `duration` at the end spent on the ground, 0 to 1.
    pub roll_share: f32,
}

impl PathShape {
    /// Ground distance flown after `t` seconds: 0 at the start, `distance` at the end and after.
    pub fn ground_distance(&self, t: f32) -> f32 {
        self.distance * ease_out(t / self.duration)
    }

    /// Height above the ground after `t` seconds.
    pub fn height(&self, t: f32) -> f32 {
        let air = self.duration * (1.0 - self.roll_share);
        let peak = self.peak.max(self.start_height);
        let rise = peak - self.start_height;
        let bounce_peaks = (1..=self.bounces).map(|n| peak * self.bounce_ratio.powi(n as i32));
        // An arc from rest at height h takes sqrt(2h/g) to fall, so the time of the path is `air` = k times the sum of
        // the square roots below, with k = sqrt(2/g). That gives the one gravity that fills `air`.
        let roots =
            rise.sqrt() + peak.sqrt() + bounce_peaks.clone().map(|p| 2.0 * p.sqrt()).sum::<f32>();
        if t <= 0.0 {
            return self.start_height;
        }
        if t >= air || roots <= 0.0 {
            return 0.0;
        }
        let k = air / roots;
        let gravity = 2.0 / (k * k);
        // Height after `tau` seconds of an arc that starts at `from` going up at `speed`.
        let arc = |from: f32, speed: f32, tau: f32| {
            (from + speed * tau - gravity * tau * tau / 2.0).max(0.0)
        };
        let first = k * (rise.sqrt() + peak.sqrt());
        if t < first {
            return arc(self.start_height, gravity * k * rise.sqrt(), t);
        }
        let mut start = first;
        for bounce in bounce_peaks {
            let length = 2.0 * k * bounce.sqrt();
            if t < start + length {
                return arc(0.0, gravity * k * bounce.sqrt(), t - start);
            }
            start += length;
        }
        0.0
    }
}

/// Moves a `Projectile` along a `PathShape` in `direction`. The entity's point is where the projectile is drawn and
/// collides: the ground position along `direction` plus the height, straight up the screen. Each frame the kit sets
/// the projectile's `velocity` so the point follows `ground_distance` and `height`. The kit keeps the `DepthSort`
/// `feet_offset` at the height above the ground, from the moment the path is added, so the projectile sorts by the
/// ground under it. At the end of the path the kit removes it (`HitCause::PathEnd`). Blocked areas and the map edge
/// stop it as any projectile.
#[derive(Component, Clone, Copy, Debug)]
#[require(Projectile = Projectile::new(Vec2::ZERO, 0.0))]
#[component(on_insert = sort_at_path_start)]
pub struct ProjectilePath {
    pub shape: PathShape,
    /// Unit direction over the ground.
    pub direction: Vec2,
    /// How far the point is above the ground when the height is 0, for example half the art height so that a ball
    /// rests on the ground.
    pub rest_height: f32,
    /// Seconds since the start.
    pub elapsed: f32,
}

impl ProjectilePath {
    pub fn new(shape: PathShape, direction: Vec2, rest_height: f32) -> Self {
        Self {
            shape,
            direction: direction.normalize_or_zero(),
            rest_height,
            elapsed: 0.0,
        }
    }

    /// How far the point is above the ground now.
    pub fn above_ground(&self) -> f32 {
        self.rest_height + self.shape.height(self.elapsed)
    }
}

/// Sets the sort of a new path projectile, so its first frame is drawn at the right depth.
fn sort_at_path_start(mut world: DeferredWorld, context: HookContext) {
    let Some(above) = world
        .get::<ProjectilePath>(context.entity)
        .map(ProjectilePath::above_ground)
    else {
        return;
    };
    if let Some(mut sort) = world.get_mut::<DepthSort>(context.entity) {
        sort.feet_offset = above;
    }
}

/// The move of a path point from `from` to `to` seconds: along the ground and up or down by the change of height.
fn path_move(path: &ProjectilePath, from: f32, to: f32) -> Vec2 {
    let shape = &path.shape;
    path.direction * (shape.ground_distance(to) - shape.ground_distance(from))
        + Vec2::Y * (shape.height(to) - shape.height(from))
}

/// The projectiles that follow a path, with their sort.
type PathQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static mut Projectile,
        &'static mut ProjectilePath,
        &'static mut DepthSort,
    ),
    Without<Spent>,
>;

fn follow_paths(
    time: Res<Time>,
    mut paths: PathQuery,
    mut hits: MessageWriter<ProjectileHit>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (entity, transform, mut projectile, mut path, mut sort) in &mut paths {
        if path.elapsed >= path.shape.duration {
            let position = transform.translation.truncate();
            debug!("projectile: removed at {position:?} (PathEnd)");
            hits.write(ProjectileHit {
                projectile: entity,
                position,
                cause: HitCause::PathEnd,
            });
            commands.entity(entity).try_insert(Spent);
            continue;
        }
        let next = (path.elapsed + dt).min(path.shape.duration);
        projectile.velocity = if dt > 0.0 {
            path_move(&path, path.elapsed, next) / dt
        } else {
            Vec2::ZERO
        };
        path.elapsed = next;
        // `move_projectiles` moves the point to where the path is at `next`.
        sort.feet_offset = path.above_ground();
    }
}

fn despawn_spent(mut commands: Commands, spent: Query<Entity, With<Spent>>) {
    for entity in &spent {
        commands.entity(entity).despawn();
    }
}

/// Marks an entity with a `HitBox` that projectiles hit. The projectile is removed and `ProjectileHit` carries the
/// entity in `HitCause::Target`. The kit does not remove the target: the game reads the message and decides.
#[derive(Component, Clone, Copy, Debug)]
pub struct ProjectileTarget;

/// Marks the sprite entity of a projectile, a child of the `Projectile` entity. The kit turns it to point along the
/// flight direction, so the art must be drawn pointing to the right (along +x). Only the sprite turns, not the
/// projectile entity, so a sprite lifted above the ground point stays at its height.
#[derive(Component, Clone, Copy, Debug)]
pub struct ProjectileSprite;

/// The rotation that turns art drawn pointing along +x to point along `velocity`. None when it is not moving.
fn sprite_rotation(velocity: Vec2) -> Option<Quat> {
    (velocity != Vec2::ZERO).then(|| Quat::from_rotation_z(velocity.to_angle()))
}

fn orient_sprites(
    mut sprites: Query<(&ChildOf, &mut Transform), With<ProjectileSprite>>,
    projectiles: Query<&Projectile>,
) {
    for (parent, mut transform) in &mut sprites {
        let Ok(projectile) = projectiles.get(parent.parent()) else {
            continue;
        };
        if let Some(rotation) = sprite_rotation(projectile.velocity) {
            transform.rotation = rotation;
        }
    }
}

/// Why a projectile was removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitCause {
    /// It reached the outline of a blocked area.
    Blocked,
    /// It reached the edge of the map.
    MapEdge,
    /// It reached the hit box of this entity (`ProjectileTarget`).
    Target(Entity),
    /// It reached the end of its `ProjectilePath`.
    PathEnd,
}

/// Sent when a projectile has been removed, for impact effects or damage. `position` is its point (see `Projectile`).
/// `projectile` is the removed entity: it still exists until `PostUpdate`, so a reader `.after(ProjectileSet)` can
/// check its components.
#[derive(Message, Clone, Copy, Debug)]
pub struct ProjectileHit {
    pub projectile: Entity,
    pub position: Vec2,
    pub cause: HitCause,
}

/// The entities that projectiles can hit.
type TargetQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Transform, &'static HitBox),
    (With<ProjectileTarget>, Without<Projectile>),
>;

fn move_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<CollisionWorld>,
    bounds: Option<Res<CameraBounds>>,
    mut projectiles: Query<(Entity, &mut Transform, &Projectile), Without<Spent>>,
    targets: TargetQuery,
    mut hits: MessageWriter<ProjectileHit>,
) {
    let boxes: Vec<(Entity, Rect)> = targets
        .iter()
        .map(|(entity, transform, hit_box)| {
            (entity, hit_box.rect_at(transform.translation.truncate()))
        })
        .collect();
    let rects: Vec<Rect> = boxes.iter().map(|(_, rect)| *rect).collect();
    let area = bounds.map_or(Rect::new(f32::MIN, f32::MIN, f32::MAX, f32::MAX), |b| b.0);
    for (entity, mut transform, projectile) in &mut projectiles {
        let old = transform.translation.truncate();
        let stepped = step_ignoring(
            old,
            projectile.velocity,
            time.delta_secs(),
            world.edges(),
            area,
            projectile.ignored,
        );
        let reached = match stepped {
            Step::Flying(point) | Step::HitBlocked(point) | Step::LeftMap(point) => point,
        };
        // A target counts only up to the point where the shot stopped, so a target behind a tree is safe.
        let lift = Vec2::new(0.0, projectile.height);
        let aimed: &[Rect] = if projectile.hits_targets { &rects } else { &[] };
        let cause = if let Some((index, point)) = target_hit(old + lift, reached + lift, aimed) {
            (point - lift, HitCause::Target(boxes[index].0))
        } else {
            match stepped {
                Step::Flying(point) => {
                    transform.translation.x = point.x;
                    transform.translation.y = point.y;
                    continue;
                }
                Step::HitBlocked(point) => (point, HitCause::Blocked),
                Step::LeftMap(point) => (point, HitCause::MapEdge),
            }
        };
        debug!("projectile: removed at {:?} ({:?})", cause.0, cause.1);
        hits.write(ProjectileHit {
            projectile: entity,
            position: cause.0,
            cause: cause.1,
        });
        commands.entity(entity).try_insert(Spent);
    }
}

/// The first of the `targets` that the segment from `old` to `new` enters: its index and the entry point. A segment
/// that starts inside a target hits it at `old`. The segment, not the end point, is tested, so a long step cannot
/// jump over a thin target. A segment that only touches an edge or a corner (a graze) does not enter, as in
/// `hitbox::overlaps`.
pub fn target_hit(old: Vec2, new: Vec2, targets: &[Rect]) -> Option<(usize, Vec2)> {
    let delta = new - old;
    let mut best: Option<(usize, f32)> = None;
    for (index, rect) in targets.iter().enumerate() {
        let Some(t) = entry_time(old, delta, *rect) else {
            continue;
        };
        if best.is_none_or(|(_, earliest)| t < earliest) {
            best = Some((index, t));
        }
    }
    best.map(|(index, t)| (index, old + delta * t))
}

/// When (0 to 1) the segment `start + delta * t` is first inside `rect`. The slab method on each axis. The edges
/// are not inside, so a segment along an edge or through a corner only is no entry.
fn entry_time(start: Vec2, delta: Vec2, rect: Rect) -> Option<f32> {
    let mut enter = 0.0f32;
    let mut leave = 1.0f32;
    for (from, change, low, high) in [
        (start.x, delta.x, rect.min.x, rect.max.x),
        (start.y, delta.y, rect.min.y, rect.max.y),
    ] {
        if change == 0.0 {
            if from <= low || from >= high {
                return None;
            }
            continue;
        }
        let (a, b) = ((low - from) / change, (high - from) / change);
        enter = enter.max(a.min(b));
        leave = leave.min(a.max(b));
    }
    (enter < leave).then_some(enter)
}

/// What one frame of flight did to a projectile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    /// Still flying, now at this ground point.
    Flying(Vec2),
    /// Reached the outline of a blocked area. The point is where it stopped, on the edge.
    HitBlocked(Vec2),
    /// Went past the edge of the map. The point is the last place inside it.
    LeftMap(Vec2),
}

/// Moves a projectile from `old` by `velocity * dt` and says what happened. `segment_hit` follows the straight line
/// of the move, so a long step cannot jump over a thin blocked band and a diagonal step cannot cut a corner. A blocked
/// move wins over leaving the map.
pub fn step(old: Vec2, velocity: Vec2, dt: f32, edges: &[Edge], bounds: Rect) -> Step {
    let new = old + velocity * dt;
    if let Some(hit) = segment_hit(old, new, edges) {
        return Step::HitBlocked(hit.point);
    }
    if !bounds.contains(new) {
        return Step::LeftMap(new.clamp(bounds.min, bounds.max));
    }
    Step::Flying(new)
}

/// `step`, but an edge that would stop the move at a point of `ignored` (its border included) does not stop it. The
/// move is resolved again without that edge, so an edge of the same outline outside `ignored` still stops it.
pub fn step_ignoring(
    old: Vec2,
    velocity: Vec2,
    dt: f32,
    edges: &[Edge],
    bounds: Rect,
    ignored: Option<Rect>,
) -> Step {
    let Some(area) = ignored else {
        return step(old, velocity, dt, edges, bounds);
    };
    let mut kept = edges.to_vec();
    loop {
        match segment_hit(old, old + velocity * dt, &kept) {
            Some(hit) if area.contains(hit.point) => kept.retain(|edge| *edge != hit.edge),
            _ => return step(old, velocity, dt, &kept, bounds),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{Collected, DT, collect, edges_of, map_rect, timed_app};

    /// Twice a walk speed of 80 px/s.
    const SPEED: f32 = 160.0;

    /// A blocked area, x 100 to 120, y 50 to 54: a 4 px band like the base of a lone tree.
    fn band_edges() -> Vec<Edge> {
        edges_of(&[Rect::new(100.0, 50.0, 120.0, 54.0)])
    }

    #[test]
    fn the_sprite_turns_from_plus_x_to_the_flight_direction_and_a_still_shot_keeps_its_turn() {
        // Art drawn along +x points up for a shot that flies up. 1e-5 is rounding of a quarter turn in f32.
        let up = sprite_rotation(Vec2::new(0.0, 5.0)).unwrap() * Vec3::X;
        assert!((up - Vec3::Y).length() < 1e-5, "{up}");
        assert_eq!(sprite_rotation(Vec2::ZERO), None);
    }

    fn target() -> Vec<Rect> {
        vec![Rect::new(100.0, 50.0, 110.0, 62.0)]
    }

    type Seen = Collected<ProjectileHit>;

    fn app() -> App {
        let mut app = timed_app(std::time::Duration::from_millis(16));
        app.init_resource::<CollisionWorld>()
            .init_resource::<Seen>()
            .add_plugins(ProjectilePlugin)
            .add_systems(Update, collect::<ProjectileHit>.after(ProjectileSet));
        app
    }

    #[test]
    fn a_shot_at_hand_height_hits_a_body_at_the_same_feet_height() {
        let body = HitBox {
            offset: Vec2::new(0.0, 6.0),
            half_size: Vec2::new(5.0, 4.0),
        };
        let mut app = app();
        let target = app
            .world_mut()
            .spawn((
                Transform::from_xyz(100.0, 50.0, 0.0),
                body,
                ProjectileTarget,
            ))
            .id();
        let low = app
            .world_mut()
            .spawn((
                Transform::from_xyz(60.0, 50.0, 0.0),
                Projectile::new(Vec2::X, 160.0).at_height(6.0),
            ))
            .id();
        let flat = app
            .world_mut()
            .spawn((
                Transform::from_xyz(60.0, 50.0, 0.0),
                Projectile::new(Vec2::X, 160.0),
            ))
            .id();
        for _ in 0..60 {
            app.update();
        }
        let seen = &app.world().resource::<Seen>().0;
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].cause, HitCause::Target(target));
        assert!(
            app.world().get_entity(low).is_err(),
            "the shot at body height is removed"
        );
        assert!(
            app.world().get_entity(flat).is_ok(),
            "the shot along the ground passes under the body"
        );
    }

    #[test]
    fn a_target_is_hit_from_each_side() {
        let cases = [
            (
                Vec2::new(90.0, 56.0),
                Vec2::new(105.0, 56.0),
                Vec2::new(100.0, 56.0),
            ),
            (
                Vec2::new(120.0, 56.0),
                Vec2::new(105.0, 56.0),
                Vec2::new(110.0, 56.0),
            ),
            (
                Vec2::new(105.0, 40.0),
                Vec2::new(105.0, 56.0),
                Vec2::new(105.0, 50.0),
            ),
            (
                Vec2::new(105.0, 70.0),
                Vec2::new(105.0, 56.0),
                Vec2::new(105.0, 62.0),
            ),
        ];
        for (old, new, expected) in cases {
            let (index, point) = target_hit(old, new, &target()).unwrap();
            assert_eq!(index, 0);
            assert!(point.distance(expected) < 1e-4, "{old:?}: {point:?}");
        }
    }

    #[test]
    fn a_diagonal_shot_hits_a_target() {
        // Enters through the corner (100, 50) and goes on inside.
        let (index, point) =
            target_hit(Vec2::new(90.0, 40.0), Vec2::new(110.0, 60.0), &target()).unwrap();
        assert_eq!(index, 0);
        assert!(point.distance(Vec2::new(100.0, 50.0)) < 1e-4, "{point:?}");
    }

    #[test]
    fn a_shot_that_only_grazes_an_edge_or_a_corner_misses() {
        // The target is x 100 to 110, y 50 to 62. Touching is not overlapping (`hitbox::overlaps`).
        let cases = [
            // Along the top edge, and along the left edge.
            (Vec2::new(90.0, 62.0), Vec2::new(120.0, 62.0)),
            (Vec2::new(100.0, 40.0), Vec2::new(100.0, 70.0)),
            // Through the top left corner only.
            (Vec2::new(90.0, 52.0), Vec2::new(110.0, 72.0)),
            // Ends on the left face, and starts on it moving away.
            (Vec2::new(90.0, 56.0), Vec2::new(100.0, 56.0)),
            (Vec2::new(100.0, 56.0), Vec2::new(90.0, 56.0)),
        ];
        for (old, new) in cases {
            assert_eq!(target_hit(old, new, &target()), None, "{old:?} to {new:?}");
        }
        // Just inside the top edge is a hit.
        let (index, point) =
            target_hit(Vec2::new(90.0, 61.9), Vec2::new(120.0, 61.9), &target()).unwrap();
        assert_eq!(index, 0);
        assert!(point.distance(Vec2::new(100.0, 61.9)) < 1e-4, "{point:?}");
    }

    #[test]
    fn a_long_step_cannot_jump_over_a_target() {
        let hit = target_hit(Vec2::new(0.0, 56.0), Vec2::new(300.0, 56.0), &target());
        assert_eq!(hit.map(|(_, point)| point.x), Some(100.0));
    }

    #[test]
    fn shots_beside_or_short_of_a_target_miss() {
        let miss = |old: Vec2, new: Vec2| target_hit(old, new, &target());
        assert_eq!(miss(Vec2::new(90.0, 70.0), Vec2::new(120.0, 70.0)), None);
        assert_eq!(miss(Vec2::new(90.0, 56.0), Vec2::new(99.0, 56.0)), None);
        assert_eq!(miss(Vec2::new(120.0, 56.0), Vec2::new(130.0, 56.0)), None);
        assert_eq!(miss(Vec2::new(90.0, 56.0), Vec2::new(90.0, 56.0)), None);
    }

    #[test]
    fn a_shot_that_starts_inside_hits_at_once() {
        let (_, point) =
            target_hit(Vec2::new(105.0, 56.0), Vec2::new(120.0, 56.0), &target()).unwrap();
        assert_eq!(point, Vec2::new(105.0, 56.0));
    }

    #[test]
    fn the_nearest_of_several_targets_is_hit() {
        let targets = vec![
            Rect::new(200.0, 50.0, 210.0, 62.0),
            Rect::new(100.0, 50.0, 110.0, 62.0),
        ];
        let (index, _) =
            target_hit(Vec2::new(0.0, 56.0), Vec2::new(300.0, 56.0), &targets).unwrap();
        assert_eq!(index, 1);
    }

    #[test]
    fn open_ground_moves_by_velocity_times_dt() {
        let result = step(
            Vec2::new(10.0, 10.0),
            Vec2::new(SPEED, 0.0),
            DT,
            &[],
            map_rect(),
        );
        let Step::Flying(point) = result else {
            panic!("expected Flying, got {result:?}");
        };
        assert!((point.x - (10.0 + SPEED * DT)).abs() < 1e-4);
        assert_eq!(point.y, 10.0);
    }

    #[test]
    fn straight_into_a_face_stops_on_the_edge() {
        // Flying right at y 52, the left face of the band is at x 100.
        let result = step(
            Vec2::new(99.0, 52.0),
            Vec2::new(SPEED, 0.0),
            DT,
            &band_edges(),
            map_rect(),
        );
        assert_eq!(result, Step::HitBlocked(Vec2::new(100.0, 52.0)));
    }

    #[test]
    fn upward_into_the_band_stops_on_its_bottom_edge() {
        let result = step(
            Vec2::new(110.0, 49.0),
            Vec2::new(0.0, SPEED),
            DT,
            &band_edges(),
            map_rect(),
        );
        assert_eq!(result, Step::HitBlocked(Vec2::new(110.0, 50.0)));
    }

    #[test]
    fn a_diagonal_step_stops_where_its_line_meets_the_band_even_across_a_corner() {
        // The band is x 100 to 120, y 50 to 54. Each step is 6 by 6 in one second. The corner cases pass only a
        // corner of the band, outside the path that moves along x first and then along y.
        let cases = [
            // Across the bottom right, bottom left, top right and top left corners.
            (
                Vec2::new(116.0, 48.0),
                Vec2::new(6.0, 6.0),
                Vec2::new(118.0, 50.0),
            ),
            (
                Vec2::new(104.0, 48.0),
                Vec2::new(-6.0, 6.0),
                Vec2::new(102.0, 50.0),
            ),
            (
                Vec2::new(116.0, 56.0),
                Vec2::new(6.0, -6.0),
                Vec2::new(118.0, 54.0),
            ),
            (
                Vec2::new(104.0, 56.0),
                Vec2::new(-6.0, -6.0),
                Vec2::new(102.0, 54.0),
            ),
            // Into the left face, stopped where the line meets it.
            (
                Vec2::new(97.0, 50.5),
                Vec2::new(6.0, 6.0),
                Vec2::new(100.0, 53.5),
            ),
        ];
        for (old, velocity, expected) in cases {
            assert_eq!(
                step(old, velocity, 1.0, &band_edges(), map_rect()),
                Step::HitBlocked(expected),
                "{old:?} by {velocity:?}"
            );
        }
    }

    #[test]
    fn a_path_beside_the_band_is_free() {
        let result = step(
            Vec2::new(130.0, 40.0),
            Vec2::new(0.0, SPEED),
            DT,
            &band_edges(),
            map_rect(),
        );
        assert!(matches!(result, Step::Flying(_)), "{result:?}");
    }

    #[test]
    fn a_path_above_the_band_is_free() {
        // Flying right along y 60, above the band.
        let result = step(
            Vec2::new(99.0, 60.0),
            Vec2::new(SPEED, 0.0),
            DT,
            &band_edges(),
            map_rect(),
        );
        assert!(matches!(result, Step::Flying(_)), "{result:?}");
    }

    #[test]
    fn leaving_each_side_of_the_map_is_detected() {
        let cases = [
            (
                Vec2::new(199.0, 100.0),
                Vec2::new(SPEED, 0.0),
                Vec2::new(200.0, 100.0),
            ),
            (
                Vec2::new(1.0, 100.0),
                Vec2::new(-SPEED, 0.0),
                Vec2::new(0.0, 100.0),
            ),
            (
                Vec2::new(100.0, 199.0),
                Vec2::new(0.0, SPEED),
                Vec2::new(100.0, 200.0),
            ),
            (
                Vec2::new(100.0, 1.0),
                Vec2::new(0.0, -SPEED),
                Vec2::new(100.0, 0.0),
            ),
        ];
        for (old, velocity, expected) in cases {
            assert_eq!(
                step(old, velocity, DT, &[], map_rect()),
                Step::LeftMap(expected)
            );
        }
    }

    #[test]
    fn starting_on_an_edge_and_moving_away_is_free() {
        let result = step(
            Vec2::new(100.0, 52.0),
            Vec2::new(-SPEED, 0.0),
            DT,
            &band_edges(),
            map_rect(),
        );
        assert!(matches!(result, Step::Flying(_)), "{result:?}");
    }

    #[test]
    fn a_hit_wins_over_leaving_the_map() {
        // The band reaches past the right edge of the map. In one step the shot reaches the band at x 198 and would
        // end at x 207, outside the map: it stops on the face.
        let edges = edges_of(&[Rect::new(198.0, 50.0, 220.0, 54.0)]);
        let result = step(
            Vec2::new(197.0, 52.0),
            Vec2::new(10.0, 0.0),
            1.0,
            &edges,
            map_rect(),
        );
        assert_eq!(result, Step::HitBlocked(Vec2::new(198.0, 52.0)));
    }

    /// 6000 px/s to the right: 96 px in one 16 ms frame.
    fn fast_right() -> Projectile {
        Projectile::new(Vec2::X, 6000.0)
    }

    /// Fires a shot from `from` to the right at 6000 px/s, 96 px in one 16 ms frame. The map is `CameraBounds` and
    /// the collision world holds the band x 100 to 120, y 50 to 54. Returns the hits seen after a few frames.
    fn fire_right(from: Vec2, targets: &[(Vec2, HitBox)], shot: Projectile) -> Vec<ProjectileHit> {
        let mut app = app();
        app.insert_resource(CameraBounds(map_rect()));
        app.world_mut()
            .resource_mut::<CollisionWorld>()
            .add_blocked(&[Rect::new(100.0, 50.0, 120.0, 54.0)]);
        for (at, hit_box) in targets {
            app.world_mut().spawn((
                Transform::from_xyz(at.x, at.y, 0.0),
                *hit_box,
                ProjectileTarget,
            ));
        }
        app.world_mut()
            .spawn((Transform::from_xyz(from.x, from.y, 0.0), shot));
        for _ in 0..10 {
            app.update();
        }
        app.world().resource::<Seen>().0.clone()
    }

    #[test]
    fn a_target_behind_a_blocked_area_is_safe() {
        // In one frame the shot reaches the band at x 100 and, without the band, the target at x 107 behind it.
        let behind = HitBox {
            offset: Vec2::ZERO,
            half_size: Vec2::new(3.0, 10.0),
        };
        let seen = fire_right(
            Vec2::new(90.0, 52.0),
            &[(Vec2::new(110.0, 52.0), behind)],
            fast_right(),
        );
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert_eq!(seen[0].cause, HitCause::Blocked);
        assert_eq!(seen[0].position, Vec2::new(100.0, 52.0));
    }

    #[test]
    fn a_shot_that_leaves_the_map_is_reported_on_the_map_edge() {
        let seen = fire_right(Vec2::new(150.0, 100.0), &[], fast_right());
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert_eq!(seen[0].cause, HitCause::MapEdge);
        assert_eq!(seen[0].position, Vec2::new(200.0, 100.0));
    }

    #[test]
    fn a_shot_that_starts_inside_a_blocked_area_leaves_it_freely() {
        // Edges stop only a move that comes from the free side. Out through each side of the band x 100 to 120, y 50
        // to 54: left, right, top and bottom.
        for (old, velocity) in [
            (Vec2::new(101.0, 52.0), Vec2::new(-SPEED, 0.0)),
            (Vec2::new(119.0, 52.0), Vec2::new(SPEED, 0.0)),
            (Vec2::new(110.0, 53.5), Vec2::new(0.0, SPEED)),
            (Vec2::new(110.0, 50.5), Vec2::new(0.0, -SPEED)),
        ] {
            let result = step(old, velocity, DT, &band_edges(), map_rect());
            let expected = old + velocity * DT;
            assert_eq!(result, Step::Flying(expected), "{old:?}");
        }
    }

    #[test]
    fn a_shot_without_targets_passes_through_a_target() {
        // In open ground on the way to the map edge at x 200.
        let body = HitBox {
            offset: Vec2::ZERO,
            half_size: Vec2::new(5.0, 10.0),
        };
        let from = Vec2::new(130.0, 100.0);
        let targets = [(Vec2::new(160.0, 100.0), body)];
        let hit = fire_right(from, &targets, fast_right());
        let passed = fire_right(from, &targets, fast_right().without_targets());
        assert_eq!(
            (hit[0].cause, hit[0].position),
            (HitCause::Target(hit_target(&hit)), Vec2::new(155.0, 100.0))
        );
        assert_eq!(passed.len(), 1, "{passed:?}");
        assert_eq!(
            (passed[0].cause, passed[0].position),
            (HitCause::MapEdge, Vec2::new(200.0, 100.0))
        );
    }

    /// The entity in a `Target` hit, so a test can compare the whole cause.
    fn hit_target(seen: &[ProjectileHit]) -> Entity {
        match seen[0].cause {
            HitCause::Target(entity) => entity,
            other => panic!("expected a target hit, got {other:?}"),
        }
    }

    /// A neutral path: 2 s, 120 px, from 10 px up to a 40 px arc, 2 bounces at a quarter height, 30 % rolling.
    fn shape() -> PathShape {
        PathShape {
            distance: 120.0,
            duration: 2.0,
            start_height: 10.0,
            peak: 40.0,
            bounces: 2,
            bounce_ratio: 0.25,
            roll_share: 0.3,
        }
    }

    /// The top of each arc, sampled every 0.1 ms over the whole path. An arc ends where the height drops below 0.1 px
    /// (near the ground the height changes fast, so f32 noise at a top cannot split an arc).
    fn peaks(shape: &PathShape) -> Vec<f32> {
        let mut tops = Vec::new();
        let mut top = 0.0f32;
        for i in 0..=(shape.duration * 10_000.0) as usize {
            let height = shape.height(i as f32 / 10_000.0);
            if height < 0.1 {
                if top > 0.0 {
                    tops.push(top);
                }
                top = 0.0;
            } else {
                top = top.max(height);
            }
        }
        tops
    }

    #[test]
    fn the_ground_distance_eases_out_from_zero_to_the_full_distance() {
        let shape = shape();
        let at = |t: f32| shape.ground_distance(t);
        // Quadratic ease-out: three quarters of the way at half the time.
        assert_eq!(
            [at(0.0), at(1.0), at(2.0), at(3.0)],
            [0.0, 90.0, 120.0, 120.0]
        );
    }

    #[test]
    fn the_arc_and_each_bounce_peak_at_their_heights_then_it_rolls_flat() {
        // 0.01 px: the 0.1 ms samples miss a top by far less (gravity here is about 100 px/s²).
        let close = |found: Vec<f32>, expected: &[f32]| {
            assert_eq!(found.len(), expected.len(), "{found:?}");
            for (found, expected) in found.iter().zip(expected) {
                assert!((found - expected).abs() < 0.01, "{found} vs {expected}");
            }
        };
        let mut shape = shape();
        assert_eq!(shape.height(0.0), 10.0);
        close(peaks(&shape), &[40.0, 10.0, 2.5]);
        // In the air until 70 % of 2 s, then on the ground.
        assert!(shape.height(1.39) > 0.0);
        assert_eq!(
            [shape.height(1.4), shape.height(1.7), shape.height(2.0)],
            [0.0; 3]
        );
        for bounces in [0, 3] {
            shape.bounces = bounces;
            let expected: Vec<f32> = (0..=bounces)
                .map(|n| 40.0 * 0.25f32.powi(n as i32))
                .collect();
            close(peaks(&shape), &expected);
            assert!(shape.height(1.39) > 0.0, "{bounces} bounces");
        }
    }

    #[test]
    fn a_path_shot_moves_along_the_ground_and_up_by_its_height_sorts_at_the_ground_and_ends_after_its_distance()
     {
        let mut app = app();
        let from = Vec2::new(20.0, 30.0);
        let direction = Vec2::new(0.6, 0.8);
        let shot = app
            .world_mut()
            .spawn((
                Transform::from_xyz(from.x, from.y, 0.0),
                ProjectilePath::new(shape(), direction, 5.0),
            ))
            .id();
        // The kit adds the projectile and its sort, at the start height (10) above the rest height (5), before any
        // frame runs.
        assert_eq!(
            app.world().get::<DepthSort>(shot).unwrap().feet_offset,
            15.0
        );
        // The first update has no time step, then 29 frames of 16 ms: 0.464 s, in the first arc. The point is the
        // ground move plus the change of height from the start (10 px). 1e-3: the sum of f32 steps.
        for _ in 0..30 {
            app.update();
        }
        let shape = shape();
        let expected = from
            + direction * shape.ground_distance(0.464)
            + Vec2::Y * (shape.height(0.464) - 10.0);
        let at = app
            .world()
            .get::<Transform>(shot)
            .unwrap()
            .translation
            .truncate();
        assert!(at.distance(expected) < 1e-3, "{at} vs {expected}");
        let sort = app.world().get::<DepthSort>(shot).unwrap().feet_offset;
        assert!((sort - (5.0 + shape.height(0.464))).abs() < 1e-3, "{sort}");
        // 2 s is 125 frames after the first. The next frame removes it.
        for _ in 30..127 {
            app.update();
        }
        let seen = &app.world().resource::<Seen>().0;
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert_eq!(
            (seen[0].projectile, seen[0].cause),
            (shot, HitCause::PathEnd)
        );
        // 120 px along (0.6, 0.8) is (72, 96), and it ends on the ground, 10 px below its start height.
        assert!(
            seen[0].position.distance(from + Vec2::new(72.0, 86.0)) < 1e-3,
            "{seen:?}"
        );
        assert!(app.world().get_entity(shot).is_err());
    }

    #[test]
    fn an_ignored_blocked_area_is_flown_through_and_others_still_stop_the_shot() {
        // Two bands side by side at y 50 to 54: x 100 to 120 (ignored) and x 140 to 160. Flying right along y 52 from
        // x 99, 200 px in one step: through the first, stopped at the face of the second.
        let edges = edges_of(&[
            Rect::new(100.0, 50.0, 120.0, 54.0),
            Rect::new(140.0, 50.0, 160.0, 54.0),
        ]);
        let ignored = Some(Rect::new(100.0, 50.0, 120.0, 54.0));
        let right = |ignored| {
            step_ignoring(
                Vec2::new(99.0, 52.0),
                Vec2::new(200.0, 0.0),
                1.0,
                &edges,
                map_rect(),
                ignored,
            )
        };
        assert_eq!(right(None), Step::HitBlocked(Vec2::new(100.0, 52.0)));
        assert_eq!(right(ignored), Step::HitBlocked(Vec2::new(140.0, 52.0)));
        // Down into the ignored band through its top edge, and out of its bottom.
        let down = step_ignoring(
            Vec2::new(110.0, 56.0),
            Vec2::new(0.0, -10.0),
            1.0,
            &edges,
            map_rect(),
            ignored,
        );
        assert_eq!(down, Step::Flying(Vec2::new(110.0, 46.0)));
    }

    #[derive(Component)]
    struct Marked;

    #[derive(Resource, Default)]
    struct MarkedHits(Vec<bool>);

    fn check_marks(
        mut hits: MessageReader<ProjectileHit>,
        marked: Query<(), With<Marked>>,
        mut seen: ResMut<MarkedHits>,
    ) {
        for hit in hits.read() {
            seen.0.push(marked.contains(hit.projectile));
        }
    }

    #[test]
    fn a_reader_after_the_set_sees_the_removed_shot_which_is_gone_after_the_frame() {
        let mut app = app();
        app.insert_resource(CameraBounds(map_rect()))
            .init_resource::<MarkedHits>()
            .add_systems(Update, check_marks.after(ProjectileSet));
        // Leaves the map in the first frame with a time step (the first update has none).
        let shot = app
            .world_mut()
            .spawn((Transform::from_xyz(190.0, 100.0, 0.0), fast_right(), Marked))
            .id();
        app.update();
        app.update();
        assert_eq!(app.world().resource::<MarkedHits>().0, [true]);
        assert!(app.world().get_entity(shot).is_err());
    }
}
