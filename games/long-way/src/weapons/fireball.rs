//! The fireball (W4, `PLAN-weapons.md`): a shot that the game aims and that explodes on impact.
//!
//! - **Target.** The living enemy closest to the player among those on screen, or the closest on the map when none is
//!   on screen (`choose_target`). With no enemy at all it flies in a random direction and explodes after
//!   `FIREBALL.range`.
//! - **Flight.** The kit's `Projectile` moves it and reports what it hits (`ProjectileHit`). Every frame the fireball
//!   steers to its target's body, so a moving target cannot dodge it. It explodes on the first enemy it touches (the
//!   target or one in the way), on a blocking feature and at the map edge. When the target is gone or dead in
//!   flight, it flies on to the target's last place and explodes there.
//! - **Explosion.** The enemy it hit takes `direct`. Every other living enemy whose body is within
//!   `FIREBALL.blast_radius` takes `blast`, through features (b10 to b12). It does not hurt the player and does not
//!   push.
//!
//! The look is the spike's: the `head` animation, four trail emitters that follow the ball, a muzzle burst and five
//! impact bursts.

use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_aseprite_ultra::prelude::*;
use game_kit::camera::{GameCamera, visible_rect};
use game_kit::depth::DepthSort;
use game_kit::fx::{EffectLibrary, Follows, Fx, FxLooks};
use game_kit::hitbox::HitBox;
use game_kit::projectile::{
    HitCause, Projectile, ProjectileHit, ProjectilePlugin, ProjectileSprite, ProjectileTarget,
};

use super::values::{Damage, FIREBALL, values};
use super::{CastWeapon, WeaponHit, WeaponSystems};
use crate::enemy_hp::EnemyHp;
use crate::weapon_menu::WeaponKind;

const ART: &str = "art/weapons/fireball_and_rings.aseprite";
const HEAD_TAG: &str = "head";
/// The head art is 64 by 64 with the ball in the middle, about 50 px across: about 27 px at this scale (the spike).
const HEAD_SCALE: f32 = 0.55;
/// The particle look of every fireball emitter (`FxLooks`): the one frame of `tail`, a full-size blob, for the whole
/// life of a particle, as in the spike (`blob_last.png`), so the trail is a cone as wide as the head (the user,
/// 2026-11-13).
const TAIL: &str = "tail";
/// Emitters that follow the ball, back to front.
const TRAIL: [&str; 4] = [
    "effects/fireball_smoke.particle.ron",
    "effects/fireball_flame.particle.ron",
    "effects/fireball_core.particle.ron",
    "effects/fireball_embers.particle.ron",
];
/// One-shot emitters at the explosion, back to front.
const IMPACT: [&str; 5] = [
    "effects/impact_smoke.particle.ron",
    "effects/impact_fire.particle.ron",
    "effects/impact_shockwave.particle.ron",
    "effects/impact_sparks.particle.ron",
    "effects/impact_flash.particle.ron",
];
const MUZZLE: &str = "effects/muzzle.particle.ron";

pub struct FireballPlugin;

impl Plugin for FireballPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProjectilePlugin)
            .add_message::<Exploded>()
            .add_systems(Startup, load_look)
            .add_systems(
                Update,
                (
                    mark_targets,
                    cast_fireballs,
                    steer,
                    explode_on_hits,
                    explode,
                )
                    .chain()
                    .in_set(WeaponSystems::Effect),
            )
            .add_systems(
                Update,
                (dress_fireballs, turn_trails, show_explosions).after(WeaponSystems::Effect),
            );
    }
}

/// A fireball in flight, on the entity of its `Projectile`.
#[derive(Component, Debug)]
pub struct Fireball {
    /// The enemy it goes for. `None` when it was cast with no enemy on the map.
    target: Option<Entity>,
    /// The ground point it heads for: below the middle of the target's body by the flight height, so the ball meets
    /// the body. With no target, a point `FIREBALL.range` away.
    aim: Vec2,
    /// The target is gone or dead: the fireball flies on to `aim` and explodes there.
    lost: bool,
    /// Pixels flown, and the ground point of the last frame to measure them.
    travelled: f32,
    last: Vec2,
}

/// Why a fireball exploded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Why {
    /// It touched an enemy, which takes the direct hit.
    Enemy,
    /// It reached a blocking feature.
    Blocked,
    /// It reached the map edge.
    MapEdge,
    /// It flew its range with no target.
    Range,
    /// It reached the last place of a target that was gone.
    Lost,
}

/// A fireball exploded at the ground point `at`. `direct` is the enemy it touched.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
struct Exploded {
    at: Vec2,
    direct: Option<Entity>,
    why: Why,
}

/// The enemy a fireball goes for: the one closest to `from` among those whose body centre is in `view`, or the
/// closest of all when none is in view (or there is no view). `None` with no enemies.
fn choose_target(from: Vec2, view: Option<Rect>, enemies: &[(Entity, Vec2)]) -> Option<Entity> {
    let closest = |in_view_only: bool| {
        enemies
            .iter()
            .filter(|(_, at)| !in_view_only || view.is_some_and(|view| view.contains(*at)))
            .min_by(|a, b| {
                a.1.distance_squared(from)
                    .total_cmp(&b.1.distance_squared(from))
            })
            .map(|(entity, _)| *entity)
    };
    closest(true).or_else(|| closest(false))
}

/// A direction from a seed, spread evenly around the circle. No random crate needed.
fn random_direction(seed: u64) -> Vec2 {
    // splitmix64
    let mut x = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    Vec2::from_angle((x >> 40) as f32 / (1u64 << 24) as f32 * TAU)
}

/// The ground point to steer to, so the ball (at its flight height) meets the middle of `body`.
fn aim_at(body: Rect) -> Vec2 {
    body.center() - Vec2::new(0.0, FIREBALL.height)
}

/// The middle of the explosion: the ball's height above its ground point.
fn blast_centre(ground: Vec2) -> Vec2 {
    ground + Vec2::new(0.0, FIREBALL.height)
}

fn in_blast(centre: Vec2, body: Rect) -> bool {
    centre.clamp(body.min, body.max).distance(centre) < FIREBALL.blast_radius
}

/// Living enemies are projectile targets. A dead one is not, so a dying ghost does not stop a shot (D15).
fn mark_targets(
    enemies: Query<(Entity, &EnemyHp, Has<ProjectileTarget>), With<HitBox>>,
    mut commands: Commands,
) {
    for (entity, hp, marked) in &enemies {
        if hp.is_dead() && marked {
            commands.entity(entity).remove::<ProjectileTarget>();
        } else if !hp.is_dead() && !marked {
            commands.entity(entity).insert(ProjectileTarget);
        }
    }
}

fn cast_fireballs(
    mut cast: MessageReader<CastWeapon>,
    enemies: Query<(Entity, &Transform, &HitBox, &EnemyHp)>,
    camera: Query<(&GlobalTransform, &Projection), With<GameCamera>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (index, order) in cast
        .read()
        .filter(|order| order.weapon == WeaponKind::Fireball)
        .enumerate()
    {
        let bodies: Vec<(Entity, Rect)> = enemies
            .iter()
            .filter(|(.., hp)| !hp.is_dead())
            .map(|(entity, transform, hit_box, _)| {
                (entity, hit_box.rect_at(transform.translation.truncate()))
            })
            .collect();
        let centres: Vec<(Entity, Vec2)> = bodies
            .iter()
            .map(|(entity, body)| (*entity, body.center()))
            .collect();
        let view = camera
            .single()
            .ok()
            .and_then(|(transform, projection)| visible_rect(transform, projection));
        let target = choose_target(order.centre, view, &centres);
        let body = target.and_then(|target| bodies.iter().find(|(entity, _)| *entity == target));
        let (direction, aim) = match body {
            Some((_, body)) => {
                let aim = aim_at(*body);
                let direction = (aim - order.origin).try_normalize().unwrap_or(order.facing);
                (direction, aim)
            }
            None => {
                let seed = time.elapsed().as_nanos() as u64 ^ index as u64;
                let direction = random_direction(seed);
                (direction, order.origin + direction * FIREBALL.range)
            }
        };
        let start = order.origin + direction * FIREBALL.muzzle;
        match target {
            Some(target) => info!(
                "fireball: cast at ({:.0}, {:.0}) at {target:?}, {:.0} px away{}",
                start.x,
                start.y,
                aim.distance(order.origin),
                if view.is_some_and(|view| view.contains(blast_centre(aim))) {
                    ", on screen"
                } else {
                    ", off screen"
                }
            ),
            None => info!(
                "fireball: cast at ({:.0}, {:.0}) with no enemy on the map, towards ({:.2}, {:.2})",
                start.x, start.y, direction.x, direction.y
            ),
        }
        commands.spawn((
            Fireball {
                target,
                aim,
                lost: false,
                travelled: 0.0,
                last: start,
            },
            Projectile::new(direction, FIREBALL.speed).at_height(FIREBALL.height),
            Transform::from_translation(start.extend(0.0)),
            Visibility::default(),
            DepthSort { feet_offset: 0.0 },
        ));
    }
}

/// Points each fireball at its target, notices a lost target, and explodes a fireball that has reached the end of its
/// range or the last place of a lost target.
fn steer(
    time: Res<Time>,
    mut balls: Query<(Entity, &mut Fireball, &mut Projectile, &Transform)>,
    enemies: Query<(&Transform, &HitBox, &EnemyHp), Without<Fireball>>,
    mut exploded: MessageWriter<Exploded>,
    mut commands: Commands,
) {
    let step = FIREBALL.speed * time.delta_secs();
    for (entity, mut ball, mut projectile, transform) in &mut balls {
        let at = transform.translation.truncate();
        ball.travelled += at.distance(ball.last);
        ball.last = at;
        if let Some(target) = ball.target
            && !ball.lost
        {
            match enemies.get(target) {
                Ok((transform, hit_box, hp)) if !hp.is_dead() => {
                    ball.aim = aim_at(hit_box.rect_at(transform.translation.truncate()));
                }
                _ => {
                    ball.lost = true;
                    info!(
                        "fireball: target {target:?} is gone, flies on to ({:.0}, {:.0})",
                        ball.aim.x, ball.aim.y
                    );
                }
            }
        }
        let end = match (ball.target, ball.lost) {
            (None, _) if ball.travelled >= FIREBALL.range => Some((at, Why::Range)),
            (Some(_), true) if at.distance(ball.aim) <= step => Some((ball.aim, Why::Lost)),
            _ => None,
        };
        if let Some((point, why)) = end {
            exploded.write(Exploded {
                at: point,
                direct: None,
                why,
            });
            commands.entity(entity).despawn();
            continue;
        }
        if ball.target.is_some()
            && let Some(direction) = (ball.aim - at).try_normalize()
        {
            projectile.velocity = direction * FIREBALL.speed;
        }
    }
}

/// A projectile that the kit stopped is a fireball (the only projectile in Long Way): it explodes there.
fn explode_on_hits(mut hits: MessageReader<ProjectileHit>, mut exploded: MessageWriter<Exploded>) {
    for hit in hits.read() {
        let (direct, why) = match hit.cause {
            HitCause::Target(enemy) => (Some(enemy), Why::Enemy),
            HitCause::Blocked => (None, Why::Blocked),
            HitCause::MapEdge => (None, Why::MapEdge),
        };
        exploded.write(Exploded {
            at: hit.position,
            direct,
            why,
        });
    }
}

/// The enemy hit takes the direct damage, every other living enemy in the blast takes the blast damage.
fn explode(
    mut exploded: MessageReader<Exploded>,
    enemies: Query<(Entity, &Transform, &HitBox, &EnemyHp)>,
    mut hits: MessageWriter<WeaponHit>,
) {
    let Damage::Impact { direct, blast } = values(WeaponKind::Fireball).damage else {
        return;
    };
    for boom in exploded.read() {
        let centre = blast_centre(boom.at);
        let mut hit = |target: Entity, amount: f32| {
            hits.write(WeaponHit {
                target,
                weapon: WeaponKind::Fireball,
                amount,
                push: Vec2::ZERO,
            });
        };
        let mut caught = Vec::new();
        for (entity, transform, hit_box, hp) in &enemies {
            if hp.is_dead() {
                continue;
            }
            if Some(entity) == boom.direct {
                hit(entity, direct);
            } else if in_blast(centre, hit_box.rect_at(transform.translation.truncate())) {
                hit(entity, blast);
                caught.push(entity);
            }
        }
        info!(
            "fireball: exploded at ({:.0}, {:.0}) ({:?}), direct {:?}, blast {}: {caught:?}",
            boom.at.x,
            boom.at.y,
            boom.why,
            boom.direct,
            caught.len()
        );
    }
}

fn load_look(
    server: Res<AssetServer>,
    mut looks: ResMut<FxLooks>,
    mut library: ResMut<EffectLibrary>,
) {
    looks.request(&server, TAIL, ART, TAIL, true);
    for file in TRAIL.iter().chain(IMPACT.iter()).chain([MUZZLE].iter()) {
        library.keep(&server, file);
    }
}

/// An emitter of a fireball's trail. It points backwards along the flight.
#[derive(Component)]
struct Trail;

/// The emitters point this way for a ball flying at `velocity`.
fn backwards(velocity: Vec2) -> Quat {
    Quat::from_rotation_z(velocity.to_angle() + PI)
}

/// Gives a new fireball its head, its trail and a muzzle burst.
fn dress_fireballs(
    new: Query<(Entity, &Transform, &Projectile), Added<Fireball>>,
    server: Res<AssetServer>,
    looks: Res<FxLooks>,
    mut fx: Fx,
) {
    let look = looks.get(TAIL).cloned().unwrap_or_default();
    let glow = values(WeaponKind::Fireball).glow;
    let lift = Vec2::new(0.0, FIREBALL.height);
    for (owner, transform, projectile) in &new {
        let at = transform.translation.truncate() + lift;
        let head = fx
            .commands
            .spawn((
                ProjectileSprite,
                AseAnimation {
                    aseprite: server.load(ART),
                    animation: Animation::tag(HEAD_TAG).with_repeat(AnimationRepeat::Loop),
                },
                Sprite::default(),
                Anchor(Vec2::ZERO),
                Transform::from_translation(lift.extend(0.1)).with_scale(Vec3::splat(HEAD_SCALE)),
            ))
            .id();
        fx.commands.entity(owner).add_child(head);
        let rotation = backwards(projectile.velocity);
        for (layer, file) in TRAIL.iter().enumerate() {
            let emitter = fx.following(
                owner,
                file,
                Transform::from_translation(at.extend(0.0)).with_rotation(rotation),
                &look,
                glow,
            );
            // A slightly lower foot for each layer keeps them in order, back to front.
            fx.commands.entity(emitter).insert((
                Trail,
                Follows::new(owner)
                    .with_offset(lift)
                    .with_rotation(rotation),
                DepthSort {
                    feet_offset: FIREBALL.height + layer as f32 * 0.01,
                },
            ));
        }
        let muzzle = fx.one_shot(
            MUZZLE,
            Transform::from_translation(at.extend(0.0))
                .with_rotation(Quat::from_rotation_z(projectile.velocity.to_angle())),
            &look,
            glow,
        );
        fx.commands.entity(muzzle).insert(DepthSort {
            feet_offset: FIREBALL.height,
        });
    }
}

/// The trail turns with the ball as it steers.
fn turn_trails(balls: Query<&Projectile>, mut trails: Query<&mut Follows, With<Trail>>) {
    for mut follows in &mut trails {
        if let Ok(projectile) = balls.get(follows.owner)
            && projectile.velocity != Vec2::ZERO
        {
            follows.rotation = backwards(projectile.velocity);
        }
    }
}

fn show_explosions(mut exploded: MessageReader<Exploded>, looks: Res<FxLooks>, mut fx: Fx) {
    let look = looks.get(TAIL).cloned().unwrap_or_default();
    let glow = values(WeaponKind::Fireball).glow;
    for boom in exploded.read() {
        let centre = blast_centre(boom.at);
        for (layer, file) in IMPACT.iter().enumerate() {
            let burst = fx.one_shot(
                file,
                Transform::from_translation(centre.extend(0.0)),
                &look,
                glow,
            );
            fx.commands.entity(burst).insert(DepthSort {
                feet_offset: FIREBALL.height + layer as f32 * 0.01,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_kit::camera::CameraBounds;
    use game_kit::collision::CollisionWorld;

    const DT_MS: u64 = 16;

    #[derive(Resource, Default)]
    struct Log {
        hits: Vec<WeaponHit>,
        booms: Vec<Exploded>,
    }

    fn collect(
        mut hits: MessageReader<WeaponHit>,
        mut booms: MessageReader<Exploded>,
        mut log: ResMut<Log>,
    ) {
        log.hits.extend(hits.read().copied());
        log.booms.extend(booms.read().copied());
    }

    /// Moves an entity at this velocity every frame, like the kit's movement.
    #[derive(Component)]
    struct Walk(Vec2);

    fn walk(time: Res<Time>, mut walkers: Query<(&Walk, &mut Transform)>) {
        for (walk, mut transform) in &mut walkers {
            transform.translation += (walk.0 * time.delta_secs()).extend(0.0);
        }
    }

    fn app(blocked: &[Rect], bounds: Option<Rect>) -> App {
        let mut app = crate::test_util::timed_app(DT_MS);
        let mut world = CollisionWorld::default();
        world.add_blocked(blocked);
        app.add_plugins(ProjectilePlugin)
            .add_message::<CastWeapon>()
            .add_message::<WeaponHit>()
            .add_message::<Exploded>()
            .insert_resource(world)
            .init_resource::<Log>()
            .add_systems(
                Update,
                (
                    walk,
                    mark_targets,
                    cast_fireballs,
                    steer,
                    explode_on_hits,
                    explode,
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

    /// A body like a ghost's: 12 by 20, its middle 12 above the feet.
    fn body() -> HitBox {
        HitBox {
            offset: Vec2::new(0.0, 12.0),
            half_size: Vec2::new(6.0, 10.0),
        }
    }

    fn enemy(app: &mut App, feet: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                EnemyHp::full(100.0),
                Transform::from_translation(feet.extend(0.0)),
                body(),
            ))
            .id()
    }

    /// Casts from a player whose feet are at the origin.
    fn cast(app: &mut App) {
        app.world_mut().write_message(CastWeapon {
            weapon: WeaponKind::Fireball,
            origin: Vec2::ZERO,
            centre: Vec2::new(0.0, 25.0),
            facing: Vec2::X,
        });
    }

    fn run_until_exploded(app: &mut App) {
        for _ in 0..400 {
            app.update();
            if !app.world().resource::<Log>().booms.is_empty() {
                // One more frame, so the hits written after the explosion are collected.
                app.update();
                return;
            }
        }
        panic!("never exploded");
    }

    fn booms(app: &App) -> Vec<Exploded> {
        app.world().resource::<Log>().booms.clone()
    }

    /// The damage each enemy took, in order.
    fn taken(app: &App, target: Entity) -> Vec<f32> {
        app.world()
            .resource::<Log>()
            .hits
            .iter()
            .filter(|hit| hit.target == target)
            .map(|hit| hit.amount)
            .collect()
    }

    fn damage() -> (f32, f32) {
        let Damage::Impact { direct, blast } = values(WeaponKind::Fireball).damage else {
            panic!("the fireball should deal impact damage");
        };
        (direct, blast)
    }

    #[test]
    fn the_target_is_the_closest_enemy_on_screen_or_else_the_closest_on_the_map() {
        let from = Vec2::new(10.0, -20.0);
        let view = Rect::new(-90.0, -120.0, 110.0, 80.0);
        let [near_off, far_on, farther_on] =
            [1u32, 2, 3].map(|i| Entity::from_raw_u32(i).expect("a valid index"));
        // Off screen is at least 100 px from `from`, so the on-screen ones are further than the one off screen.
        let enemies = [
            (near_off, from + Vec2::new(-101.0, 0.0)),
            (farther_on, from + Vec2::new(-99.0, 99.0)),
            (far_on, from + Vec2::new(95.0, 95.0)),
        ];
        assert_eq!(choose_target(from, Some(view), &enemies), Some(far_on));
        // None on screen, or no screen: the closest on the map.
        let elsewhere = Rect::new(1000.0, 1000.0, 1200.0, 1200.0);
        assert_eq!(
            choose_target(from, Some(elsewhere), &enemies),
            Some(near_off)
        );
        assert_eq!(choose_target(from, None, &enemies), Some(near_off));
        assert_eq!(choose_target(from, Some(view), &[]), None);
    }

    #[test]
    fn random_directions_are_unit_vectors_spread_around_the_circle() {
        let mut quadrants = [0; 4];
        for seed in 0..64 {
            let direction = random_direction(seed * 16_000_000);
            assert!((direction.length() - 1.0).abs() < 1e-4, "{direction}");
            quadrants[(direction.x >= 0.0) as usize * 2 + (direction.y >= 0.0) as usize] += 1;
        }
        assert!(quadrants.iter().all(|&n| n >= 8), "{quadrants:?}");
    }

    #[test]
    fn a_fireball_hits_its_moving_target_and_passes_a_dead_enemy() {
        let mut app = app(&[], None);
        // A dead enemy, closer and in the line at the cast: not the target and not hit.
        let dead = enemy(&mut app, Vec2::new(60.0, -2.0));
        app.world_mut().get_mut::<EnemyHp>(dead).unwrap().current = 0.0;
        // The target walks across the line at a ghost's speed; a shot aimed where it was would pass under it.
        let target = enemy(&mut app, Vec2::new(220.0, 0.0));
        app.world_mut()
            .entity_mut(target)
            .insert(Walk(Vec2::new(0.0, crate::ghost::SPEED)));
        cast(&mut app);
        run_until_exploded(&mut app);
        assert_eq!(booms(&app).len(), 1);
        assert_eq!(booms(&app)[0].why, Why::Enemy);
        assert_eq!(booms(&app)[0].direct, Some(target));
        assert_eq!(taken(&app, target), vec![damage().0]);
        assert!(taken(&app, dead).is_empty());
    }

    #[test]
    fn an_enemy_in_the_way_takes_the_direct_hit_and_the_target_near_it_the_blast() {
        let mut app = app(&[], None);
        let target = enemy(&mut app, Vec2::new(170.0, 0.0));
        let far = enemy(&mut app, Vec2::new(400.0, 0.0));
        cast(&mut app);
        app.update();
        // Walks into the line after the cast.
        let in_the_way = enemy(&mut app, Vec2::new(110.0, 0.0));
        run_until_exploded(&mut app);
        let (direct, blast) = damage();
        assert_eq!(booms(&app)[0].direct, Some(in_the_way));
        assert_eq!(taken(&app, in_the_way), vec![direct]);
        assert_eq!(taken(&app, target), vec![blast]);
        assert!(taken(&app, far).is_empty());
    }

    #[test]
    fn a_target_behind_a_feature_is_missed_and_takes_the_blast_through_it_when_close() {
        let mut app = app(&[Rect::new(100.0, -60.0, 110.0, 60.0)], None);
        let target = enemy(&mut app, Vec2::new(150.0, 0.0));
        let behind = enemy(&mut app, Vec2::new(300.0, 0.0));
        cast(&mut app);
        run_until_exploded(&mut app);
        let boom = booms(&app)[0];
        assert_eq!((boom.why, boom.direct), (Why::Blocked, None));
        assert!((boom.at.x - 100.0).abs() < 0.5, "{boom:?}");
        assert_eq!(taken(&app, target), vec![damage().1]);
        assert!(taken(&app, behind).is_empty());
    }

    #[test]
    fn when_the_target_is_gone_or_dead_in_flight_it_explodes_where_the_target_was() {
        for kill in [false, true] {
            let mut app = app(&[], None);
            let target = enemy(&mut app, Vec2::new(300.0, 0.0));
            let neighbour = enemy(&mut app, Vec2::new(300.0, 60.0));
            cast(&mut app);
            run_for(&mut app, 0.2);
            if kill {
                app.world_mut().get_mut::<EnemyHp>(target).unwrap().current = 0.0;
            } else {
                app.world_mut().entity_mut(target).despawn();
            }
            run_until_exploded(&mut app);
            let boom = booms(&app)[0];
            assert_eq!((boom.why, boom.direct), (Why::Lost, None), "kill {kill}");
            let last = aim_at(body().rect_at(Vec2::new(300.0, 0.0)));
            assert!(boom.at.distance(last) < 1e-3, "{boom:?}");
            assert_eq!(taken(&app, neighbour), vec![damage().1], "kill {kill}");
            if kill {
                assert!(taken(&app, target).is_empty());
            }
        }
    }

    fn run_for(app: &mut App, seconds: f32) {
        let end = app.world().resource::<Time>().elapsed_secs() + seconds;
        while app.world().resource::<Time>().elapsed_secs() < end {
            app.update();
        }
    }

    #[test]
    fn with_no_enemy_it_flies_its_range_in_some_direction_and_explodes() {
        let mut app = app(&[], None);
        cast(&mut app);
        run_until_exploded(&mut app);
        let boom = booms(&app)[0];
        assert_eq!((boom.why, boom.direct), (Why::Range, None));
        let flown = boom.at.length() - FIREBALL.muzzle;
        let step = FIREBALL.speed * DT_MS as f32 / 1000.0;
        assert!(
            flown >= FIREBALL.range && flown < FIREBALL.range + 2.0 * step,
            "flew {flown}"
        );
    }

    #[test]
    fn it_always_explodes_at_the_map_edge() {
        let map = Rect::new(-150.0, -200.0, 250.0, 180.0);
        let mut app = app(&[], Some(map));
        cast(&mut app);
        run_until_exploded(&mut app);
        let boom = booms(&app)[0];
        assert_eq!(boom.why, Why::MapEdge);
        let on_edge = [
            boom.at.x - map.min.x,
            map.max.x - boom.at.x,
            boom.at.y - map.min.y,
            map.max.y - boom.at.y,
        ]
        .into_iter()
        .any(|gap| gap.abs() < 1e-3);
        assert!(on_edge, "{boom:?}");
    }

    #[test]
    fn the_blast_hits_every_other_living_enemy_within_its_radius_and_no_one_else() {
        let mut app = app(&[], None);
        let at = Vec2::new(-40.0, 30.0);
        let centre = blast_centre(at);
        let hit = enemy(&mut app, at + Vec2::new(0.0, -12.0));
        // Bodies whose nearest side is 1 px inside and 1 px outside the radius, to the right of the centre.
        let feet_at = |gap: f32| Vec2::new(centre.x + gap + 6.0, centre.y - 12.0);
        let inside = enemy(&mut app, feet_at(FIREBALL.blast_radius - 1.0));
        let outside = enemy(&mut app, feet_at(FIREBALL.blast_radius + 1.0));
        let dead = enemy(&mut app, feet_at(10.0));
        app.world_mut().get_mut::<EnemyHp>(dead).unwrap().current = 0.0;
        app.world_mut().write_message(Exploded {
            at,
            direct: Some(hit),
            why: Why::Enemy,
        });
        app.update();
        let (direct, blast) = damage();
        assert_eq!(taken(&app, hit), vec![direct]);
        assert_eq!(taken(&app, inside), vec![blast]);
        assert!(taken(&app, outside).is_empty());
        assert!(taken(&app, dead).is_empty());
    }

    #[test]
    fn living_enemies_are_projectile_targets_and_dead_ones_are_not() {
        let mut app = app(&[], None);
        let ghost = enemy(&mut app, Vec2::ZERO);
        app.update();
        assert!(app.world().get::<ProjectileTarget>(ghost).is_some());
        app.world_mut().get_mut::<EnemyHp>(ghost).unwrap().current = 0.0;
        app.update();
        assert!(app.world().get::<ProjectileTarget>(ghost).is_none());
    }

    /// `impact_shockwave.particle.ron`, integrated as Enoki does: the speed shrinks by `damp * progress * dt` each
    /// frame, where progress is the share of the life gone.
    fn shockwave_reach(speed: f32, life: f32, damp: f32) -> f32 {
        let dt = 1.0 / 240.0;
        let (mut v, mut travelled, mut t) = (speed, 0.0, 0.0);
        while t < life {
            t += dt;
            v *= 1.0 - (t / life) * damp * dt;
            travelled += v * dt;
        }
        travelled
    }

    #[test]
    fn the_shockwave_ends_at_about_the_blast_radius() {
        // linear_speed 340, lifetime 0.28, linear_damp 3.5 in the effect file.
        let reach = shockwave_reach(340.0, 0.28, 3.5);
        assert!(
            (reach - FIREBALL.blast_radius).abs() < 5.0,
            "the ring ends at {reach}, the blast at {}",
            FIREBALL.blast_radius
        );
    }
}
