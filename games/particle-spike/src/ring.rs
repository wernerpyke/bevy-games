//! Ring of fire or ice: a ring that grows from the player to a maximum radius, stays there and burns for a while,
//! then disappears. Step 5 of the spike.
//!
//! Gameplay is plain: the radius is eased out (`rules::ring_radius`) up to `MAX_RADIUS` over `GROW_SECS`, then it stays. The ring is a band (`band`).
//! A body that overlaps the band is hurt, and again every `HURT_INTERVAL` seconds while it stays in it, so a body that
//! walks into the band later is hurt too (`rules::rect_overlaps_band`). After `BURN_SECS` the ring ends.
//! The look is made of upward triangles: flames (`wall_fire`) or ice shards (`wall_ice`), the same sprites as the wall.
//! - Growing: `FRONT_EMITTERS` emitters on the circle, moved outward every frame to the eased radius (`move_front`). Their
//!   particles stay where they are born, so the band behind the front is the trail, and the front is exact whatever the
//!   easing. The hurt band reaches back `TRAIL_SECS` in time.
//! - Burning: when the ring is nearly at the maximum radius, `BURNERS` small emitters are placed on the circle.
//!   They shimmer until the burn time is over. Particles cannot be held on a ring by Enoki itself (`Circle`
//!   emission fills a disc, and damping depends on the life fraction), so the ring is made of emitters.
//!
//! Fire and ice use the same effect files and differ in the sprite sheet (effects 2 and 6).

use crate::fireball::{FLASH_SECS, HitFlash};
use crate::glow::{AdditiveSprite, Glow, SpriteLook};
use crate::harness::{Body, EffectLibrary, Enemy, FireEffect};
use crate::rules::{rect_overlaps_band, ring_radius};
use bevy::prelude::*;
use bevy_enoki::prelude::*;
use std::f32::consts::TAU;

/// Indexes of the effects in `harness::EFFECTS`.
const FIRE_RING: usize = 2;
const ICE_RING: usize = 6;

/// The ring grows to `MAX_RADIUS` in this many seconds, eased out (`rules::ring_radius`): fast at the start, slowing to a stop.
const GROW_SECS: f32 = 0.767;
/// The ring stops growing here.
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
/// Number of emitters on the growing front. They move outward with the eased radius.
const FRONT_EMITTERS: usize = 40;
/// The damaging band while growing reaches back this far in time: from the radius now to the radius this long ago.
/// It matches the trail the moving emitters leave (the front particle life is a little longer).
const TRAIL_SECS: f32 = 0.12;

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

/// An emitter on the growing front, at `angle` on the circle. It follows the radius of its ring. When the ring has grown
/// (or is gone) it stops and removes itself once its last particle is gone.
#[derive(Component)]
struct Front {
    owner: Entity,
    angle: f32,
}

/// An emitter on the burning ring. When the ring is gone it stops and removes itself like a `Pulse`.
#[derive(Component)]
struct Burner(Entity);

pub struct RingPlugin;

impl Plugin for RingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_style).add_systems(
            Update,
            (launch, grow_rings, move_front, end_burners).chain(),
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
        let owner = commands
            .spawn(Ring {
                centre: shot.from,
                radius: 0.0,
                age: 0.0,
                hurt: Vec::new(),
                burn: None,
                element,
            })
            .id();
        for i in 0..FRONT_EMITTERS {
            let mut front = commands.spawn((
                ParticleEffectHandle(server.load(FRONT.0)),
                Transform::from_translation(shot.from.extend(FRONT.1)),
                Front {
                    owner,
                    angle: TAU * i as f32 / FRONT_EMITTERS as f32,
                },
            ));
            look(&style, element).insert(&glow, &mut front);
        }
    }
}

/// The band that hurts: inner and outer radius. While the front grows it reaches from the radius `TRAIL_SECS` ago to the
/// radius now (the trail of the moving emitters), and while the ring burns it is the band of the burners.
fn band(age: f32, radius: f32) -> (f32, f32) {
    if radius >= MAX_RADIUS - BURN_LEAD {
        (MAX_RADIUS - BAND_HALF, MAX_RADIUS + BAND_HALF)
    } else {
        (
            ring_radius(age - TRAIL_SECS, GROW_SECS, MAX_RADIUS),
            radius + 4.0,
        )
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
        ring.radius = ring_radius(ring.age, GROW_SECS, MAX_RADIUS);

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

        let (inner, outer) = band(ring.age, ring.radius);
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

fn move_front(
    rings: Query<&Ring>,
    mut emitters: Query<(Entity, &Front, &mut Transform, &mut ParticleSpawnerState)>,
    mut commands: Commands,
) {
    for (entity, front, mut transform, mut state) in &mut emitters {
        match rings.get(front.owner) {
            Ok(ring) if ring.age < GROW_SECS => {
                let position = ring.centre + Vec2::from_angle(front.angle) * ring.radius;
                transform.translation = position.extend(FRONT.1);
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
