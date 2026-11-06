//! Projectiles that fly in a straight line over the ground and disappear when they hit something blocked or leave
//! the map. See `dev-docs/extensions/projectiles.md`.
//!
//! A projectile is a point on the ground, like the feet of a character, and uses the same collision edges
//! (`collision::resolve`). `step` is plain Rust with no Bevy world, so it is unit tested without a running app.

use bevy::prelude::*;

use crate::camera::CameraBounds;
use crate::collision::{CollisionWorld, resolve};
use crate::hitbox::HitBox;
use crate::outline::Edge;

/// Moves every `Projectile`, removes the ones that hit something and sends `ProjectileHit`. Needs `CollisionPlugin`
/// (the blocked areas). The map rectangle is `CameraBounds`: without it a projectile never leaves the map.
pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ProjectileHit>()
            .add_systems(Update, (move_projectiles, orient_sprites));
    }
}

/// A projectile in flight. The entity's `Transform` x and y are its ground point, the point that collides, so the
/// game puts the sprite on a child entity lifted above it. Give the entity `DepthSort { feet_offset: 0.0 }` to draw it
/// in the world layer (`adr-x2`). The game spawns the entity and its sprite, the kit only moves it.
#[derive(Component, Clone, Copy, Debug)]
pub struct Projectile {
    /// Ground velocity in world units per second.
    pub velocity: Vec2,
    /// How high above the ground point the projectile flies, as the sprite child is lifted. A `HitBox` covers a body
    /// above the feet of its owner, so a target is tested at this height. Blocked areas are tested on the ground.
    pub height: f32,
}

impl Projectile {
    /// A projectile that flies along `direction` (any length except zero) at `speed`.
    pub fn new(direction: Vec2, speed: f32) -> Self {
        Self {
            velocity: direction.normalize_or_zero() * speed,
            height: 0.0,
        }
    }

    /// The height at which the projectile flies and hits targets.
    pub fn at_height(mut self, height: f32) -> Self {
        self.height = height;
        self
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
}

/// Sent when a projectile has been removed, for impact effects or damage. `position` is the ground point.
#[derive(Message, Clone, Copy, Debug)]
pub struct ProjectileHit {
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
    mut projectiles: Query<(Entity, &mut Transform, &Projectile)>,
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
        let stepped = step(
            old,
            projectile.velocity,
            time.delta_secs(),
            world.edges(),
            area,
        );
        let reached = match stepped {
            Step::Flying(point) | Step::HitBlocked(point) | Step::LeftMap(point) => point,
        };
        // A target counts only up to the point where the shot stopped, so a target behind a tree is safe.
        let lift = Vec2::new(0.0, projectile.height);
        let cause = if let Some((index, point)) = target_hit(old + lift, reached + lift, &rects) {
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
            position: cause.0,
            cause: cause.1,
        });
        commands.entity(entity).despawn();
    }
}

/// The first of the `targets` that the segment from `old` to `new` enters: its index and the entry point. A segment
/// that starts inside a target hits it at `old`. The segment, not the end point, is tested, so a long step cannot
/// jump over a thin target.
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

/// When (0 to 1) the segment `start + delta * t` is first inside `rect`. The slab method on each axis.
fn entry_time(start: Vec2, delta: Vec2, rect: Rect) -> Option<f32> {
    let mut enter = 0.0f32;
    let mut leave = 1.0f32;
    for (from, change, low, high) in [
        (start.x, delta.x, rect.min.x, rect.max.x),
        (start.y, delta.y, rect.min.y, rect.max.y),
    ] {
        if change == 0.0 {
            if from < low || from > high {
                return None;
            }
            continue;
        }
        let (a, b) = ((low - from) / change, (high - from) / change);
        enter = enter.max(a.min(b));
        leave = leave.min(a.max(b));
    }
    (enter <= leave).then_some(enter)
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

/// Moves a projectile from `old` by `velocity * dt` and says what happened. `resolve` stops a move when it
/// crosses an edge, so a long step cannot jump over a thin blocked band. A blocked move wins over leaving the map.
pub fn step(old: Vec2, velocity: Vec2, dt: f32, edges: &[Edge], bounds: Rect) -> Step {
    let new = old + velocity * dt;
    let moved = resolve(old, new, edges);
    if moved.blocked_x.is_some() || moved.blocked_y.is_some() {
        return Step::HitBlocked(moved.feet);
    }
    if !bounds.contains(new) {
        return Step::LeftMap(new.clamp(bounds.min, bounds.max));
    }
    Step::Flying(new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outline;

    const DT: f32 = 1.0 / 60.0;
    /// Twice a walk speed of 80 px/s.
    const SPEED: f32 = 160.0;

    fn map() -> Rect {
        Rect::new(0.0, 0.0, 200.0, 200.0)
    }

    /// A blocked area, x 100 to 120, y 50 to 54: a 4 px band like the base of a lone tree.
    fn band_edges() -> Vec<Edge> {
        edges_of(Rect::new(100.0, 50.0, 120.0, 54.0))
    }

    fn edges_of(area: Rect) -> Vec<Edge> {
        outline::build(&[area])
            .iter()
            .flat_map(|c| c.edges.iter().copied())
            .collect()
    }

    #[test]
    fn the_sprite_turns_to_the_flight_direction() {
        let turned = |velocity: Vec2| sprite_rotation(velocity).unwrap() * Vec3::X;
        assert!((turned(Vec2::new(5.0, 0.0)) - Vec3::X).length() < 1e-5);
        assert!((turned(Vec2::new(0.0, 5.0)) - Vec3::Y).length() < 1e-5);
        assert!((turned(Vec2::new(-5.0, 0.0)) - Vec3::NEG_X).length() < 1e-5);
        assert!((turned(Vec2::new(0.0, -5.0)) - Vec3::NEG_Y).length() < 1e-5);
        let diagonal = turned(Vec2::new(3.0, 3.0));
        assert!((diagonal - Vec3::new(1.0, 1.0, 0.0).normalize()).length() < 1e-5);
        assert_eq!(sprite_rotation(Vec2::ZERO), None);
    }

    fn target() -> Vec<Rect> {
        vec![Rect::new(100.0, 50.0, 110.0, 62.0)]
    }

    #[derive(Resource, Default)]
    struct Seen(Vec<ProjectileHit>);

    fn collect(mut reader: MessageReader<ProjectileHit>, mut seen: ResMut<Seen>) {
        seen.0.extend(reader.read().copied());
    }

    fn app() -> App {
        use bevy::time::TimeUpdateStrategy;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(16),
            ))
            .init_resource::<CollisionWorld>()
            .init_resource::<Seen>()
            .add_plugins(ProjectilePlugin)
            .add_systems(Update, collect.after(move_projectiles));
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
        let hit = target_hit(Vec2::new(90.0, 40.0), Vec2::new(110.0, 60.0), &target());
        assert!(hit.is_some());
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
        let result = step(Vec2::new(10.0, 10.0), Vec2::new(SPEED, 0.0), DT, &[], map());
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
            map(),
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
            map(),
        );
        assert_eq!(result, Step::HitBlocked(Vec2::new(110.0, 50.0)));
    }

    #[test]
    fn diagonal_into_a_face_is_a_hit() {
        let velocity = Vec2::new(1.0, 1.0).normalize() * SPEED;
        let result = step(Vec2::new(99.5, 51.0), velocity, DT, &band_edges(), map());
        assert!(matches!(result, Step::HitBlocked(_)), "{result:?}");
    }

    #[test]
    fn a_step_larger_than_the_band_is_still_a_hit() {
        // One 100 px step from below the band to above it must not jump over the 4 px band.
        let result = step(
            Vec2::new(110.0, 40.0),
            Vec2::new(0.0, 6000.0),
            DT,
            &band_edges(),
            map(),
        );
        assert_eq!(result, Step::HitBlocked(Vec2::new(110.0, 50.0)));
    }

    #[test]
    fn a_path_beside_the_band_is_free() {
        let result = step(
            Vec2::new(130.0, 40.0),
            Vec2::new(0.0, SPEED),
            DT,
            &band_edges(),
            map(),
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
            map(),
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
            assert_eq!(step(old, velocity, DT, &[], map()), Step::LeftMap(expected));
        }
    }

    #[test]
    fn starting_on_an_edge_and_moving_away_is_free() {
        let result = step(
            Vec2::new(100.0, 52.0),
            Vec2::new(-SPEED, 0.0),
            DT,
            &band_edges(),
            map(),
        );
        assert!(matches!(result, Step::Flying(_)), "{result:?}");
    }

    #[test]
    fn a_hit_wins_over_leaving_the_map() {
        // The band touches the right edge of the map: the shot stops on its face, inside the map.
        let edges = edges_of(Rect::new(198.0, 50.0, 220.0, 54.0));
        let result = step(
            Vec2::new(197.0, 52.0),
            Vec2::new(SPEED, 0.0),
            DT,
            &edges,
            map(),
        );
        assert_eq!(result, Step::HitBlocked(Vec2::new(198.0, 52.0)));
    }

    #[test]
    fn a_shot_through_a_corner_contact_is_a_hit() {
        let edges: Vec<Edge> = outline::build(&[
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(10.0, 10.0, 20.0, 20.0),
        ])
        .iter()
        .flat_map(|c| c.edges.iter().copied())
        .collect();
        let bounds = Rect::new(-50.0, -50.0, 50.0, 50.0);
        // This line passes exactly through the vertex (10, 10).
        let shot = step(
            Vec2::new(14.0, 6.0),
            Vec2::new(-8.0, 8.0),
            1.0,
            &edges,
            bounds,
        );
        assert!(matches!(shot, Step::HitBlocked(_)), "{shot:?}");
    }
}
