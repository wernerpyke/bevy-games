//! Ring of fire or ice: a ring that grows from the player to a maximum radius, stays there and burns for a while,
//! then disappears. Step 5 of the spike.
//!
//! Gameplay is plain: the radius is `SPEED * time` up to `MAX_RADIUS`, then it stays. The ring is a band (`band`).
//! A body that overlaps the band is hurt, and again every `HURT_INTERVAL` seconds while it stays in it, so a body that
//! walks into the band later is hurt too (`rules::rect_overlaps_band`). After `BURN_SECS` the ring ends.
//! The look is made of upward triangles: flames (`wall_fire`) or ice shards (`wall_ice`), the same sprites as the wall.
//! - Growing: one point emitter at the player. Every particle is shot out at the speed the ring grows, so it stays on
//!   the ring front by construction.
//! - Burning: when the ring is nearly at the maximum radius, `BURNERS` small emitters are placed on the circle.
//!   They shimmer until the burn time is over. Particles cannot be held on a ring by Enoki itself (`Circle`
//!   emission fills a disc, and damping depends on the life fraction), so the ring is made of emitters.
//!
//! Fire and ice use the same effect files and differ in the sprite sheet. I toggles the element (env `SPIKE_ICE` starts
//! with ice, for scripted screenshots).

use crate::fireball::{FLASH_SECS, HitFlash};
use crate::glow::{AdditiveSprite, Glow, SpriteLook};
use crate::harness::{Body, EffectLibrary, Enemy, FireEffect};
use crate::rules::rect_overlaps_band;
use bevy::prelude::*;
use bevy_enoki::prelude::*;
use std::f32::consts::TAU;

/// Indexes of the effects in `harness::EFFECTS`.
const FIRE_RING: usize = 2;
const ICE_RING: usize = 6;

/// Radius growth in art pixels per second. The front effect file has the same speed (`linear_speed`).
const SPEED: f32 = 300.0;
/// The ring stops growing here. The front effect file has `lifetime` = `MAX_RADIUS / SPEED`.
/// 230 keeps the whole ring inside the view (half height 256), so the stand-ins at x = ±410 are out of reach.
const MAX_RADIUS: f32 = 230.0;
/// The ring burns for this long once it is at the maximum radius.
const BURN_SECS: f32 = 2.0;
/// The burners start this many art pixels before the maximum radius, so the growing front hands over to them.
const BURN_LEAD: f32 = 40.0;
/// Number of burner emitters on the ring. Spacing is `TAU * MAX_RADIUS / BURNERS` = 36 px.
const BURNERS: usize = 40;
/// The damaging band is this wide on each side of the maximum radius while the ring burns (the burners spread 20 px).
const BAND_HALF: f32 = 24.0;
/// A body that stays in the band is hurt again after this many seconds.
const HURT_INTERVAL: f32 = 0.5;
/// The growing emitters spawn for this long, which gives the front its thickness (`SPEED * PULSE_SECS` art pixels).
const PULSE_SECS: f32 = 0.12;

const FRONT: (&str, f32) = ("effects/ring_front.particle.ron", 2.1);
const BURN: &str = "effects/ring_burn.particle.ron";
const Z_BURN: f32 = 2.2;
/// Square frames side by side (fire 6, ice 4): flames flicker, shards grow, over the life of a particle.
const FIRE_SPRITE: &str = "generated/wall_fire.png";
const ICE_SPRITE: &str = "generated/wall_ice.png";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Element {
    Fire,
    Ice,
}

#[derive(Resource)]
pub struct RingStyle {
    fire: SpriteLook,
    ice: SpriteLook,
}

/// The ring. `hurt` holds the age of the ring at the last hit of each body. `burn` runs while the ring is at the maximum radius.
#[derive(Component)]
struct Ring {
    centre: Vec2,
    radius: f32,
    age: f32,
    hurt: Vec<(Entity, f32)>,
    burn: Option<Timer>,
    element: Element,
}

/// A growing emitter. After `PULSE_SECS` it stops and removes itself once its last particle is gone.
#[derive(Component)]
struct Pulse(Timer);

/// An emitter on the burning ring. When the ring is gone it stops and removes itself like a `Pulse`.
#[derive(Component)]
struct Burner(Entity);

pub struct RingPlugin;

impl Plugin for RingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_style).add_systems(
            Update,
            (launch, grow_rings, end_pulses, end_burners).chain(),
        );
    }
}

fn load_style(
    server: Res<AssetServer>,
    mut normals: ResMut<Assets<SpriteParticle2dMaterial>>,
    mut adds: ResMut<Assets<AdditiveSprite>>,
    mut library: ResMut<EffectLibrary>,
    mut commands: Commands,
) {
    library.0.push(server.load(FRONT.0));
    library.0.push(server.load(BURN));
    commands.insert_resource(RingStyle {
        fire: SpriteLook::new(&mut normals, &mut adds, server.load(FIRE_SPRITE), 6),
        ice: SpriteLook::new(&mut normals, &mut adds, server.load(ICE_SPRITE), 4),
    });
}

pub fn look(style: &RingStyle, element: Element) -> &SpriteLook {
    match element {
        Element::Fire => &style.fire,
        Element::Ice => &style.ice,
    }
}

fn launch(
    mut fired: MessageReader<FireEffect>,
    style: Res<RingStyle>,
    glow: Res<Glow>,
    server: Res<AssetServer>,
    mut commands: Commands,
) {
    for shot in fired.read() {
        let element = match shot.effect {
            FIRE_RING => Element::Fire,
            ICE_RING => Element::Ice,
            _ => continue,
        };
        commands.spawn(Ring {
            centre: shot.from,
            radius: 0.0,
            age: 0.0,
            hurt: Vec::new(),
            burn: None,
            element,
        });
        let mut front = commands.spawn((
            ParticleEffectHandle(server.load(FRONT.0)),
            Transform::from_translation(shot.from.extend(FRONT.1)),
            Pulse(Timer::from_seconds(PULSE_SECS, TimerMode::Once)),
        ));
        look(&style, element).insert(&glow, &mut front);
    }
}

/// The band that hurts: inner and outer radius. While the front grows it is the thickness of the front, and while the ring
/// burns it is the band of the burners.
fn band(radius: f32) -> (f32, f32) {
    if radius >= MAX_RADIUS - BURN_LEAD {
        (MAX_RADIUS - BAND_HALF, MAX_RADIUS + BAND_HALF)
    } else {
        ((radius - SPEED * PULSE_SECS).max(0.0), radius + 4.0)
    }
}

fn grow_rings(
    time: Res<Time>,
    style: Res<RingStyle>,
    glow: Res<Glow>,
    server: Res<AssetServer>,
    mut rings: Query<(Entity, &mut Ring)>,
    bodies: Query<(Entity, &Body, Has<Enemy>)>,
    mut commands: Commands,
) {
    for (entity, mut ring) in &mut rings {
        ring.age += time.delta_secs();
        ring.radius = (ring.radius + SPEED * time.delta_secs()).min(MAX_RADIUS);

        if ring.burn.is_none() && ring.radius >= MAX_RADIUS - BURN_LEAD {
            ring.burn = Some(Timer::from_seconds(BURN_SECS, TimerMode::Once));
            for i in 0..BURNERS {
                let angle = TAU * i as f32 / BURNERS as f32;
                let position = ring.centre + Vec2::from_angle(angle) * MAX_RADIUS;
                let mut burner = commands.spawn((
                    ParticleEffectHandle(server.load(BURN)),
                    Transform::from_translation(position.extend(Z_BURN)),
                    Burner(entity),
                ));
                look(&style, ring.element).insert(&glow, &mut burner);
            }
        }

        let (inner, outer) = band(ring.radius);
        for (body_entity, body, is_enemy) in &bodies {
            if !rect_overlaps_band(ring.centre, inner, outer, body.0) {
                continue;
            }
            let age = ring.age;
            let last = ring
                .hurt
                .iter_mut()
                .find(|(entity, _)| *entity == body_entity);
            match last {
                Some((_, at)) if age - *at < HURT_INTERVAL => continue,
                Some((_, at)) => *at = age,
                None => ring.hurt.push((body_entity, age)),
            }
            if is_enemy {
                info!("ring hurt the enemy at age {age:.2} s");
                commands
                    .entity(body_entity)
                    .insert(HitFlash(Timer::from_seconds(FLASH_SECS, TimerMode::Once)));
            }
        }

        // The burn time only runs once the ring is at the maximum radius, so the whole burn is seen.
        if ring.radius >= MAX_RADIUS
            && let Some(burn) = ring.burn.as_mut()
            && burn.tick(time.delta()).is_finished()
        {
            commands.entity(entity).despawn();
        }
    }
}

fn end_pulses(
    time: Res<Time>,
    mut emitters: Query<(Entity, &mut Pulse, &mut ParticleSpawnerState)>,
    mut commands: Commands,
) {
    for (entity, mut pulse, mut state) in &mut emitters {
        if pulse.0.tick(time.delta()).is_finished() {
            state.active = false;
            commands.entity(entity).insert(OneShot::Despawn);
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
