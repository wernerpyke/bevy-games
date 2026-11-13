//! The ice ring (W2, `PLAN-weapons.md`): a shield, not a damage weapon (the user, 2026-11-13). It grows and holds
//! like the fire ring (`Ring` in `ring.rs`), and this module adds what differs:
//! - **Reach.** At the cast, `ICE_RAYS` rays leave the centre. Each stops at the first blocking feature and at the map
//!   edge (`ray_limits`), and the ring stops with it, so it is not a full circle near a tree.
//! - **Push.** The growing front carries every enemy it reaches to the edge of the ring, and deals the push damage
//!   once.
//! - **Barrier.** At full size, until the ring goes, an enemy that walks into it is moved back out and takes the touch
//!   damage once per cast. Only enemies: the player passes freely.
//! - **Stun.** An enemy that a push cannot move all the way, because a feature or the map edge is in the way, is
//!   `Stunned` until the ring goes. It takes no more damage from this ring. An enemy pushed into open space is not.
//!
//! Balance rule b3 (`docs/balance.md`): one ring never kills a ghost.

use std::collections::HashMap;
use std::f32::consts::TAU;

use bevy::prelude::*;
use game_kit::camera::CameraBounds;
use game_kit::collision::{CollisionWorld, ray_distance};
use game_kit::hitbox::HitBox;
use game_kit::movement::{Mover, Stunned};
use game_kit::outline::Edge;

use super::WeaponHit;
use super::damage::pushed_feet;
use super::ring::Ring;
use super::values::{Damage, ICE_RAYS};
use crate::enemy_hp::EnemyHp;

/// A move that ends this many pixels short of what was asked was stopped by a feature or the map edge.
const PIN_TOLERANCE: f32 = 0.5;
/// A body this little inside the ring is at its edge (rounding after a push), so it is left alone.
const EDGE_TOLERANCE: f32 = 0.5;

/// On the entity of an ice `Ring`: how far the ring reaches in each direction, and what it did to each enemy.
#[derive(Component, Debug)]
pub struct IceRing {
    /// The final radius of each ray. Ray `i` points at angle `TAU * i / limits.len()`.
    limits: Vec<f32>,
    bodies: HashMap<Entity, Body>,
}

/// What one ice ring has done to one enemy.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Body {
    pushed: f32,
    push_dealt: bool,
    touch_dealt: bool,
    stunned: bool,
}

impl IceRing {
    pub fn new(centre: Vec2, max_radius: f32, edges: &[Edge], bounds: Option<Rect>) -> Self {
        Self {
            limits: ray_limits(centre, max_radius, ICE_RAYS, edges, bounds),
            bodies: HashMap::new(),
        }
    }

    /// How far the ring reaches at `angle` (radians from +x), between the two nearest rays.
    pub fn limit_at(&self, angle: f32) -> f32 {
        let count = self.limits.len();
        let at = angle.rem_euclid(TAU) / TAU * count as f32;
        let first = at.floor() as usize % count;
        let second = (first + 1) % count;
        let share = at - at.floor();
        self.limits[first] + (self.limits[second] - self.limits[first]) * share
    }
}

/// The final radius of `count` rays from `centre`, ray `i` at angle `TAU * i / count`: `max`, or less where a blocked
/// edge (`ray_distance`) or the edge of `bounds` comes first.
pub fn ray_limits(
    centre: Vec2,
    max: f32,
    count: usize,
    edges: &[Edge],
    bounds: Option<Rect>,
) -> Vec<f32> {
    (0..count)
        .map(|i| {
            let dir = Vec2::from_angle(TAU * i as f32 / count as f32);
            let limit = ray_distance(centre, dir, max, edges);
            bounds.map_or(limit, |bounds| {
                limit.min(distance_to_leave(centre, dir, bounds))
            })
        })
        .collect()
}

/// How far a ray from `from`, inside `rect`, goes in the unit direction `dir` before it leaves `rect`.
fn distance_to_leave(from: Vec2, dir: Vec2, rect: Rect) -> f32 {
    let axis = |start: f32, step: f32, min: f32, max: f32| {
        if step > 0.0 {
            (max - start) / step
        } else if step < 0.0 {
            (min - start) / step
        } else {
            f32::INFINITY
        }
    };
    axis(from.x, dir.x, rect.min.x, rect.max.x)
        .min(axis(from.y, dir.y, rect.min.y, rect.max.y))
        .max(0.0)
}

/// Pushes, holds back and stuns the enemies of each ice ring. It runs after the kit has moved the enemies, so a step
/// into the ring is undone in the same frame. A dead enemy is skipped (D15).
#[allow(clippy::type_complexity)]
pub(super) fn update_ice_rings(
    mut rings: Query<(&Ring, &mut IceRing)>,
    enemies: Query<(Entity, &Transform, &HitBox, &EnemyHp, Option<&Mover>)>,
    world: Res<CollisionWorld>,
    bounds: Option<Res<CameraBounds>>,
    mut hits: MessageWriter<WeaponHit>,
    mut commands: Commands,
) {
    let bounds = bounds.map(|bounds| bounds.0);
    for (ring, mut ice) in &mut rings {
        let Damage::PushAndTouch { push, touch } = ring.values.damage else {
            continue;
        };
        let growing = ring.age < ring.shape.grow_secs;
        for (target, transform, hit_box, hp, mover) in &enemies {
            if hp.is_dead() {
                continue;
            }
            let rect = hit_box.rect_at(transform.translation.truncate());
            let away = (rect.center() - ring.centre)
                .try_normalize()
                .unwrap_or(Vec2::X);
            let front = ring.radius.min(ice.limit_at(away.to_angle()));
            let near = ring.centre.clamp(rect.min, rect.max).distance(ring.centre);
            let inside = front - near;
            if inside <= EDGE_TOLERANCE {
                continue;
            }
            let body = ice.bodies.entry(target).or_default();
            if body.stunned {
                continue;
            }
            let wanted = away * inside;
            let feet = transform.translation.truncate();
            let margin = mover.map_or(0.0, |mover| mover.edge_margin);
            let moved = pushed_feet(feet, wanted, &world, bounds, margin) - feet;
            body.pushed += moved.length();

            let mut amount = 0.0;
            if growing && !body.push_dealt {
                body.push_dealt = true;
                amount = push;
                info!("ice ring: {target:?} pushed, {push} damage");
            } else if !growing && !body.touch_dealt {
                body.touch_dealt = true;
                amount = touch;
                info!("ice ring: {target:?} walked into the ring, {touch} damage");
            }
            if (wanted - moved).length() > PIN_TOLERANCE {
                body.stunned = true;
                commands.entity(target).insert(Stunned);
                let at = feet + moved;
                info!(
                    "ice ring: {target:?} pinned at ({:.0}, {:.0}), stunned until the ring goes",
                    at.x, at.y
                );
            }
            hits.write(WeaponHit {
                target,
                weapon: ring.weapon,
                amount,
                push: wanted,
            });
        }
    }
}

/// When an ice ring goes, the enemies it stunned walk again, and the log says what it did.
pub(super) fn release_stunned(
    remove: On<Remove<IceRing>>,
    rings: Query<&IceRing>,
    mut commands: Commands,
) {
    let Ok(ice) = rings.get(remove.entity) else {
        return;
    };
    let mut report = Vec::new();
    for (target, body) in &ice.bodies {
        if body.stunned {
            commands.entity(*target).try_remove::<Stunned>();
        }
        report.push(format!(
            "{target:?} pushed {:.0} px{}{}{}",
            body.pushed,
            if body.push_dealt { ", push damage" } else { "" },
            if body.touch_dealt {
                ", touch damage"
            } else {
                ""
            },
            if body.stunned { ", stunned" } else { "" },
        ));
    }
    info!(
        "ice ring: ended, touched {}: [{}]",
        report.len(),
        report.join("; ")
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy_hp::DamageEnemy;
    use crate::weapon_menu::WeaponKind;
    use crate::weapons::CastWeapon;
    use crate::weapons::damage::apply_hits;
    use crate::weapons::ring::{cast_rings, update_rings};
    use crate::weapons::values::ICE_RING;

    const DT_MS: u64 = 16;
    /// The ring has grown and the barrier holds.
    const FULL: f32 = ICE_RING.grow_secs + 0.1;
    /// The ring is gone.
    const GONE: f32 = ICE_RING.grow_secs + ICE_RING.burn_secs + 0.2;

    #[derive(Resource, Default)]
    struct Log(Vec<WeaponHit>);

    fn collect(mut hits: MessageReader<WeaponHit>, mut log: ResMut<Log>) {
        log.0.extend(hits.read().copied());
    }

    /// Bodies with `Walker` take a step towards the origin every frame before the ring acts, like the kit's movement.
    #[derive(Component)]
    struct Walker(f32);

    fn walk(mut walkers: Query<(&Walker, &mut Transform)>) {
        for (walker, mut transform) in &mut walkers {
            transform.translation.x -= walker.0;
        }
    }

    fn app(blocked: &[Rect], bounds: Option<Rect>) -> App {
        let mut app = crate::test_util::timed_app(DT_MS);
        let mut world = CollisionWorld::default();
        world.add_blocked(blocked);
        app.add_message::<CastWeapon>()
            .add_message::<WeaponHit>()
            .add_message::<DamageEnemy>()
            .insert_resource(world)
            .init_resource::<Log>()
            .add_observer(release_stunned)
            .add_systems(
                Update,
                (
                    walk,
                    cast_rings,
                    update_rings,
                    update_ice_rings,
                    apply_hits,
                    collect,
                )
                    .chain(),
            );
        if let Some(bounds) = bounds {
            app.insert_resource(CameraBounds(bounds));
        }
        app.update();
        app
    }

    fn cast(app: &mut App) {
        app.world_mut().write_message(CastWeapon {
            weapon: WeaponKind::IceRing,
            origin: Vec2::ZERO,
            centre: Vec2::ZERO,
            facing: Vec2::X,
        });
    }

    /// An enemy whose body is a 10 by 10 box on its feet `x` pixels to the right of the centre.
    fn enemy(app: &mut App, x: f32) -> Entity {
        app.world_mut()
            .spawn((
                EnemyHp::full(100.0),
                Transform::from_xyz(x, 0.0, 0.0),
                HitBox {
                    offset: Vec2::ZERO,
                    half_size: Vec2::splat(5.0),
                },
            ))
            .id()
    }

    fn run_until(app: &mut App, seconds: f32) {
        while app.world().resource::<Time>().elapsed_secs() < seconds {
            app.update();
        }
    }

    fn x(app: &App, entity: Entity) -> f32 {
        app.world().get::<Transform>(entity).unwrap().translation.x
    }

    fn damage(app: &App, target: Entity) -> Vec<f32> {
        app.world()
            .resource::<Log>()
            .0
            .iter()
            .filter(|hit| hit.target == target && hit.amount > 0.0)
            .map(|hit| hit.amount)
            .collect()
    }

    fn stunned(app: &App, entity: Entity) -> bool {
        app.world().get::<Stunned>(entity).is_some()
    }

    #[test]
    fn each_ray_reaches_the_maximum_or_the_first_block_or_the_map_edge() {
        let mut world = CollisionWorld::default();
        world.add_blocked(&[Rect::new(100.0, -20.0, 110.0, 20.0)]);
        let bounds = Rect::new(-300.0, -50.0, 300.0, 300.0);
        // Four rays: right, up, left, down.
        let limits = ray_limits(Vec2::ZERO, 192.0, 4, world.edges(), Some(bounds));
        assert_eq!(limits.len(), 4);
        assert_eq!(limits[0], 100.0);
        assert!((limits[1] - 192.0).abs() < 1e-3, "{limits:?}");
        assert!((limits[2] - 192.0).abs() < 1e-3, "{limits:?}");
        assert!((limits[3] - 50.0).abs() < 1e-3, "{limits:?}");
        // No map: only the block cuts a ray.
        let open = ray_limits(Vec2::ZERO, 192.0, 4, world.edges(), None);
        assert!((open[3] - 192.0).abs() < 1e-3, "{open:?}");
    }

    #[test]
    fn the_reach_between_two_rays_is_shared_by_angle() {
        let ring = IceRing {
            limits: vec![100.0, 200.0, 200.0, 40.0],
            bodies: HashMap::new(),
        };
        assert_eq!(ring.limit_at(0.0), 100.0);
        assert!((ring.limit_at(TAU / 8.0) - 150.0).abs() < 1e-3);
        // Between the last ray (down) and the first (right), across a full turn, and for a negative angle.
        assert!((ring.limit_at(TAU * 7.0 / 8.0) - 70.0).abs() < 1e-3);
        assert!((ring.limit_at(-TAU / 8.0) - 70.0).abs() < 1e-3);
    }

    #[test]
    fn the_front_carries_a_body_to_the_edge_in_open_space_without_a_stun() {
        let mut app = app(&[], None);
        let body = enemy(&mut app, 50.0);
        cast(&mut app);
        run_until(&mut app, FULL);
        // The near side of the body (5 px left of its feet) is at the edge.
        assert!(
            (x(&app, body) - 5.0 - ICE_RING.max_radius).abs() < 1.0,
            "{}",
            x(&app, body)
        );
        assert!(!stunned(&app, body));
        assert_eq!(damage(&app, body), [50.0]);
    }

    #[test]
    fn a_body_carried_into_a_block_is_stunned_takes_no_more_damage_and_walks_again_when_the_ring_goes()
     {
        let mut app = app(&[Rect::new(120.0, -100.0, 130.0, 100.0)], None);
        let body = enemy(&mut app, 50.0);
        cast(&mut app);
        run_until(&mut app, FULL);
        assert_eq!(x(&app, body), 120.0);
        assert!(stunned(&app, body));
        run_until(&mut app, GONE - 0.3);
        assert!(stunned(&app, body));
        run_until(&mut app, GONE);
        assert!(!stunned(&app, body));
        assert_eq!(damage(&app, body), [50.0]);
    }

    #[test]
    fn a_body_pushed_against_the_map_edge_is_stunned() {
        let mut app = app(&[], Some(Rect::new(-300.0, -300.0, 150.0, 300.0)));
        let body = enemy(&mut app, 50.0);
        cast(&mut app);
        run_until(&mut app, FULL);
        assert_eq!(x(&app, body), 150.0);
        assert!(stunned(&app, body));
    }

    #[test]
    fn a_body_behind_a_block_is_not_reached() {
        let mut app = app(&[Rect::new(100.0, -100.0, 110.0, 100.0)], None);
        let behind = enemy(&mut app, 150.0);
        cast(&mut app);
        run_until(&mut app, GONE);
        assert_eq!(x(&app, behind), 150.0);
        assert!(app.world().resource::<Log>().0.is_empty());
    }

    #[test]
    fn a_body_that_walks_into_the_full_ring_is_held_at_the_edge_and_hurt_once() {
        let mut app = app(&[], None);
        cast(&mut app);
        run_until(&mut app, FULL);
        let body = enemy(&mut app, 260.0);
        app.world_mut().entity_mut(body).insert(Walker(2.0));
        while app.world().resource::<Time>().elapsed_secs() < GONE - 0.3 {
            app.update();
            let near = x(&app, body) - 5.0;
            assert!(near >= ICE_RING.max_radius - 1e-3, "inside at {near}");
        }
        assert_eq!(damage(&app, body), [10.0]);
        assert!(!stunned(&app, body));
        // When the ring is gone, the body walks in.
        run_until(&mut app, GONE + 0.5);
        assert!(x(&app, body) - 5.0 < ICE_RING.max_radius - 10.0);
    }

    #[test]
    fn one_ring_deals_a_pushed_body_that_walks_back_its_push_and_one_touch() {
        // Balance rule b3 at work: at most push plus touch from one ring.
        let mut app = app(&[], None);
        let body = enemy(&mut app, 50.0);
        cast(&mut app);
        run_until(&mut app, FULL);
        app.world_mut().entity_mut(body).insert(Walker(2.0));
        run_until(&mut app, GONE - 0.3);
        assert_eq!(damage(&app, body), [50.0, 10.0]);
    }

    #[test]
    fn a_dead_enemy_that_still_has_its_hit_box_is_not_pushed() {
        // D15: the ring does not rely on the enemy's plugin removing the `HitBox`.
        let mut app = app(&[], None);
        let living = enemy(&mut app, 50.0);
        let dead = enemy(&mut app, -50.0);
        app.world_mut().get_mut::<EnemyHp>(dead).unwrap().current = 0.0;
        cast(&mut app);
        run_until(&mut app, FULL);
        assert_eq!(damage(&app, living), [50.0]);
        assert_eq!(x(&app, dead), -50.0);
        assert!(
            app.world()
                .resource::<Log>()
                .0
                .iter()
                .all(|hit| hit.target != dead)
        );
    }
}
