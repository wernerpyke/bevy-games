//! Water ring (whirlpool): a ring around the player that swirls. Step 7 of the spike, three ways to draw it.
//!
//! Gameplay is plain and the same for all of them: the ring is centred on the player and follows it, and it lasts
//! `WHIRL_SECS`. It is a band (`INNER` to `OUTER`). A body in the band is hurt, and again every `HURT_INTERVAL` seconds
//! while it stays (`rules::rect_overlaps_band`). The shield also pushes the red block straight out of the circle
//! (`PUSH_SPEED`, `rules::push_delta`), and blockers stop it. These are design values for you, not decisions.
//!
//! The look is the attractor one (B) unless env `SPIKE_WHIRL=0..3` picks another. WASD moves the player.
//! - A pivot: 12 emitters on two circles, one clockwise and one counter-clockwise. The droplets stay where they are
//!   born, so each emitter leaves an arc behind it.
//! - A spiral: 12 emitters turn clockwise while their radius shrinks, over and over. Each leaves a spiral thread.
//! - B attractor: 10 emitters on a small circle send droplets along the tangent, faster than a circular orbit. An
//!   attractor at the player (set per emitter, it is an offset from the emitter) bends them, so they spiral outwards
//!   through the band. This is the look for a shield that pushes enemies away.
//! - C sprite: an animated ring (`whirlpool_placeholder.aseprite`, 80 ms frames in the file, looping) with a few spray
//!   emitters on it. The ring fades in, and out at the end, with its alpha.
//!
//! When the whirlpool is gone the emitters stop and remove themselves after their last particle.

use crate::fireball::{FLASH_SECS, HitFlash};
use crate::glow::Glow;
use crate::harness::{Blocker, Body, EffectLibrary, Enemy, FireEffect, MarchTrigger, Player};
use crate::rules::{Rect2, move_rect, push_delta, rect_overlaps_band};
use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use bevy_enoki::prelude::*;
use std::f32::consts::{FRAC_PI_2, TAU};

/// Index of the effect in `harness::EFFECTS`.
const EFFECT: usize = 4;

/// The hurt band around the player, in art pixels.
const INNER: f32 = 70.0;
const OUTER: f32 = 110.0;
const WHIRL_SECS: f32 = 5.0;
const FADE_IN_SECS: f32 = 0.3;
const FADE_OUT_SECS: f32 = 0.5;
const HURT_INTERVAL: f32 = 0.5;
/// The shield pushes the red block straight away from the player at this speed while any part of it is inside `OUTER`.
const PUSH_SPEED: f32 = 220.0;

/// Turning speed of the pivots and of the spray, in radians per second (positive: counter-clockwise).
const SPIN: f32 = 2.4;
const PIVOT_OUTER_RADIUS: f32 = 102.0;
const PIVOT_INNER_RADIUS: f32 = 78.0;
/// One spiral thread goes from the outer to the inner radius in this long.
const SPIRAL_SECS: f32 = 1.6;
const SPIRAL_OUTER: f32 = OUTER + 14.0;
const SPIRAL_INNER: f32 = INNER - 26.0;
/// Radius of the emitters of the attractor variant and the strength of the attractor. A circle of radius R needs the
/// speed sqrt(strength / R): sqrt(1.6e6 / 65) = 157 px/s. The effect file starts at 185, above that, so the droplets
/// spiral outwards: from the inner edge of the band to just past its outer edge in their life (see the test).
const ORBIT_RADIUS: f32 = 65.0;
const ORBIT_STRENGTH: f32 = 1.6e6;
const SPRAY_RADIUS: f32 = 90.0;

const DROP: &str = "effects/whirl_drop.particle.ron";
const ORBIT: &str = "effects/whirl_orbit.particle.ron";
const SPRAY: &str = "effects/whirl_spray.particle.ron";
const RING_FILE: &str = "art/whirlpool_placeholder.aseprite";
const RING_TAG: &str = "ring";
const Z_RING: f32 = 0.8;
const Z_DROPS: f32 = 2.2;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Variant {
    Pivot,
    Spiral,
    Attractor,
    Sprite,
}

const VARIANTS: [Variant; 4] = [
    Variant::Pivot,
    Variant::Spiral,
    Variant::Attractor,
    Variant::Sprite,
];

impl Variant {
    fn label(self) -> &'static str {
        match self {
            Variant::Pivot => "A pivot",
            Variant::Spiral => "A spiral",
            Variant::Attractor => "B attractor",
            Variant::Sprite => "C sprite",
        }
    }

    fn emitters(self) -> usize {
        match self {
            Variant::Pivot | Variant::Spiral => 12,
            Variant::Attractor => 10,
            Variant::Sprite => 8,
        }
    }

    fn effect(self) -> &'static str {
        match self {
            Variant::Pivot | Variant::Spiral => DROP,
            Variant::Attractor => ORBIT,
            Variant::Sprite => SPRAY,
        }
    }
}

/// The variant the next whirlpool will use.
#[derive(Resource)]
struct WhirlLook(usize);

/// The whirlpool. `hurt` holds the age of the whirlpool at the last hit of each body.
#[derive(Component)]
struct Whirlpool {
    variant: Variant,
    centre: Vec2,
    age: f32,
    hurt: Vec<(Entity, f32)>,
}

/// An emitter of a whirlpool, placed every frame from the age. When the whirlpool is gone it stops and removes itself.
#[derive(Component)]
struct WhirlEmitter {
    owner: Entity,
    slot: usize,
}

/// The animated ring of variant C.
#[derive(Component)]
struct RingSprite(Entity);

pub struct WhirlpoolPlugin;

impl Plugin for WhirlpoolPlugin {
    fn build(&self, app: &mut App) {
        let start = std::env::var("SPIKE_WHIRL")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|i| *i < VARIANTS.len())
            .unwrap_or(2);
        app.insert_resource(WhirlLook(start))
            .add_systems(Startup, load_effects)
            .add_systems(
                Update,
                (
                    launch,
                    age_whirlpools,
                    push_enemy,
                    place_emitters,
                    set_attractors,
                    update_rings,
                    end_emitters,
                )
                    .chain(),
            );
    }
}

fn load_effects(server: Res<AssetServer>, mut library: ResMut<EffectLibrary>) {
    for file in [DROP, ORBIT, SPRAY] {
        library.0.push(server.load(file));
    }
}

/// Where emitter `slot` of a whirlpool sits `age` seconds after it started: its radius around the player and its angle
/// (radians, counter-clockwise, y up).
fn pose(variant: Variant, slot: usize, age: f32) -> (f32, f32) {
    let slots = variant.emitters();
    let step = TAU / slots as f32;
    match variant {
        Variant::Pivot => {
            let ring = slots / 2;
            if slot < ring {
                (
                    PIVOT_OUTER_RADIUS,
                    slot as f32 * TAU / ring as f32 - SPIN * age,
                )
            } else {
                let i = slot - ring;
                (
                    PIVOT_INNER_RADIUS,
                    (i as f32 + 0.5) * TAU / ring as f32 + 1.3 * SPIN * age,
                )
            }
        }
        Variant::Spiral => {
            let phase = (age / SPIRAL_SECS + slot as f32 / slots as f32).fract();
            (
                SPIRAL_OUTER + (SPIRAL_INNER - SPIRAL_OUTER) * phase,
                slot as f32 * step - SPIN * age,
            )
        }
        Variant::Attractor => (ORBIT_RADIUS, slot as f32 * step),
        Variant::Sprite => (SPRAY_RADIUS, slot as f32 * step - 0.5 * SPIN * age),
    }
}

/// Opacity of the animated ring `age` seconds after the start: in over `FADE_IN_SECS`, out over the last `FADE_OUT_SECS`.
fn ring_alpha(age: f32) -> f32 {
    (age / FADE_IN_SECS)
        .min((WHIRL_SECS - age) / FADE_OUT_SECS)
        .clamp(0.0, 1.0)
}

fn launch(
    mut fired: MessageReader<FireEffect>,
    look: Res<WhirlLook>,
    glow: Res<Glow>,
    server: Res<AssetServer>,
    mut commands: Commands,
) {
    for shot in fired.read().filter(|shot| shot.effect == EFFECT) {
        let variant = VARIANTS[look.0];
        let owner = commands
            .spawn((
                Whirlpool {
                    variant,
                    centre: shot.from,
                    age: 0.0,
                    hurt: Vec::new(),
                },
                MarchTrigger,
            ))
            .id();
        for slot in 0..variant.emitters() {
            let (radius, angle) = pose(variant, slot, 0.0);
            let position = shot.from + Vec2::from_angle(angle) * radius;
            let mut drop = commands.spawn((
                ParticleEffectHandle(server.load(variant.effect())),
                Transform::from_translation(position.extend(Z_DROPS)),
                WhirlEmitter { owner, slot },
            ));
            glow.insert_color(&mut drop);
        }
        if variant == Variant::Sprite {
            commands.spawn((
                RingSprite(owner),
                AseAnimation {
                    aseprite: server.load(RING_FILE),
                    animation: Animation::tag(RING_TAG).with_repeat(AnimationRepeat::Loop),
                },
                Sprite {
                    color: Color::WHITE.with_alpha(0.0),
                    ..default()
                },
                Transform::from_translation(shot.from.extend(Z_RING)),
            ));
        }
        info!("whirlpool: {}", variant.label());
    }
}

/// Ages the whirlpools, keeps them on the player, hurts the bodies in the band and ends them.
fn age_whirlpools(
    time: Res<Time>,
    player: Single<&Transform, With<Player>>,
    mut pools: Query<(Entity, &mut Whirlpool)>,
    bodies: Query<(Entity, &Body, Has<Enemy>)>,
    mut commands: Commands,
) {
    for (entity, mut pool) in &mut pools {
        pool.age += time.delta_secs();
        pool.centre = player.translation.truncate();
        if pool.age >= WHIRL_SECS {
            commands.entity(entity).despawn();
            continue;
        }
        for (body_entity, body, is_enemy) in &bodies {
            if !rect_overlaps_band(pool.centre, INNER, OUTER, body.0) {
                continue;
            }
            let age = pool.age;
            match pool.hurt.iter_mut().find(|(e, _)| *e == body_entity) {
                Some((_, at)) if age - *at < HURT_INTERVAL => continue,
                Some((_, at)) => *at = age,
                None => pool.hurt.push((body_entity, age)),
            }
            if is_enemy {
                info!("whirlpool hurt the enemy at age {age:.2} s");
                commands
                    .entity(body_entity)
                    .insert(HitFlash(Timer::from_seconds(FLASH_SECS, TimerMode::Once)));
            }
        }
    }
}

/// The shield pushes the red block out of the circle (`PUSH_SPEED`), and blockers stop it. The red block marches back at the
/// player while a whirlpool lasts (`MarchTrigger`), so it settles at the edge.
fn push_enemy(
    time: Res<Time>,
    pools: Query<&Whirlpool>,
    blockers: Query<&Body, (With<Blocker>, Without<Enemy>)>,
    mut enemy: Single<(&mut Body, &mut Transform), With<Enemy>>,
) {
    let rects: Vec<Rect2> = blockers.iter().map(|b| b.0).collect();
    let (body, transform) = &mut *enemy;
    for pool in &pools {
        let push = push_delta(pool.centre, OUTER, body.0, PUSH_SPEED, time.delta_secs());
        if push != Vec2::ZERO {
            body.0.centre = move_rect(body.0, push, &rects);
            transform.translation.x = body.0.centre.x;
            transform.translation.y = body.0.centre.y;
        }
    }
}

fn place_emitters(pools: Query<&Whirlpool>, mut emitters: Query<(&WhirlEmitter, &mut Transform)>) {
    for (emitter, mut transform) in &mut emitters {
        let Ok(pool) = pools.get(emitter.owner) else {
            continue;
        };
        let (radius, angle) = pose(pool.variant, emitter.slot, pool.age);
        let position = pool.centre + Vec2::from_angle(angle) * radius;
        transform.translation = position.extend(Z_DROPS);
        // The attractor variant sends its droplets clockwise along the ring: the emitter points along the tangent.
        if pool.variant == Variant::Attractor {
            transform.rotation = Quat::from_rotation_z(angle - FRAC_PI_2);
        }
    }
}

/// The attractor of an emitter is an offset from the emitter, so it points from the emitter to the player. The emitters
/// of the attractor variant stay at one radius, so the offset does not change. It is set once per effect instance (a
/// reloaded effect file has none again, so it is set again).
fn set_attractors(
    pools: Query<&Whirlpool>,
    mut emitters: Query<(&WhirlEmitter, &mut ParticleEffectInstance)>,
) {
    for (emitter, mut instance) in &mut emitters {
        let Ok(pool) = pools.get(emitter.owner) else {
            continue;
        };
        if pool.variant != Variant::Attractor {
            continue;
        }
        if let Some(effect) = instance.0.as_mut()
            && effect.attractors.is_none()
        {
            let (radius, angle) = pose(pool.variant, emitter.slot, 0.0);
            effect.attractors = Some(vec![Attractor {
                position: -Vec2::from_angle(angle) * radius,
                strength: ORBIT_STRENGTH,
                min_distance: 10.0,
            }]);
        }
    }
}

/// The animated ring follows the player and fades in and out. It is removed with its whirlpool.
fn update_rings(
    pools: Query<&Whirlpool>,
    mut rings: Query<(Entity, &RingSprite, &mut Sprite, &mut Transform)>,
    mut commands: Commands,
) {
    for (entity, ring, mut sprite, mut transform) in &mut rings {
        match pools.get(ring.0) {
            Ok(pool) => {
                sprite.color = Color::WHITE.with_alpha(ring_alpha(pool.age));
                transform.translation = pool.centre.extend(Z_RING);
            }
            Err(_) => commands.entity(entity).despawn(),
        }
    }
}

fn end_emitters(
    pools: Query<(), With<Whirlpool>>,
    mut emitters: Query<(Entity, &WhirlEmitter, &mut ParticleSpawnerState)>,
    mut commands: Commands,
) {
    for (entity, emitter, mut state) in &mut emitters {
        if !pools.contains(emitter.owner) {
            state.active = false;
            commands
                .entity(entity)
                .remove::<WhirlEmitter>()
                .insert(OneShot::Despawn);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pivot_emitters_keep_their_radius_and_the_rings_turn_opposite_ways() {
        let (r0, a0) = pose(Variant::Pivot, 0, 0.0);
        let (r1, a1) = pose(Variant::Pivot, 0, 1.0);
        assert_eq!(r0, r1);
        assert!(a1 < a0, "outer ring turns clockwise");
        let (_, b0) = pose(Variant::Pivot, 6, 0.0);
        let (_, b1) = pose(Variant::Pivot, 6, 1.0);
        assert!(b1 > b0, "inner ring turns counter-clockwise");
    }

    #[test]
    fn spiral_emitters_move_inwards_and_start_over() {
        let radius = |age| pose(Variant::Spiral, 0, age).0;
        assert!(radius(0.5) < radius(0.0));
        assert!((radius(0.0) - SPIRAL_OUTER).abs() < 1e-3);
        assert!(radius(SPIRAL_SECS - 0.01) < SPIRAL_INNER + 1.0);
        // After one full thread the emitter is back at the outside.
        assert!((radius(SPIRAL_SECS * 1.0001) - SPIRAL_OUTER).abs() < 1.0);
        // Neighbours are at different points of their thread.
        assert!(pose(Variant::Spiral, 1, 0.0).0 < pose(Variant::Spiral, 0, 0.0).0);
    }

    #[test]
    fn attractor_emitters_do_not_move() {
        assert_eq!(
            pose(Variant::Attractor, 3, 0.0),
            pose(Variant::Attractor, 3, 2.0)
        );
    }

    #[test]
    fn every_variant_has_emitters_inside_the_view() {
        for variant in VARIANTS {
            for slot in 0..variant.emitters() {
                let (radius, _) = pose(variant, slot, 0.7);
                assert!((20.0..=130.0).contains(&radius), "{variant:?} {radius}");
            }
        }
    }

    #[test]
    fn the_ring_fades_in_stands_and_fades_out() {
        assert_eq!(ring_alpha(0.0), 0.0);
        assert_eq!(ring_alpha(FADE_IN_SECS), 1.0);
        assert_eq!(ring_alpha(WHIRL_SECS / 2.0), 1.0);
        assert!((ring_alpha(WHIRL_SECS - FADE_OUT_SECS / 2.0) - 0.5).abs() < 1e-5);
        assert_eq!(ring_alpha(WHIRL_SECS), 0.0);
    }

    /// Enoki attractor, integrated like Enoki does: v += d * (strength / max(d2, min2) * dt / |d|), p += v * dt.
    /// Returns the distance from the player after `secs` for a droplet that starts at `radius` with `speed` along the tangent.
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
    fn the_attractor_droplets_spiral_outwards_across_the_band() {
        // The start speed of whirl_orbit.particle.ron (185 px/s) is above the circular speed.
        assert!(185.0 > (ORBIT_STRENGTH / ORBIT_RADIUS).sqrt());
        let at = |secs| orbit_radius(ORBIT_RADIUS, 185.0, secs);
        assert!((at(0.0) - ORBIT_RADIUS).abs() < 1.0);
        assert!(
            at(0.4) > at(0.0) && at(0.9) > at(0.4) && at(1.3) > at(0.9),
            "moves out all the time"
        );
        assert!(at(0.2) < INNER, "starts inside the band");
        assert!(
            (OUTER..OUTER + 30.0).contains(&at(1.3)),
            "ends just past the band: {}",
            at(1.3)
        );
    }
}
