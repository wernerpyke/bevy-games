//! The fire ring (W1, `PLAN-weapons.md`): a ring that grows from the player's body to a maximum radius, stays there
//! and burns for a while, then goes. It does not follow the player.
//!
//! Gameplay is plain and has no particles: `Ring` holds the radius and the age, and `update_rings` tests every enemy
//! against the damaging band (`game_kit::area`) and sends `WeaponHit`. While the front grows it carries a hit enemy
//! outward at the front's speed, as far as the weapon's pushback allows. The look reads the same `Ring`:
//! - Growing: `front_emitters` emitters on the circle, moved outward every frame to the eased radius (`move_front`).
//!   Their particles stay where they are born, so the band behind the front is the trail. The damaging band reaches
//!   back `trail_secs` in time to match it.
//! - Burning: when the ring is nearly at the maximum radius, `burners` small emitters are placed on the circle and
//!   shimmer until the burn time is over. Enoki cannot hold particles on a ring by itself, so the ring is made of
//!   emitters.
//!
//! Features do not stop the ring and do not react to it (`PLAN-weapons.md` D10); a pushed enemy stops at them.

use std::collections::HashMap;
use std::f32::consts::TAU;

use bevy::prelude::*;
use game_kit::area::{rect_overlaps_band, ring_radius};
use game_kit::depth::DepthSort;
use game_kit::fx::{EffectLibrary, Fx, FxLooks, Look, OneShot, ParticleSpawnerState};
use game_kit::hitbox::HitBox;

use super::damage::Contact;
use super::values::{RING, RingShape, WeaponValues, values};
use super::{CastWeapon, WeaponHit};
use crate::enemy_hp::EnemyHp;
use crate::weapon_menu::WeaponKind;

const FRONT_EFFECT: &str = "effects/fire_ring_front.particle.ron";
const BURN_EFFECT: &str = "effects/fire_ring_burn.particle.ron";
const ART: &str = "art/weapons/fireball_and_rings.aseprite";
const LOOK: &str = "fire_ring";

pub struct RingPlugin;

impl Plugin for RingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_look).add_systems(
            Update,
            (
                cast_rings,
                update_rings,
                spawn_front,
                spawn_burners,
                move_front,
                end_burners,
            )
                .chain(),
        );
    }
}

/// A ring in the world. `contacts` remembers what the ring has done to each enemy it touched.
#[derive(Component)]
pub struct Ring {
    weapon: WeaponKind,
    centre: Vec2,
    radius: f32,
    age: f32,
    values: WeaponValues,
    contacts: HashMap<Entity, Contact>,
    burn: Option<Timer>,
}

/// On a ring from the moment it is at the maximum radius, so the look places its burners.
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

/// The damaging band: inner and outer radius. While the front grows it reaches from the radius `trail_secs` ago to the
/// radius now, and while the ring burns it is the band of the burners.
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

fn cast_rings(mut cast: MessageReader<CastWeapon>, mut commands: Commands) {
    for order in cast.read() {
        if order.weapon != WeaponKind::FireRing {
            continue;
        }
        info!(
            "ring: started at ({:.0}, {:.0}), max radius {:.0}",
            order.centre.x, order.centre.y, RING.max_radius
        );
        commands.spawn(Ring {
            weapon: order.weapon,
            centre: order.centre,
            radius: 0.0,
            age: 0.0,
            values: values(order.weapon),
            contacts: HashMap::new(),
            burn: None,
        });
    }
}

/// Grows the ring, hurts and pushes the enemies in its band, and ends it when the burn is over.
fn update_rings(
    time: Res<Time>,
    mut rings: Query<(Entity, &mut Ring)>,
    enemies: Query<(Entity, &Transform, &HitBox), With<EnemyHp>>,
    mut hits: MessageWriter<WeaponHit>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (entity, mut ring) in &mut rings {
        let before = ring.radius;
        ring.age += dt;
        ring.radius = ring_radius(ring.age, RING.grow_secs, RING.max_radius);
        let advance = ring.radius - before;

        if ring.burn.is_none() && ring.radius >= RING.max_radius - RING.burn_lead {
            ring.burn = Some(Timer::from_seconds(RING.burn_secs, TimerMode::Once));
            commands.entity(entity).insert(Burning);
        }

        let (inner, outer) = band(&RING, ring.age, ring.radius);
        let (centre, values, weapon) = (ring.centre, ring.values, ring.weapon);
        for (target, transform, hit_box) in &enemies {
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

        // The burn time only runs once the ring is at the maximum radius, so the whole burn is seen.
        if ring.radius >= RING.max_radius
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
            info!(
                "ring: ended at age {:.2} s, touched {}: [{}]",
                ring.age,
                touched.len(),
                touched.join("; ")
            );
            commands.entity(entity).despawn();
        }
    }
}

fn load_look(
    server: Res<AssetServer>,
    mut looks: ResMut<FxLooks>,
    mut library: ResMut<EffectLibrary>,
) {
    looks.request(&server, LOOK, ART, LOOK, true);
    library.keep(&server, FRONT_EFFECT);
    library.keep(&server, BURN_EFFECT);
}

/// The look of the ring particles: the sprite strip, or flat squares until it has loaded.
fn look(looks: &FxLooks) -> Look {
    looks.get(LOOK).cloned().unwrap_or_default()
}

/// Places the front emitters of a new ring on the circle (at radius 0, so all at the centre).
fn spawn_front(new: Query<(Entity, &Ring), Added<Ring>>, looks: Res<FxLooks>, mut fx: Fx) {
    let look = look(&looks);
    for (owner, ring) in &new {
        for i in 0..RING.front_emitters {
            let emitter = fx.emitter(
                FRONT_EFFECT,
                Transform::from_translation(ring.centre.extend(0.0)),
                &look,
                ring.values.glow,
            );
            fx.commands.entity(emitter).insert((
                Front {
                    owner,
                    angle: TAU * i as f32 / RING.front_emitters as f32,
                },
                DepthSort { feet_offset: 0.0 },
            ));
        }
    }
}

/// Places the burners on the full circle when the ring starts to burn.
fn spawn_burners(new: Query<(Entity, &Ring), Added<Burning>>, looks: Res<FxLooks>, mut fx: Fx) {
    let look = look(&looks);
    for (owner, ring) in &new {
        for i in 0..RING.burners {
            let angle = TAU * i as f32 / RING.burners as f32;
            let at = ring.centre + Vec2::from_angle(angle) * RING.max_radius;
            let emitter = fx.emitter(
                BURN_EFFECT,
                Transform::from_translation(at.extend(0.0)),
                &look,
                ring.values.glow,
            );
            fx.commands
                .entity(emitter)
                .insert((Burner(owner), DepthSort { feet_offset: 0.0 }));
        }
    }
}

/// Moves each front emitter to the radius of its ring. When the ring has grown (or is gone) the emitter stops and
/// removes itself once its last particle is gone.
fn move_front(
    rings: Query<&Ring>,
    mut emitters: Query<(Entity, &Front, &mut Transform, &mut ParticleSpawnerState)>,
    mut commands: Commands,
) {
    for (entity, front, mut transform, mut state) in &mut emitters {
        match rings.get(front.owner) {
            Ok(ring) if ring.age < RING.grow_secs => {
                let at = ring.centre + Vec2::from_angle(front.angle) * ring.radius;
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
    use std::time::Duration;

    #[derive(Resource, Default)]
    struct Log(Vec<(f32, WeaponHit)>);

    fn collect(mut hits: MessageReader<WeaponHit>, time: Res<Time>, mut log: ResMut<Log>) {
        for hit in hits.read() {
            log.0.push((time.elapsed_secs(), *hit));
        }
    }

    fn app(dt_ms: u64) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_millis(dt_ms),
            ))
            .add_message::<CastWeapon>()
            .add_message::<WeaponHit>()
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
    fn a_fire_ring_cast_makes_a_ring_at_the_centre_and_other_weapons_make_none() {
        let mut app = app(16);
        cast(&mut app, WeaponKind::IceRing, Vec2::ZERO);
        cast(&mut app, WeaponKind::Fireball, Vec2::ZERO);
        app.update();
        assert_eq!(rings(&mut app), 0);
        cast(&mut app, WeaponKind::FireRing, Vec2::new(10.0, 20.0));
        app.update();
        assert_eq!(rings(&mut app), 1);
        let ring = app
            .world_mut()
            .query::<&Ring>()
            .single(app.world())
            .unwrap();
        assert_eq!(ring.centre, Vec2::new(10.0, 20.0));
    }

    #[test]
    fn a_body_the_front_passes_is_hurt_once_and_a_body_out_of_reach_never() {
        let mut app = app(16);
        let near = enemy(&mut app, 100.0);
        let far = enemy(&mut app, RING.max_radius + RING.band_half + 40.0);
        cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
        run(&mut app, 4.0, 16);
        let near_hits = damage_hits(&hits_on(&app, near));
        assert_eq!(near_hits.len(), 1, "{near_hits:?}");
        assert_eq!(hits_on(&app, near)[0].1.amount, 20.0);
        assert!(hits_on(&app, far).is_empty());
    }

    #[test]
    fn a_body_in_the_burn_band_is_hurt_again_every_tick_until_the_ring_ends() {
        let mut app = app(16);
        let burning = enemy(&mut app, RING.max_radius);
        cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
        run(&mut app, 5.0, 16);
        let times = damage_hits(&hits_on(&app, burning));
        assert!(times.len() >= 4, "{times:?}");
        for pair in times.windows(2) {
            assert!(pair[1] - pair[0] >= 0.49, "{times:?}");
        }
        // The ring ended: nothing after its burn (grow 0.77 s plus burn 2 s).
        assert!(*times.last().unwrap() < 3.0, "{times:?}");
        assert_eq!(rings(&mut app), 0);
    }

    #[test]
    fn no_body_is_skipped_at_a_low_frame_rate() {
        for dt_ms in [16, 50, 100] {
            let mut app = app(dt_ms);
            let bodies: Vec<Entity> = [30.0, 90.0, 150.0, 190.0]
                .map(|d| enemy(&mut app, d))
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
        let mut app = app(16);
        let body = enemy(&mut app, 100.0);
        cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
        run(&mut app, 4.0, 16);
        let pushes: Vec<Vec2> = hits_on(&app, body).iter().map(|(_, h)| h.push).collect();
        let total: Vec2 = pushes.iter().sum();
        let limit = match values(WeaponKind::FireRing).pushback {
            super::super::values::Pushback::Pixels(n) => n,
            other => panic!("the fire ring should push by pixels, not {other:?}"),
        };
        assert!(total.x > 0.0 && total.y.abs() < 1e-3, "{total}");
        assert!(total.length() <= limit + 1e-3, "{total} over {limit}");
        assert!(pushes.iter().all(|p| p.x >= 0.0));
    }

    #[test]
    fn a_body_that_is_already_dead_or_has_no_hit_box_is_not_hit() {
        let mut app = app(16);
        let plain = app
            .world_mut()
            .spawn(Transform::from_xyz(100.0, 0.0, 0.0))
            .id();
        cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
        run(&mut app, 4.0, 16);
        assert!(hits_on(&app, plain).is_empty());
    }

    #[test]
    fn the_ring_starts_to_burn_at_the_lead_radius_and_not_before() {
        let mut app = app(16);
        cast(&mut app, WeaponKind::FireRing, Vec2::ZERO);
        app.update();
        app.update();
        assert!(
            app.world_mut()
                .query::<&Burning>()
                .iter(app.world())
                .count()
                == 0
        );
        run(&mut app, 1.0, 16);
        assert!(
            app.world_mut()
                .query::<&Burning>()
                .iter(app.world())
                .count()
                == 1
        );
    }

    #[test]
    fn the_damaging_band_matches_the_trail_while_growing_and_the_burners_when_full() {
        let growing = band(
            &RING,
            0.3,
            ring_radius(0.3, RING.grow_secs, RING.max_radius),
        );
        assert!(
            growing.0 < growing.1 && growing.1 < RING.max_radius - RING.burn_lead + 4.0 + 100.0
        );
        let full = band(&RING, 1.0, RING.max_radius);
        assert_eq!(
            full,
            (
                RING.max_radius - RING.band_half,
                RING.max_radius + RING.band_half
            )
        );
    }
}
