//! The rings (W1 fire, W2 ice): a ring grows from the middle of the player's body to a maximum radius, holds there for
//! a while, then goes. It does not follow the player.
//!
//! Gameplay is plain and has no particles. `Ring` holds the radius and the age of both rings. The fire ring's hits are
//! here (`update_rings`): every enemy in the damaging band (`game_kit::area`) gets a `WeaponHit`. The ice ring's reach,
//! push, barrier and stun are in `ice_ring.rs`, which reads the same `Ring`. The look reads them too:
//! - Growing: `front_emitters` positions on the circle, moved outward every frame to the eased radius (`move_front`).
//!   Their particles stay where they are born, so the band behind the front is the trail. The fire ring's damaging band
//!   reaches back `trail_secs` in time to match it. An ice emitter stops where its direction is cut short.
//! - Burning: when the ring is nearly at the maximum radius, `burners` small emitters are placed on the circle (for
//!   ice, at the reach of their direction) and shimmer until the ring goes. Enoki cannot hold particles on a ring by
//!   itself, so the ring is made of emitters.
//!
//! The fire ring passes over features and they do not react to it (`adr-lw33`); a pushed enemy stops at them. The ice
//! ring stops at them (`ice_ring.rs`).

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::time::Duration;

use bevy::prelude::*;
use game_kit::area::{rect_overlaps_band, ring_radius};
use game_kit::camera::CameraBounds;
use game_kit::collision::CollisionWorld;
use game_kit::depth::DepthSort;
use game_kit::fx::{EffectLibrary, Fx, FxLooks, Look, OneShot, ParticleSpawnerState};
use game_kit::hitbox::HitBox;

use super::damage::Contact;
use super::ice_ring::{self, IceRing};
use super::values::{RingShape, WeaponValues, ring_shape, values};
use super::{CastWeapon, WeaponHit, WeaponSystems};
use crate::enemy_hp::EnemyHp;
use crate::weapon_menu::WeaponKind;

const ART: &str = "art/weapons/fireball_and_rings.aseprite";

/// The particles of a ring: its effect files, its sprite looks (tags of `ART`) and how many emitters share one
/// position on the circle.
struct RingArt {
    front: &'static str,
    burn: &'static str,
    looks: &'static [&'static str],
    /// Ice has two emitters per position, each at half the spawn rate, so the shards of neighbours mix (the spike).
    per_position: usize,
    /// The `spawn_rate` of the front and burn files. The emitters of one position start this far apart, shared out.
    front_rate: f32,
    burn_rate: f32,
}

const FIRE_ART: RingArt = RingArt {
    front: "effects/fire_ring_front.particle.ron",
    burn: "effects/fire_ring_burn.particle.ron",
    looks: &["fire_ring"],
    per_position: 1,
    front_rate: 0.03,
    burn_rate: 0.1,
};

const ICE_ART: RingArt = RingArt {
    front: "effects/ice_ring_front.particle.ron",
    burn: "effects/ice_ring_burn.particle.ron",
    looks: &["ice_ring1", "ice_ring2", "ice_ring3", "ice_ring4"],
    per_position: 2,
    front_rate: 0.06,
    burn_rate: 0.2,
};

fn art(weapon: WeaponKind) -> &'static RingArt {
    match weapon {
        WeaponKind::IceRing => &ICE_ART,
        _ => &FIRE_ART,
    }
}

pub struct RingPlugin;

impl Plugin for RingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_looks)
            .add_systems(
                Update,
                (cast_rings, update_rings, ice_ring::update_ice_rings)
                    .chain()
                    .in_set(WeaponSystems::Effect),
            )
            .add_systems(
                Update,
                (spawn_front, spawn_burners, move_front, end_burners)
                    .chain()
                    .after(WeaponSystems::Effect),
            )
            .add_observer(ice_ring::release_stunned);
    }
}

/// A ring in the world. `contacts` remembers what a fire ring has done to each enemy it touched.
#[derive(Component)]
pub struct Ring {
    pub(super) weapon: WeaponKind,
    pub(super) centre: Vec2,
    pub(super) radius: f32,
    pub(super) age: f32,
    pub(super) shape: RingShape,
    pub(super) values: WeaponValues,
    contacts: HashMap<Entity, Contact>,
    burn: Option<Timer>,
}

/// On a ring from the moment it is nearly at the maximum radius, so the look places its burners.
#[derive(Component)]
struct Burning;

/// An emitter on the growing front, at `angle` on the circle.
#[derive(Component)]
struct Front {
    owner: Entity,
    angle: f32,
}

/// An emitter on the burning ring. When the ring is gone it stops and removes itself.
#[derive(Component)]
struct Burner(Entity);

/// The fire ring's damaging band: inner and outer radius. While the front grows it reaches from the radius
/// `trail_secs` ago to the radius now, and while the ring burns it is the band of the burners.
fn band(shape: &RingShape, age: f32, radius: f32) -> (f32, f32) {
    if radius >= shape.max_radius - shape.burn_lead {
        (
            shape.max_radius - shape.band_half,
            shape.max_radius + shape.band_half,
        )
    } else {
        (
            ring_radius(age - shape.trail_secs, shape.grow_secs, shape.max_radius),
            radius + 4.0,
        )
    }
}

pub(super) fn cast_rings(
    mut cast: MessageReader<CastWeapon>,
    world: Res<CollisionWorld>,
    bounds: Option<Res<CameraBounds>>,
    mut commands: Commands,
) {
    for order in cast.read() {
        let Some(shape) = ring_shape(order.weapon) else {
            continue;
        };
        info!(
            "ring: {} started at ({:.0}, {:.0}), max radius {:.0}",
            order.weapon.label(),
            order.centre.x,
            order.centre.y,
            shape.max_radius
        );
        let mut ring = commands.spawn(Ring {
            weapon: order.weapon,
            centre: order.centre,
            radius: 0.0,
            age: 0.0,
            shape,
            values: values(order.weapon),
            contacts: HashMap::new(),
            burn: None,
        });
        if order.weapon == WeaponKind::IceRing {
            ring.insert(IceRing::new(
                order.centre,
                shape.max_radius,
                world.edges(),
                bounds.as_ref().map(|bounds| bounds.0),
            ));
        }
    }
}

/// Grows every ring and ends it when its time at full size is over. For the fire ring it also hurts and pushes the
/// enemies in its band. A dead enemy is skipped, even while it still has its `HitBox` (`weapons.md`, "Dead enemies").
pub(super) fn update_rings(
    time: Res<Time>,
    mut rings: Query<(Entity, &mut Ring)>,
    enemies: Query<(Entity, &Transform, &HitBox, &EnemyHp)>,
    mut hits: MessageWriter<WeaponHit>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (entity, mut ring) in &mut rings {
        let shape = ring.shape;
        let before = ring.radius;
        ring.age += dt;
        ring.radius = ring_radius(ring.age, shape.grow_secs, shape.max_radius);
        let advance = ring.radius - before;

        if ring.burn.is_none() && ring.radius >= shape.max_radius - shape.burn_lead {
            ring.burn = Some(Timer::from_seconds(shape.burn_secs, TimerMode::Once));
            commands.entity(entity).insert(Burning);
        }

        if ring.weapon == WeaponKind::FireRing {
            let (inner, outer) = band(&shape, ring.age, ring.radius);
            let (centre, values, weapon) = (ring.centre, ring.values, ring.weapon);
            for (target, transform, hit_box, hp) in &enemies {
                if hp.is_dead() {
                    continue;
                }
                let rect = hit_box.rect_at(transform.translation.truncate());
                if !rect_overlaps_band(centre, inner, outer, rect) {
                    continue;
                }
                let contact = ring.contacts.entry(target).or_default();
                let amount = contact.damage(&values, dt);
                let away = (rect.center() - centre).try_normalize().unwrap_or(Vec2::X);
                let push = contact.push(values.pushback, away * advance);
                if amount.is_some() || push != Vec2::ZERO {
                    hits.write(WeaponHit {
                        target,
                        weapon,
                        amount: amount.unwrap_or(0.0),
                        push,
                    });
                }
            }
        }

        // The burn time only runs once the ring is at the maximum radius, so the whole burn is seen.
        if ring.radius >= shape.max_radius
            && let Some(burn) = ring.burn.as_mut()
            && burn.tick(time.delta()).is_finished()
        {
            let touched: Vec<String> = ring
                .contacts
                .iter()
                .map(|(target, contact)| {
                    format!(
                        "{target:?} {} ticks, pushed {:.0} px",
                        contact.ticks(),
                        contact.pushed()
                    )
                })
                .collect();
            // The ice ring reports its bodies when its `IceRing` is removed (`ice_ring::release_stunned`).
            if ring.weapon == WeaponKind::FireRing {
                info!(
                    "ring: {} ended at age {:.2} s, touched {}: [{}]",
                    ring.weapon.label(),
                    ring.age,
                    touched.len(),
                    touched.join("; ")
                );
            } else {
                info!(
                    "ring: {} ended at age {:.2} s",
                    ring.weapon.label(),
                    ring.age
                );
            }
            commands.entity(entity).despawn();
        }
    }
}

fn load_looks(
    server: Res<AssetServer>,
    mut looks: ResMut<FxLooks>,
    mut library: ResMut<EffectLibrary>,
) {
    for art in [&FIRE_ART, &ICE_ART] {
        for name in art.looks {
            looks.request(&server, name, ART, name, true);
        }
        library.keep(&server, art.front);
        library.keep(&server, art.burn);
    }
}

/// A scrambled number for position `index` of `owner`. `salt` gives each use its own number. No random crate needed.
fn roll(owner: Entity, index: usize, salt: u32) -> u32 {
    let mut x = (owner.to_bits() as u32 ^ (index as u32 + 1).wrapping_mul(0x9E37_79B1))
        .wrapping_add(salt.wrapping_mul(0x85EB_CA6B));
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^ (x >> 15)
}

/// The look of emitter `k` at position `index`: one of the ring's looks, a different one for each emitter of a
/// position. The sprite strip, or flat squares until it has loaded.
fn pick(art: &RingArt, looks: &FxLooks, owner: Entity, index: usize, salt: u32, k: usize) -> Look {
    let count = art.looks.len() as u32;
    let chosen = if count == 1 {
        0
    } else {
        let first = roll(owner, index, salt);
        let step = 1 + roll(owner, index, salt + 10) % (count - 1);
        (first.wrapping_add(k as u32 * step) % count) as usize
    };
    looks.get(art.looks[chosen]).cloned().unwrap_or_default()
}

/// The spawn state of emitter `k` of `count` at one position: its timer starts at its share of the spawn period.
fn stagger(k: usize, count: usize, rate: f32) -> ParticleSpawnerState {
    let mut state = ParticleSpawnerState::default();
    state
        .timer
        .set_elapsed(Duration::from_secs_f32(rate * k as f32 / count as f32));
    state
}

/// How far a ring reaches at `angle`: its maximum radius, or for the ice ring the reach of that direction.
fn reach(ring: &Ring, ice: Option<&IceRing>, angle: f32) -> f32 {
    ice.map_or(ring.shape.max_radius, |ice| ice.limit_at(angle))
}

/// Places the front emitters of a new ring on the circle (at radius 0, so all at the centre).
fn spawn_front(new: Query<(Entity, &Ring), Added<Ring>>, looks: Res<FxLooks>, mut fx: Fx) {
    for (owner, ring) in &new {
        let art = art(ring.weapon);
        for i in 0..ring.shape.front_emitters {
            let angle = TAU * i as f32 / ring.shape.front_emitters as f32;
            for k in 0..art.per_position {
                let look = pick(art, &looks, owner, i, 1, k);
                let emitter = fx.emitter(
                    art.front,
                    Transform::from_translation(ring.centre.extend(0.0)),
                    &look,
                    ring.values.glow,
                );
                fx.commands.entity(emitter).insert((
                    Front { owner, angle },
                    DepthSort { feet_offset: 0.0 },
                    stagger(k, art.per_position, art.front_rate),
                ));
            }
        }
    }
}

/// Places the burners on the circle when the ring starts to burn, each at the reach of its direction.
fn spawn_burners(
    new: Query<(Entity, &Ring, Option<&IceRing>), Added<Burning>>,
    looks: Res<FxLooks>,
    mut fx: Fx,
) {
    for (owner, ring, ice) in &new {
        let art = art(ring.weapon);
        for i in 0..ring.shape.burners {
            let angle = TAU * i as f32 / ring.shape.burners as f32;
            let at = ring.centre + Vec2::from_angle(angle) * reach(ring, ice, angle);
            for k in 0..art.per_position {
                let look = pick(art, &looks, owner, i, 2, k);
                let emitter = fx.emitter(
                    art.burn,
                    Transform::from_translation(at.extend(0.0)),
                    &look,
                    ring.values.glow,
                );
                fx.commands.entity(emitter).insert((
                    Burner(owner),
                    DepthSort { feet_offset: 0.0 },
                    stagger(k, art.per_position, art.burn_rate),
                ));
            }
        }
    }
}

/// Moves each front emitter to the radius of its ring, or to the reach of its direction when that is shorter. When the
/// ring has grown (or is gone) the emitter stops and removes itself once its last particle is gone.
fn move_front(
    rings: Query<(&Ring, Option<&IceRing>)>,
    mut emitters: Query<(Entity, &Front, &mut Transform, &mut ParticleSpawnerState)>,
    mut commands: Commands,
) {
    for (entity, front, mut transform, mut state) in &mut emitters {
        match rings.get(front.owner) {
            Ok((ring, ice)) if ring.age < ring.shape.grow_secs => {
                let radius = ring.radius.min(reach(ring, ice, front.angle));
                let at = ring.centre + Vec2::from_angle(front.angle) * radius;
                transform.translation.x = at.x;
                transform.translation.y = at.y;
            }
            _ => {
                state.active = false;
                commands
                    .entity(entity)
                    .remove::<Front>()
                    .insert(OneShot::Despawn);
            }
        }
    }
}

fn end_burners(
    rings: Query<(), With<Ring>>,
    mut burners: Query<(Entity, &Burner, &mut ParticleSpawnerState)>,
    mut commands: Commands,
) {
    for (entity, burner, mut state) in &mut burners {
        if !rings.contains(burner.0) {
            state.active = false;
            commands
                .entity(entity)
                .remove::<Burner>()
                .insert(OneShot::Despawn);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapons::values::{FIRE_RING, ICE_RING};

    #[derive(Resource, Default)]
    struct Log(Vec<(f32, WeaponHit)>);

    fn collect(mut hits: MessageReader<WeaponHit>, time: Res<Time>, mut log: ResMut<Log>) {
        for hit in hits.read() {
            log.0.push((time.elapsed_secs(), *hit));
        }
    }

    fn app(dt_ms: u64) -> App {
        let mut app = crate::test_util::timed_app(dt_ms);
        app.add_message::<CastWeapon>()
            .add_message::<WeaponHit>()
            .init_resource::<CollisionWorld>()
            .init_resource::<Log>()
            .add_systems(Update, (cast_rings, update_rings, collect).chain());
        app.update();
        app
    }

    fn cast(app: &mut App, weapon: WeaponKind, centre: Vec2) {
        app.world_mut().write_message(CastWeapon {
            weapon,
            origin: centre,
            centre,
            facing: Vec2::X,
        });
    }

    /// Halfway to the radius where the ring starts to burn: only the growing front reaches a body here.
    const FRONT_ONLY: f32 = (FIRE_RING.max_radius - FIRE_RING.burn_lead) / 2.0;

    /// An enemy whose body is a 10 by 10 box `distance` pixels to the right of the origin.
    fn enemy(app: &mut App, distance: f32) -> Entity {
        app.world_mut()
            .spawn((
                EnemyHp::full(100.0),
                Transform::from_xyz(distance, 0.0, 0.0),
                HitBox {
                    offset: Vec2::ZERO,
                    half_size: Vec2::splat(5.0),
                },
            ))
            .id()
    }

    fn run(app: &mut App, seconds: f32, dt_ms: u64) {
        for _ in 0..(seconds * 1000.0 / dt_ms as f32) as usize {
            app.update();
        }
    }

    fn hits_on(app: &App, target: Entity) -> Vec<(f32, WeaponHit)> {
        app.world()
            .resource::<Log>()
            .0
            .iter()
            .filter(|(_, hit)| hit.target == target)
            .copied()
            .collect()
    }

    fn damage_hits(hits: &[(f32, WeaponHit)]) -> Vec<f32> {
        hits.iter()
            .filter(|(_, hit)| hit.amount > 0.0)
            .map(|(at, _)| *at)
            .collect()
    }

    fn rings(app: &mut App) -> usize {
        app.world_mut().query::<&Ring>().iter(app.world()).count()
    }

    #[test]
    fn a_ring_cast_makes_a_ring_of_its_shape_at_the_centre_and_other_weapons_make_none() {
        let mut app = app(16);
        cast(&mut app, WeaponKind::Fireball, Vec2::ZERO);
        cast(&mut app, WeaponKind::Tornado, Vec2::ZERO);
        app.update();
        assert_eq!(rings(&mut app), 0);
        cast(&mut app, WeaponKind::FireRing, Vec2::new(10.0, 20.0));
        cast(&mut app, WeaponKind::IceRing, Vec2::new(-30.0, 40.0));
        app.update();
        let mut made: Vec<(WeaponKind, Vec2, f32, bool)> = app
            .world_mut()
            .query::<(&Ring, Has<IceRing>)>()
            .iter(app.world())
            .map(|(ring, ice)| (ring.weapon, ring.centre, ring.shape.max_radius, ice))
            .collect();
        made.sort_by_key(|(weapon, ..)| *weapon == WeaponKind::IceRing);
        assert_eq!(
            made,
            vec![
                (
                    WeaponKind::FireRing,
                    Vec2::new(10.0, 20.0),
                    FIRE_RING.max_radius,
                    false
                ),
                (
                    WeaponKind::IceRing,
                    Vec2::new(-30.0, 40.0),
                    ICE_RING.max_radius,
                    true
                ),
            ]
        );
    }

    #[test]
    fn a_body_the_front_passes_is_hurt_once_and_a_body_out_of_reach_never() {
        let mut app = app(16);
        let near = enemy(&mut app, FRONT_ONLY);
        let far = enemy(&mut app, FIRE_RING.max_radius + FIRE_RING.band_half + 40.0);
        cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
        run(&mut app, 4.0, 16);
        let near_hits = damage_hits(&hits_on(&app, near));
        assert_eq!(near_hits.len(), 1, "{near_hits:?}");
        assert_eq!(hits_on(&app, near)[0].1.amount, 100.0);
        assert!(hits_on(&app, far).is_empty());
    }

    #[test]
    fn a_body_in_the_burn_band_is_hurt_again_every_tick_until_the_ring_ends() {
        let mut app = app(16);
        let burning = enemy(&mut app, FIRE_RING.max_radius);
        cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
        // The burn time starts when the ring is full, so the ring is still there just before grow plus burn, and
        // gone just after.
        let end = FIRE_RING.grow_secs + FIRE_RING.burn_secs;
        run(&mut app, end - 0.05, 16);
        assert_eq!(rings(&mut app), 1);
        run(&mut app, 0.1, 16);
        assert_eq!(rings(&mut app), 0);
        run(&mut app, 2.0, 16);
        let times = damage_hits(&hits_on(&app, burning));
        // The first hit is at about 0.34 s, when the burn band starts. The ring ends after grow 0.77 s plus burn 2 s,
        // so ticks 0.5 s apart fit 5 times. A tick lands in the first 16 ms frame at or after its time.
        assert_eq!(times.len(), 5, "{times:?}");
        for pair in times.windows(2) {
            assert!((pair[1] - pair[0] - 0.5).abs() <= 0.016, "{times:?}");
        }
        assert_eq!(rings(&mut app), 0);
    }

    #[test]
    fn no_body_is_skipped_at_a_low_frame_rate() {
        for dt_ms in [16, 50, 100] {
            let mut app = app(dt_ms);
            let bodies: Vec<Entity> = [0.2, 0.5, 0.8, 1.2]
                .map(|share| enemy(&mut app, share * FIRE_RING.max_radius))
                .to_vec();
            cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
            run(&mut app, 4.0, dt_ms);
            for body in bodies {
                assert!(
                    !damage_hits(&hits_on(&app, body)).is_empty(),
                    "body skipped at {dt_ms} ms frames"
                );
            }
        }
    }

    #[test]
    fn pushback_carries_a_body_outward_by_at_most_its_limit() {
        // The fire ring does not push (b1), so the cast ring is given a pixel limit that the front passes: from
        // `FRONT_ONLY` the front would carry the body more than 20 px.
        let limit = 20.0;
        let mut app = app(16);
        let body = enemy(&mut app, FRONT_ONLY);
        cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
        app.update();
        for mut ring in app
            .world_mut()
            .query::<&mut Ring>()
            .iter_mut(app.world_mut())
        {
            ring.values.pushback = super::super::values::Pushback::Pixels(limit);
        }
        run(&mut app, 4.0, 16);
        let pushes: Vec<Vec2> = hits_on(&app, body).iter().map(|(_, h)| h.push).collect();
        let total: Vec2 = pushes.iter().sum();
        assert!(
            (total.x - limit).abs() < 1e-3 && total.y.abs() < 1e-3,
            "{total}"
        );
        assert!(pushes.iter().all(|p| p.x >= 0.0));
    }

    /// A living enemy where the front passes, so a test of a body that is not hit shows that the ring did reach it.
    fn living_beside(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                EnemyHp::full(100.0),
                Transform::from_xyz(0.0, FRONT_ONLY, 0.0),
                HitBox {
                    offset: Vec2::ZERO,
                    half_size: Vec2::splat(5.0),
                },
            ))
            .id()
    }

    #[test]
    fn a_body_with_a_hit_box_but_no_hp_is_not_hit() {
        let mut app = app(16);
        let living = living_beside(&mut app);
        let body = app
            .world_mut()
            .spawn((
                Transform::from_xyz(FRONT_ONLY, 0.0, 0.0),
                HitBox {
                    offset: Vec2::ZERO,
                    half_size: Vec2::splat(5.0),
                },
            ))
            .id();
        cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
        run(&mut app, 4.0, 16);
        assert_eq!(damage_hits(&hits_on(&app, living)).len(), 1);
        assert!(hits_on(&app, body).is_empty());
    }

    #[test]
    fn a_dead_enemy_that_still_has_its_hit_box_is_not_hit() {
        // Dead enemies (`weapons.md`): the ring does not rely on the enemy's plugin removing the `HitBox` while it plays `Die`.
        let mut app = app(16);
        let living = living_beside(&mut app);
        let dead = enemy(&mut app, FRONT_ONLY);
        app.world_mut().get_mut::<EnemyHp>(dead).unwrap().current = 0.0;
        cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
        run(&mut app, 4.0, 16);
        assert_eq!(damage_hits(&hits_on(&app, living)).len(), 1);
        assert!(hits_on(&app, dead).is_empty());
    }

    #[test]
    fn the_ring_starts_to_burn_at_the_lead_radius_and_not_before() {
        let mut app = app(16);
        cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
        let lead = FIRE_RING.max_radius - FIRE_RING.burn_lead;
        let mut before = 0.0;
        for _ in 0..100 {
            app.update();
            let (radius, burning) = app
                .world_mut()
                .query::<(&Ring, Has<Burning>)>()
                .single(app.world())
                .map(|(ring, burning)| (ring.radius, burning))
                .unwrap();
            if burning {
                assert!(before < lead, "{before} was already at {lead}");
                assert!(radius >= lead, "{radius} is below {lead}");
                return;
            }
            before = radius;
        }
        panic!("the ring never started to burn");
    }

    #[test]
    fn the_damaging_band_matches_the_trail_while_growing_and_the_burners_when_full() {
        let trail = ring_radius(
            0.3 - FIRE_RING.trail_secs,
            FIRE_RING.grow_secs,
            FIRE_RING.max_radius,
        );
        let radius = ring_radius(0.3, FIRE_RING.grow_secs, FIRE_RING.max_radius);
        assert_eq!(band(&FIRE_RING, 0.3, radius), (trail, radius + 4.0));
        // The band changes at the lead radius: just below it is the trail, at it the band of the burners.
        let lead = FIRE_RING.max_radius - FIRE_RING.burn_lead;
        let full = (
            FIRE_RING.max_radius - FIRE_RING.band_half,
            FIRE_RING.max_radius + FIRE_RING.band_half,
        );
        assert_eq!(band(&FIRE_RING, 0.3, lead - 0.5), (trail, lead - 0.5 + 4.0));
        assert_eq!(band(&FIRE_RING, 0.3, lead), full);
        assert_eq!(band(&FIRE_RING, 1.0, FIRE_RING.max_radius), full);
    }
}
