//! Fireball: a flying flame head with a trail of particles that explodes on an enemy or a blocked feature.
//!
//! Gameplay is plain: a point that flies in a straight line (`rules::first_hit`). The look is layered:
//! the head is the `pickup_red` animation of Long Way, four emitters follow it (core, flame, embers, smoke),
//! and a hit spawns five one-shot emitters (flash, fire, sparks, smoke, shockwave).
//! Effect files: `assets/effects/*.particle.ron`.

use crate::glow::{AdditiveSprite, Glow, SpriteLook};
use crate::harness::{Body, ENEMY_COLOR, EffectLibrary, Enemy, FireEffect};
use crate::rules::{Rect2, first_hit};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_aseprite_ultra::prelude::*;
use bevy_enoki::prelude::*;
use std::f32::consts::PI;

/// Index of the effect in `harness::EFFECTS`.
const EFFECT: usize = 1;

const SPEED: f32 = 420.0;
/// Radius of the hit circle in art pixels.
const HIT_RADIUS: f32 = 14.0;
/// The ball fizzles out after this distance.
const MAX_RANGE: f32 = 1100.0;
/// Distance from the player to where the ball starts.
const MUZZLE_OFFSET: f32 = 34.0;

const HEAD_FILE: &str = "art/fireball_placeholder.aseprite";
const HEAD_TAG: &str = "head";
/// The head art is 64x64 with the ball in the middle. The ball is 42 to 64 art px across (about 50 on average), so at 0.55 it is about 27 px on screen.
const HEAD_SCALE: f32 = 0.55;
/// The ball is centred in the canvas.
const HEAD_ANCHOR: Anchor = Anchor(Vec2::ZERO);

/// Emitters that follow the ball, with their z offset from it. Back to front.
const TRAIL: [(&str, f32); 4] = [
    ("effects/fireball_smoke.particle.ron", -0.3),
    ("effects/fireball_flame.particle.ron", -0.1),
    ("effects/fireball_core.particle.ron", 0.1),
    ("effects/fireball_embers.particle.ron", 0.3),
];
/// One-shot emitters at the hit point.
const IMPACT: [(&str, f32); 5] = [
    ("effects/impact_smoke.particle.ron", 4.0),
    ("effects/impact_fire.particle.ron", 4.1),
    ("effects/impact_shockwave.particle.ron", 4.2),
    ("effects/impact_sparks.particle.ron", 4.3),
    ("effects/impact_flash.particle.ron", 4.4),
];
const BLOB_FILE: &str = "generated/blob_last.png";
const MUZZLE: &str = "effects/muzzle.particle.ron";

const Z_HEAD: f32 = 3.0;
pub const FLASH_SECS: f32 = 0.12;

#[derive(Component)]
struct Fireball {
    velocity: Vec2,
    travelled: f32,
}

/// An emitter that moves with the fireball `owner`, pointing backwards. When the owner is gone the emitter stops
/// spawning and removes itself after its last particle has died.
#[derive(Component)]
struct Follows {
    owner: Entity,
    z: f32,
}

/// A white flash on an enemy that was hit.
#[derive(Component)]
pub struct HitFlash(pub Timer);

pub struct FireballPlugin;

impl Plugin for FireballPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (keep_loaded, load_look))
            .add_systems(
                Update,
                (
                    toggle_look,
                    launch,
                    move_fireballs,
                    emitters_follow,
                    hit_flash,
                )
                    .chain(),
            );
    }
}

/// Which particle look the next shots use: flat squares (Enoki default) or the sprites exported from `fireball_placeholder.aseprite`. B toggles.
#[derive(Resource)]
pub struct Look {
    pub sprites: bool,
    blob: SpriteLook,
}

fn load_look(
    server: Res<AssetServer>,
    mut normals: ResMut<Assets<SpriteParticle2dMaterial>>,
    mut adds: ResMut<Assets<AdditiveSprite>>,
    mut commands: Commands,
) {
    commands.insert_resource(Look {
        sprites: true,
        blob: SpriteLook::new(&mut normals, &mut adds, server.load(BLOB_FILE), 1),
    });
}

fn toggle_look(keys: Res<ButtonInput<KeyCode>>, mut look: ResMut<Look>) {
    if keys.just_pressed(KeyCode::KeyB) {
        look.sprites = !look.sprites;
        info!(
            "particle look: {}",
            if look.sprites { "sprites" } else { "squares" }
        );
    }
}

fn keep_loaded(server: Res<AssetServer>, mut library: ResMut<EffectLibrary>) {
    for (file, _) in TRAIL.iter().chain(IMPACT.iter()) {
        library.0.push(server.load(*file));
    }
    library.0.push(server.load(MUZZLE));
}

fn spawn_emitter(
    commands: &mut Commands,
    server: &AssetServer,
    look: &Look,
    glow: &Glow,
    file: &'static str,
    translation: Vec3,
    rotation: Quat,
) -> Entity {
    let transform = Transform {
        translation,
        rotation,
        ..default()
    };
    let effect = ParticleEffectHandle(server.load(file));
    let mut entity = commands.spawn((effect, transform));
    if look.sprites {
        look.blob.insert(glow, &mut entity);
    } else {
        glow.insert_color(&mut entity);
    }
    entity.id()
}

fn spawn_one_shot(
    commands: &mut Commands,
    server: &AssetServer,
    look: &Look,
    glow: &Glow,
    file: &'static str,
    translation: Vec3,
    rotation: Quat,
) {
    let emitter = spawn_emitter(commands, server, look, glow, file, translation, rotation);
    commands.entity(emitter).insert(OneShot::Despawn);
}

fn launch(
    mut fired: MessageReader<FireEffect>,
    server: Res<AssetServer>,
    look: Res<Look>,
    glow: Res<Glow>,
    mut commands: Commands,
) {
    for shot in fired.read().filter(|shot| shot.effect == EFFECT) {
        let direction = (shot.aim - shot.from).try_normalize().unwrap_or(Vec2::X);
        let start = shot.from + direction * MUZZLE_OFFSET;

        let head = commands
            .spawn((
                Fireball {
                    velocity: direction * SPEED,
                    travelled: 0.0,
                },
                AseAnimation {
                    aseprite: server.load(HEAD_FILE),
                    animation: Animation::tag(HEAD_TAG).with_repeat(AnimationRepeat::Loop),
                },
                Sprite::default(),
                HEAD_ANCHOR,
                Transform::from_translation(start.extend(Z_HEAD))
                    .with_scale(Vec3::splat(HEAD_SCALE)),
            ))
            .id();

        let backwards = Quat::from_rotation_z(direction.to_angle() + PI);
        for (file, z) in TRAIL {
            let emitter = spawn_emitter(
                &mut commands,
                &server,
                &look,
                &glow,
                file,
                start.extend(Z_HEAD + z),
                backwards,
            );
            commands.entity(emitter).insert(Follows { owner: head, z });
        }

        spawn_one_shot(
            &mut commands,
            &server,
            &look,
            &glow,
            MUZZLE,
            start.extend(Z_HEAD),
            Quat::from_rotation_z(direction.to_angle()),
        );
    }
}

fn move_fireballs(
    time: Res<Time>,
    server: Res<AssetServer>,
    look: Res<Look>,
    glow: Res<Glow>,
    mut balls: Query<(Entity, &mut Fireball, &mut Transform)>,
    bodies: Query<(Entity, &Body, Has<Enemy>)>,
    mut commands: Commands,
) {
    let targets: Vec<(Entity, Rect2, bool)> = bodies
        .iter()
        .map(|(entity, body, is_enemy)| (entity, body.0, is_enemy))
        .collect();
    let rects: Vec<Rect2> = targets.iter().map(|(_, rect, _)| *rect).collect();

    for (entity, mut ball, mut transform) in &mut balls {
        let from = transform.translation.truncate();
        let to = from + ball.velocity * time.delta_secs();

        if let Some((index, t)) = first_hit(from, to, HIT_RADIUS, &rects) {
            let point = from.lerp(to, t);
            for (file, z) in IMPACT {
                spawn_one_shot(
                    &mut commands,
                    &server,
                    &look,
                    &glow,
                    file,
                    point.extend(z),
                    Quat::IDENTITY,
                );
            }
            let (target, _, is_enemy) = targets[index];
            if is_enemy {
                commands
                    .entity(target)
                    .insert(HitFlash(Timer::from_seconds(FLASH_SECS, TimerMode::Once)));
            }
            commands.entity(entity).despawn();
        } else {
            ball.travelled += (to - from).length();
            transform.translation = to.extend(transform.translation.z);
            if ball.travelled > MAX_RANGE {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn emitters_follow(
    mut emitters: Query<(Entity, &Follows, &mut Transform, &mut ParticleSpawnerState)>,
    balls: Query<(&Fireball, &Transform), Without<Follows>>,
    mut commands: Commands,
) {
    for (entity, follows, mut transform, mut state) in &mut emitters {
        if let Ok((ball, ball_transform)) = balls.get(follows.owner) {
            transform.translation = ball_transform
                .translation
                .truncate()
                .extend(Z_HEAD + follows.z);
            transform.rotation = Quat::from_rotation_z(ball.velocity.to_angle() + PI);
        } else {
            state.active = false;
            commands
                .entity(entity)
                .remove::<Follows>()
                .insert(OneShot::Despawn);
        }
    }
}

fn hit_flash(
    time: Res<Time>,
    mut flashing: Query<(Entity, &mut Sprite, &mut HitFlash)>,
    mut commands: Commands,
) {
    for (entity, mut sprite, mut flash) in &mut flashing {
        flash.0.tick(time.delta());
        if flash.0.is_finished() {
            sprite.color = ENEMY_COLOR;
            commands.entity(entity).remove::<HitFlash>();
        } else {
            sprite.color = Color::WHITE;
        }
    }
}
