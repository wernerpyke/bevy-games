//! The tornado (W3, `PLAN-weapons.md`; it was the whirlpool): a mobile shield. A disc on the middle of the player's
//! body that follows the player for `TORNADO.duration` (twice the ice ring, b4). Its edge starts at the inner edge of
//! the dense droplets and grows outward (`TornadoShape::edge_at`).
//!
//! - **Damage.** An enemy whose body overlaps the disc is in contact. `Contact::damage` deals a tick every 0.5 s of
//!   contact, and it only counts frames in contact, so the count is the total contact time: a ghost held against the
//!   tornado dies once that passes 60% of the duration (b5).
//! - **Push** (`Pushback::Outward`). For each enemy the tornado keeps how deep it is in the disc and where its feet were.
//!   The depth may grow by the enemy's own step since then, less the push, and not below 0. When the enemy is deeper
//!   than that, it is pushed out by the difference through the collision world. So a ghost walking in gains at its walk
//!   speed less the push (b7), a ghost that stands is pushed out, the growing edge carries what it meets, and the
//!   player's own movement never takes an enemy deeper: the edge shoves it ahead (b8).
//! - **Bump.** On every `TORNADO.bump_every`-th tick an enemy is also shoved `TORNADO.bump` pixels outward over
//!   `TORNADO.bump_secs`, in or out of contact, so a ghost walking in is knocked back a few times (b9).
//! - **Features.** The tornado passes over them. A pushed enemy stops at a feature or the map edge and is not stunned:
//!   it keeps being pushed and hurt there.
//!
//! The look is the spike's whirlpool variant B (the user, 2026-11-13): `EMITTERS` emitters on a small circle send
//! droplets along the tangent, and an attractor at the centre bends them into an outward spiral.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::prelude::*;
use game_kit::Controlled;
use game_kit::camera::CameraBounds;
use game_kit::collision::CollisionWorld;
use game_kit::depth::DepthSort;
use game_kit::fx::{
    Attractor, EffectLibrary, Fx, Look, OneShot, ParticleEffectInstance, ParticleSpawnerState,
};
use game_kit::hitbox::HitBox;
use game_kit::movement::Mover;

use super::damage::{Contact, pushed_feet};
use super::values::{Pushback, TORNADO, WeaponValues, values};
use super::{CastWeapon, WeaponHit, WeaponSystems};
use crate::enemy_hp::EnemyHp;
use crate::weapon_menu::WeaponKind;

const EFFECT: &str = "effects/tornado_orbit.particle.ron";
/// Emitters on a circle of `ORBIT_RADIUS` around the centre (the spike's variant B).
const EMITTERS: usize = 10;
/// The radius of the emitters and the strength of the attractor. A circle of radius R needs the speed
/// sqrt(strength / R): sqrt(1.6e6 / 65) = 157 px/s. The effect file starts at 185, above that, so the droplets spiral
/// outward from the start of the edge (`TORNADO.start_radius`) to past its full radius in their life (the test at the
/// end).
const ORBIT_RADIUS: f32 = 65.0;
const ORBIT_STRENGTH: f32 = 1.6e6;
/// The push edge (the user, 2026-11-13): 1 px dots on the edge of the disc that turn clockwise like the droplets, so a
/// walking player sees where the push starts. They are sprites placed each frame, not particles: a particle in flight
/// either trails behind a walking player or, moved with its emitter, smears along the emitter's turn.
const EDGE_DOTS: usize = 48;
/// Radians per second, clockwise.
const EDGE_SPIN: f32 = 1.0;
const EDGE_COLOR: Color = Color::srgb(0.92, 0.97, 1.0);

pub struct TornadoPlugin;

impl Plugin for TornadoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_effect)
            .add_systems(
                Update,
                (cast_tornadoes, update_tornadoes)
                    .chain()
                    .in_set(WeaponSystems::Effect),
            )
            .add_systems(
                Update,
                (
                    spawn_emitters,
                    place_emitters,
                    set_attractors,
                    end_emitters,
                    spawn_edge_dots,
                    place_edge_dots,
                )
                    .chain()
                    .after(WeaponSystems::Effect),
            );
    }
}

/// A tornado in the world. `bodies` remembers each enemy it has seen.
#[derive(Component)]
pub struct Tornado {
    centre: Vec2,
    age: f32,
    values: WeaponValues,
    bodies: HashMap<Entity, Body>,
}

/// What one tornado knows about one enemy.
#[derive(Clone, Copy, Debug, Default)]
struct Body {
    contact: Contact,
    /// How deep the body was in the disc after the last frame's push, in pixels. 0 outside.
    depth: f32,
    /// Its feet after the last frame's push. `None` until the tornado first sees it.
    feet: Option<Vec2>,
    /// Pixels of the current bump still to shove.
    bump_left: f32,
    pushed: f32,
}

impl Tornado {
    /// The radius of the edge now.
    fn edge(&self) -> f32 {
        TORNADO.edge_at(self.age)
    }
}

/// An emitter of a tornado at `slot` on its circle. When the tornado is gone it stops and removes itself.
#[derive(Component)]
struct TornadoEmitter {
    owner: Entity,
    slot: usize,
}

/// Dot `slot` of the push edge of a tornado.
#[derive(Component)]
struct EdgeDot {
    owner: Entity,
    slot: usize,
}

/// The middle of the player's body (where a tornado is centred).
fn player_centre(transform: &Transform, hit_box: &HitBox) -> Vec2 {
    hit_box.rect_at(transform.translation.truncate()).center()
}

/// How deep a body is in a disc: the radius less the distance from the centre to the nearest point of the body.
fn depth_in(centre: Vec2, radius: f32, body: Rect) -> f32 {
    radius - centre.clamp(body.min, body.max).distance(centre)
}

fn cast_tornadoes(mut cast: MessageReader<CastWeapon>, mut commands: Commands) {
    for order in cast.read() {
        if order.weapon != WeaponKind::Tornado {
            continue;
        }
        info!(
            "tornado: started at ({:.0}, {:.0}), edge {:.0} to {:.0} in {:.1} s, lasts {:.2} s",
            order.centre.x,
            order.centre.y,
            TORNADO.start_radius,
            TORNADO.radius,
            TORNADO.grow_secs,
            TORNADO.duration
        );
        commands.spawn(Tornado {
            centre: order.centre,
            age: 0.0,
            values: values(WeaponKind::Tornado),
            bodies: HashMap::new(),
        });
    }
}

/// Moves each tornado with the player, hurts and pushes the enemies in it, and ends it after its duration. A dead
/// enemy is skipped (D15).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_tornadoes(
    time: Res<Time>,
    player: Query<(&Transform, &HitBox), With<Controlled>>,
    mut tornadoes: Query<(Entity, &mut Tornado)>,
    enemies: Query<(Entity, &Transform, &HitBox, &EnemyHp, Option<&Mover>), Without<Controlled>>,
    world: Res<CollisionWorld>,
    bounds: Option<Res<CameraBounds>>,
    mut hits: MessageWriter<WeaponHit>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let bounds = bounds.map(|bounds| bounds.0);
    for (entity, mut tornado) in &mut tornadoes {
        tornado.age += dt;
        if tornado.age >= TORNADO.duration {
            let touched: Vec<String> = tornado
                .bodies
                .iter()
                .filter(|(_, body)| body.contact.ticks() > 0 || body.pushed > 0.0)
                .map(|(target, body)| {
                    format!(
                        "{target:?} {} ticks, pushed {:.0} px",
                        body.contact.ticks(),
                        body.pushed
                    )
                })
                .collect();
            info!(
                "tornado: ended at age {:.2} s, touched {}: [{}]",
                tornado.age,
                touched.len(),
                touched.join("; ")
            );
            commands.entity(entity).despawn();
            continue;
        }
        if let Ok((transform, hit_box)) = player.single() {
            tornado.centre = player_centre(transform, hit_box);
        }
        let Pushback::Outward(push_speed) = tornado.values.pushback else {
            continue;
        };
        let (centre, values, edge) = (tornado.centre, tornado.values, tornado.edge());
        for (target, transform, hit_box, hp, mover) in &enemies {
            if hp.is_dead() {
                continue;
            }
            let feet = transform.translation.truncate();
            let rect = hit_box.rect_at(feet);
            let depth = depth_in(centre, edge, rect);
            let body = tornado.bodies.entry(target).or_default();
            // The first time the tornado sees a body, it takes the body where it is.
            let previous = body.feet.map_or(depth.max(0.0), |_| body.depth);
            let away = (rect.center() - centre).try_normalize().unwrap_or(Vec2::X);
            let own_step = body.feet.map_or(0.0, |before| (before - feet).dot(away));

            let mut amount = None;
            let mut out = 0.0;
            if depth > 0.0 {
                amount = body.contact.damage(&values, dt);
                if amount.is_some() && body.contact.ticks() % TORNADO.bump_every == 0 {
                    body.bump_left = TORNADO.bump;
                }
                let allowed = (previous + own_step - push_speed * dt).max(0.0);
                out = (depth - allowed).max(0.0);
            }
            let bump = body.bump_left.min(TORNADO.bump / TORNADO.bump_secs * dt);
            body.bump_left -= bump;
            out += bump;

            let mut push = Vec2::ZERO;
            let mut after = feet;
            if out > 0.0 {
                push = away * out;
                let margin = mover.map_or(0.0, |mover| mover.edge_margin);
                after = pushed_feet(feet, push, &world, bounds, margin);
                body.pushed += after.distance(feet);
            }
            body.feet = Some(after);
            body.depth = depth_in(centre, edge, hit_box.rect_at(after)).max(0.0);

            if amount.is_some() || push != Vec2::ZERO {
                hits.write(WeaponHit {
                    target,
                    weapon: WeaponKind::Tornado,
                    amount: amount.unwrap_or(0.0),
                    push,
                });
            }
        }
    }
}

fn load_effect(server: Res<AssetServer>, mut library: ResMut<EffectLibrary>) {
    library.keep(&server, EFFECT);
}

/// Where emitter `slot` sits around the centre: on the circle of `ORBIT_RADIUS`, turned so that it sends its droplets
/// clockwise along the tangent.
fn emitter_transform(centre: Vec2, slot: usize) -> Transform {
    let angle = TAU * slot as f32 / EMITTERS as f32;
    Transform::from_translation((centre + Vec2::from_angle(angle) * ORBIT_RADIUS).extend(0.0))
        .with_rotation(Quat::from_rotation_z(angle - FRAC_PI_2))
}

fn spawn_emitters(new: Query<(Entity, &Tornado), Added<Tornado>>, mut fx: Fx) {
    for (owner, tornado) in &new {
        for slot in 0..EMITTERS {
            let emitter = fx.emitter(
                EFFECT,
                emitter_transform(tornado.centre, slot),
                &Look::Squares,
                tornado.values.glow,
            );
            fx.commands.entity(emitter).insert((
                TornadoEmitter { owner, slot },
                DepthSort { feet_offset: 0.0 },
            ));
        }
    }
}

/// The emitters follow their tornado (and so the player).
fn place_emitters(
    tornadoes: Query<&Tornado>,
    mut emitters: Query<(&TornadoEmitter, &mut Transform)>,
) {
    for (emitter, mut transform) in &mut emitters {
        if let Ok(tornado) = tornadoes.get(emitter.owner) {
            let placed = emitter_transform(tornado.centre, emitter.slot);
            transform.translation.x = placed.translation.x;
            transform.translation.y = placed.translation.y;
            transform.rotation = placed.rotation;
        }
    }
}

/// The attractor of an emitter is an offset from the emitter, so it points from the emitter to the centre. It is set
/// once per effect instance, when the instance exists.
fn set_attractors(mut emitters: Query<(&TornadoEmitter, &mut ParticleEffectInstance)>) {
    for (emitter, mut instance) in &mut emitters {
        if let Some(effect) = instance.0.as_mut()
            && effect.attractors.is_none()
        {
            let angle = TAU * emitter.slot as f32 / EMITTERS as f32;
            effect.attractors = Some(vec![Attractor {
                position: -Vec2::from_angle(angle) * ORBIT_RADIUS,
                strength: ORBIT_STRENGTH,
                min_distance: 10.0,
            }]);
        }
    }
}

fn end_emitters(
    tornadoes: Query<(), With<Tornado>>,
    mut emitters: Query<(Entity, &TornadoEmitter, &mut ParticleSpawnerState)>,
    mut commands: Commands,
) {
    for (entity, emitter, mut state) in &mut emitters {
        if !tornadoes.contains(emitter.owner) {
            state.active = false;
            commands
                .entity(entity)
                .remove::<TornadoEmitter>()
                .insert(OneShot::Despawn);
        }
    }
}

/// Where dot `slot` of the push edge is when the tornado is `age` seconds old: on the edge, turned clockwise.
fn edge_dot_at(centre: Vec2, age: f32, slot: usize) -> Vec2 {
    let angle = TAU * slot as f32 / EDGE_DOTS as f32 - EDGE_SPIN * age;
    centre + Vec2::from_angle(angle) * TORNADO.edge_at(age)
}

fn spawn_edge_dots(new: Query<(Entity, &Tornado), Added<Tornado>>, mut commands: Commands) {
    for (owner, tornado) in &new {
        for slot in 0..EDGE_DOTS {
            let at = edge_dot_at(tornado.centre, tornado.age, slot);
            commands.spawn((
                EdgeDot { owner, slot },
                Sprite::from_color(EDGE_COLOR, Vec2::ONE),
                Transform::from_translation(at.extend(0.0)),
                DepthSort { feet_offset: 0.0 },
            ));
        }
    }
}

/// The dots follow their tornado and turn. When the tornado is gone they go too.
fn place_edge_dots(
    tornadoes: Query<&Tornado>,
    mut dots: Query<(Entity, &EdgeDot, &mut Transform)>,
    mut commands: Commands,
) {
    for (entity, dot, mut transform) in &mut dots {
        let Ok(tornado) = tornadoes.get(dot.owner) else {
            commands.entity(entity).despawn();
            continue;
        };
        let at = edge_dot_at(tornado.centre, tornado.age, dot.slot);
        transform.translation.x = at.x;
        transform.translation.y = at.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy_hp::DamageEnemy;
    use crate::weapons::damage::apply_hits;
    use crate::weapons::values::Damage;
    use game_kit::movement::Stunned;

    const DT_MS: u64 = 16;

    /// The tornado's push speed, from its values.
    fn push_speed() -> f32 {
        let Pushback::Outward(speed) = values(WeaponKind::Tornado).pushback else {
            panic!("the tornado should push outward at a speed");
        };
        speed
    }

    #[derive(Resource, Default)]
    struct Log(Vec<(f32, WeaponHit)>);

    fn collect(mut hits: MessageReader<WeaponHit>, time: Res<Time>, mut log: ResMut<Log>) {
        let now = time.elapsed_secs();
        log.0.extend(hits.read().map(|hit| (now, *hit)));
    }

    /// Moves an entity at this velocity every frame before the tornado acts, like the kit's movement.
    #[derive(Component)]
    struct Walk(Vec2);

    fn walk(time: Res<Time>, mut walkers: Query<(&Walk, &mut Transform)>) {
        for (walk, mut transform) in &mut walkers {
            transform.translation += (walk.0 * time.delta_secs()).extend(0.0);
        }
    }

    fn app(blocked: &[Rect]) -> App {
        let mut app = crate::test_util::timed_app(DT_MS);
        let mut world = CollisionWorld::default();
        world.add_blocked(blocked);
        app.add_message::<CastWeapon>()
            .add_message::<WeaponHit>()
            .add_message::<DamageEnemy>()
            .insert_resource(world)
            .init_resource::<Log>()
            .add_systems(
                Update,
                (
                    walk,
                    cast_tornadoes,
                    update_tornadoes,
                    apply_hits,
                    collect,
                    spawn_edge_dots,
                    place_edge_dots,
                )
                    .chain(),
            );
        app.update();
        app
    }

    /// A 10 by 10 body on its feet at `at`.
    fn body() -> HitBox {
        HitBox {
            offset: Vec2::ZERO,
            half_size: Vec2::splat(5.0),
        }
    }

    fn player(app: &mut App, at: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                Controlled,
                Transform::from_translation(at.extend(0.0)),
                body(),
            ))
            .id()
    }

    fn enemy(app: &mut App, at: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                EnemyHp::full(100.0),
                Transform::from_translation(at.extend(0.0)),
                body(),
            ))
            .id()
    }

    fn cast(app: &mut App) {
        app.world_mut().write_message(CastWeapon {
            weapon: WeaponKind::Tornado,
            origin: Vec2::ZERO,
            centre: Vec2::ZERO,
            facing: Vec2::X,
        });
    }

    fn feet(app: &App, entity: Entity) -> Vec2 {
        app.world()
            .get::<Transform>(entity)
            .unwrap()
            .translation
            .truncate()
    }

    fn now(app: &App) -> f32 {
        app.world().resource::<Time>().elapsed_secs()
    }

    fn run(app: &mut App, seconds: f32) {
        let end = now(app) + seconds;
        while now(app) < end {
            app.update();
        }
    }

    /// The edge of the one tornado now.
    fn edge(app: &mut App) -> f32 {
        let world = app.world_mut();
        let mut tornadoes = world.query::<&Tornado>();
        tornadoes.single(world).expect("one tornado").edge()
    }

    /// How deep `enemy` is in the tornado around `player`.
    fn depth(app: &mut App, player: Entity, enemy: Entity) -> f32 {
        depth_in(
            feet(app, player),
            edge(app),
            body().rect_at(feet(app, enemy)),
        )
    }

    /// The edge's growth, in pixels per second.
    fn growth() -> f32 {
        (TORNADO.radius - TORNADO.start_radius) / TORNADO.grow_secs
    }

    /// The times of the damage ticks on `target`.
    fn ticks(app: &App, target: Entity) -> Vec<f32> {
        app.world()
            .resource::<Log>()
            .0
            .iter()
            .filter(|(_, hit)| hit.target == target && hit.amount > 0.0)
            .map(|(at, _)| *at)
            .collect()
    }

    #[test]
    fn a_ghost_walking_at_a_standing_player_is_bumped_back_at_least_twice_and_gets_through() {
        // b7 and b9 in the real system.
        let mut app = app(&[]);
        let player = player(&mut app, Vec2::ZERO);
        let ghost = enemy(&mut app, Vec2::new(300.0, 0.0));
        app.world_mut()
            .entity_mut(ghost)
            .insert(Walk(Vec2::new(-crate::ghost::SPEED, 0.0)));
        cast(&mut app);
        // A bump is a run of frames in which the ghost goes back, more than its step.
        let (mut bumps, mut going_back, mut last) = (0, false, feet(&app, ghost).x);
        while feet(&app, ghost).x - feet(&app, player).x > 10.0 {
            assert!(now(&app) < TORNADO.duration, "never gets through");
            app.update();
            let x = feet(&app, ghost).x;
            let back = x - last > 1.0;
            if back && !going_back {
                bumps += 1;
            }
            (going_back, last) = (back, x);
        }
        assert!(bumps >= 2, "{bumps} bumps on the way in");
        // It reaches the player's body before the tick that kills (the 8th).
        let dealt = ticks(&app, ghost).len();
        assert!((4..8).contains(&dealt), "{dealt} ticks on the way in");
    }

    #[test]
    fn a_standing_enemy_inside_is_pushed_out_by_the_push_and_the_growth_then_bumped_and_left_outside()
     {
        let mut app = app(&[]);
        let player = player(&mut app, Vec2::ZERO);
        let ghost = enemy(&mut app, Vec2::new(20.0, 0.0));
        cast(&mut app);
        run(&mut app, 0.05);
        // Before its second tick (0.5 s) there is no bump: only the steady push and the growing edge.
        let start = feet(&app, ghost).x;
        run(&mut app, 0.3);
        let moved = feet(&app, ghost).x - start;
        let expected = (push_speed() + growth()) * 0.3;
        assert!((moved - expected).abs() < 1.5, "moved {moved} in 0.3 s");
        run(&mut app, 0.3);
        assert!(
            feet(&app, ghost).x - start > expected + TORNADO.bump * 0.9,
            "not bumped on the second tick: {}",
            feet(&app, ghost).x
        );
        run(&mut app, 3.0);
        let near = feet(&app, ghost).x - 5.0;
        assert!(depth(&mut app, player, ghost) <= 0.0, "near side at {near}");
        assert!(near <= TORNADO.radius + TORNADO.bump, "near side at {near}");
    }

    #[test]
    fn the_players_own_movement_never_takes_an_enemy_deeper() {
        // b8: walking at a ghost that stands shoves it ahead at the edge.
        let mut app = app(&[]);
        let player = player(&mut app, Vec2::ZERO);
        app.world_mut()
            .entity_mut(player)
            .insert(Walk(Vec2::new(300.0, 0.0)));
        let ghost = enemy(&mut app, Vec2::new(250.0, 0.0));
        cast(&mut app);
        for _ in 0..60 {
            app.update();
            let deeper = depth(&mut app, player, ghost);
            assert!(deeper < 0.01, "{deeper} px deep");
        }
        let ahead = feet(&app, ghost).x - feet(&app, player).x;
        assert!(ahead > TORNADO.start_radius, "{ahead} px ahead");
    }

    #[test]
    fn a_player_who_keeps_walking_at_a_standing_ghost_holds_it_in_contact_long_enough_to_kill_it() {
        // b5: the bumps knock it ahead, and the player catches up.
        let mut app = app(&[]);
        let player = player(&mut app, Vec2::ZERO);
        app.world_mut()
            .entity_mut(player)
            .insert(Walk(Vec2::new(crate::player::WALK_SPEED, 0.0)));
        let ghost = enemy(&mut app, Vec2::new(150.0, 0.0));
        cast(&mut app);
        run(&mut app, TORNADO.duration);
        let Damage::PerSecond(dps) = values(WeaponKind::Tornado).damage else {
            panic!("the tornado should deal damage per second");
        };
        let per_tick = dps * values(WeaponKind::Tornado).tick;
        let dealt = ticks(&app, ghost).len() as f32 * per_tick;
        assert!(dealt >= crate::ghost::HIT_POINTS, "dealt {dealt}");
    }

    #[test]
    fn contact_is_counted_in_total_across_a_break() {
        // The block holds it inside, so neither the push nor a bump breaks the contact.
        let mut app = app(&[Rect::new(50.0, -100.0, 60.0, 100.0)]);
        player(&mut app, Vec2::ZERO);
        let ghost = enemy(&mut app, Vec2::new(40.0, 0.0));
        cast(&mut app);
        // Ticks at 0, 0.5 and 1.0 s of contact, then 0.2 s more.
        run(&mut app, 1.2);
        assert_eq!(ticks(&app, ghost).len(), 3);
        let set = |app: &mut App, x: f32| {
            app.world_mut()
                .get_mut::<Transform>(ghost)
                .unwrap()
                .translation
                .x = x;
        };
        set(&mut app, 400.0);
        run(&mut app, 1.0);
        assert_eq!(ticks(&app, ghost).len(), 3);
        // Back in: the next tick is due after the 0.3 s left of its 0.5 s, not at once.
        set(&mut app, 40.0);
        run(&mut app, 0.25);
        assert_eq!(ticks(&app, ghost).len(), 3);
        run(&mut app, 0.1);
        assert_eq!(ticks(&app, ghost).len(), 4);
    }

    #[test]
    fn an_enemy_pushed_into_a_block_stays_there_unstunned_and_keeps_taking_ticks() {
        let mut app = app(&[Rect::new(150.0, -100.0, 160.0, 100.0)]);
        let player = player(&mut app, Vec2::ZERO);
        app.world_mut()
            .entity_mut(player)
            .insert(Walk(Vec2::new(100.0, 0.0)));
        let ghost = enemy(&mut app, Vec2::new(140.0, 0.0));
        cast(&mut app);
        run(&mut app, 1.0);
        app.world_mut().entity_mut(player).remove::<Walk>();
        let before = ticks(&app, ghost).len();
        run(&mut app, 2.0);
        assert_eq!(feet(&app, ghost).x, 150.0);
        assert!(app.world().get::<Stunned>(ghost).is_none());
        assert_eq!(ticks(&app, ghost).len(), before + 4);
    }

    #[test]
    fn it_follows_the_player_and_ends_after_its_duration() {
        let mut app = app(&[]);
        let player = player(&mut app, Vec2::new(10.0, 20.0));
        app.world_mut()
            .entity_mut(player)
            .insert(Walk(Vec2::new(50.0, -30.0)));
        cast(&mut app);
        let centre = |app: &mut App| {
            app.world_mut()
                .query::<&Tornado>()
                .iter(app.world())
                .map(|tornado| tornado.centre)
                .collect::<Vec<_>>()
        };
        run(&mut app, 1.0);
        let centres = centre(&mut app);
        assert_eq!(centres.len(), 1);
        // The player's body is centred on its feet here; the centre is that rectangle's middle.
        assert!(
            centres[0].distance(feet(&app, player)) < 1e-3,
            "{centres:?}"
        );
        assert!(centres[0].x > 55.0, "{centres:?}");
        run(&mut app, TORNADO.duration - 1.0 - 0.05);
        assert_eq!(centre(&mut app).len(), 1);
        run(&mut app, 0.1);
        assert!(centre(&mut app).is_empty());
    }

    #[test]
    fn the_edge_dots_sit_on_the_push_edge_turn_clockwise_follow_the_player_and_end_with_the_tornado()
     {
        let mut app = app(&[]);
        let player = player(&mut app, Vec2::new(-30.0, 15.0));
        app.world_mut()
            .entity_mut(player)
            .insert(Walk(Vec2::new(300.0, 40.0)));
        cast(&mut app);
        // The tornado's centre and edge, and the slot and position of every dot.
        let dots = |app: &mut App| {
            let centre = app
                .world_mut()
                .query::<&Tornado>()
                .iter(app.world())
                .map(|tornado| (tornado.centre, tornado.edge()))
                .next();
            let mut dots: Vec<(usize, Vec2)> = app
                .world_mut()
                .query::<(&EdgeDot, &Transform)>()
                .iter(app.world())
                .map(|(dot, transform)| (dot.slot, transform.translation.truncate()))
                .collect();
            dots.sort_by_key(|(slot, _)| *slot);
            (centre, dots)
        };
        run(&mut app, 0.5);
        let (Some((centre, edge)), before) = dots(&mut app) else {
            panic!("no tornado");
        };
        assert_eq!(before.len(), EDGE_DOTS);
        assert!(centre.distance(feet(&app, player)) < 1e-3);
        // Still growing at 0.5 s, so this is not the full radius.
        assert!(edge < TORNADO.radius - 5.0, "edge at {edge}");
        for (slot, at) in &before {
            let off = at.distance(centre) - edge;
            assert!(off.abs() < 1e-3, "dot {slot} is {off} px off the edge");
        }
        run(&mut app, 0.2);
        let (Some((later, _)), after) = dots(&mut app) else {
            panic!("no tornado");
        };
        // Clockwise: dot 0 turned by a negative angle about the centre, at the spin speed.
        let turn = (before[0].1 - centre).angle_to(after[0].1 - later);
        assert!(
            (turn + EDGE_SPIN * 0.2).abs() < 0.03,
            "turned {turn} rad in 0.2 s"
        );
        run(&mut app, TORNADO.duration);
        assert!(dots(&mut app).1.is_empty());
    }

    #[test]
    fn a_dead_enemy_that_still_has_its_hit_box_is_not_hit_or_pushed() {
        // D15: the tornado does not rely on the enemy's plugin removing the `HitBox`.
        let mut app = app(&[]);
        player(&mut app, Vec2::ZERO);
        let living = enemy(&mut app, Vec2::new(50.0, 0.0));
        let dead = enemy(&mut app, Vec2::new(-50.0, 0.0));
        app.world_mut().get_mut::<EnemyHp>(dead).unwrap().current = 0.0;
        cast(&mut app);
        // Ticks at 0 and 0.5 s of contact.
        run(&mut app, 0.9);
        assert_eq!(ticks(&app, living).len(), 2);
        assert_eq!(feet(&app, dead), Vec2::new(-50.0, 0.0));
        assert!(
            app.world()
                .resource::<Log>()
                .0
                .iter()
                .all(|(_, hit)| hit.target != dead)
        );
    }

    /// Enoki's attractor, integrated as Enoki does: v += d * (strength / max(d2, min2) * dt / |d|), p += v * dt.
    /// The distance from the centre after `secs` of a droplet that starts at `radius` with `speed` along the tangent.
    fn orbit_radius(radius: f32, speed: f32, secs: f32) -> f32 {
        let (mut x, mut y) = (radius, 0.0f32);
        let (mut vx, mut vy) = (0.0f32, -speed);
        let dt = 1.0 / 60.0;
        let mut t = 0.0;
        while t < secs {
            let d2 = x * x + y * y;
            let force = ORBIT_STRENGTH / d2.max(100.0) * dt / d2.sqrt();
            vx -= x * force;
            vy -= y * force;
            x += vx * dt;
            y += vy * dt;
            t += dt;
        }
        (x * x + y * y).sqrt()
    }

    #[test]
    fn the_droplets_spiral_outward_from_the_start_of_the_edge_to_past_its_full_radius() {
        // The start speed of tornado_orbit.particle.ron (185 px/s) is above the circular speed. A droplet lives 1.3 s.
        assert!(185.0 > (ORBIT_STRENGTH / ORBIT_RADIUS).sqrt());
        let at = |secs| orbit_radius(ORBIT_RADIUS, 185.0, secs);
        let start = at(0.0);
        assert!(
            (start - TORNADO.start_radius).abs() < 2.0,
            "starts at {start}, the edge at {}",
            TORNADO.start_radius
        );
        assert!(
            at(0.4) > at(0.0) && at(0.9) > at(0.4) && at(1.3) > at(0.9),
            "moves out all the time"
        );
        let end = at(1.3);
        assert!(
            end > TORNADO.radius,
            "ends at {end}, the edge is at {}",
            TORNADO.radius
        );
    }
}
