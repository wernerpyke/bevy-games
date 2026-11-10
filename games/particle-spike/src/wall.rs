//! Wall of ice cubes or flames: a solid rectangle that stays for a while, then disappears. Step 6 of the spike.
//!
//! Gameplay is plain: the wall is a `Body` with the `Blocker` marker, so the red block (arrow keys) and the fireball
//! stop at it. It is axis-aligned and stands across the aim: a vertical wall when the aim is mostly sideways, a
//! horizontal one when it is mostly up or down. It lasts `WALL_SECS`. While a wall stands, the red block marches
//! towards the player, so you see it walk into the wall and stop (`rules::move_rect`).
//! The look depends on the element (`I` toggles it, shared with the ring):
//! - Ice: overlapping cubes of three heights (`cube_s`, `cube_m`, `cube_t` in `ice_wall_placeholder.aseprite`). Each is a
//!   plain animated sprite that plays its growth tag once and stays on the last frame (frame times are in the file).
//!   The cubes start one after another from the middle outwards (`GROW_SPREAD`). Cubes in front (lower on screen) are
//!   drawn over the ones behind. In the last `FADE_SECS` of the wall the sprites fade out with their alpha, and they are
//!   removed with the wall.
//! - Fire: a row of Enoki emitters that keep spawning flames (`wall_fire` sprites). When the wall is gone the emitters
//!   stop and remove themselves after their last particle.

use crate::glow::Glow;
use crate::harness::{Blocker, Body, EffectLibrary, FireEffect, MarchTrigger};
use crate::ring::{Element, RingStyle, look};
use crate::rules::Rect2;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_aseprite_ultra::prelude::*;
use bevy_enoki::prelude::*;

/// Indexes of the effects in `harness::EFFECTS`.
const FIRE_WALL: usize = 3;
const ICE_WALL: usize = 5;

/// Length of the wall along its line, and its thickness, in art pixels.
const LENGTH: f32 = 220.0;
const THICKNESS: f32 = 28.0;
/// The wall stands this far from the player, toward the aim (clamped).
const MIN_DISTANCE: f32 = 80.0;
const MAX_DISTANCE: f32 = 220.0;
/// The wall ends this long after it is placed: the last cube starts at `GROW_SPREAD` and takes 0.48 s to grow (4 frames
/// of 120 ms), the cubes stand for about 2 s, then they fade out over `FADE_SECS`.
const WALL_SECS: f32 = 3.3;
const FADE_SECS: f32 = 0.4;
/// Distance between flame emitters along the line.
const FLAME_SPACING: f32 = 22.0;
/// Distance between cubes along the line. A cube is 30 px wide, so they overlap.
const CUBE_SPACING: f32 = 16.0;
/// The cubes start over this long, from the middle of the wall outwards.
const GROW_SPREAD: f32 = 0.4;
/// A cube is moved this far across the wall (each side) and along it, at random.
const CUBE_JITTER: Vec2 = Vec2::new(8.0, 3.0);

const FLAME: &str = "effects/wall_shard.particle.ron";
const CUBE_FILE: &str = "art/ice_wall_placeholder.aseprite";
const CUBE_TAGS: [&str; 3] = ["cube_s", "cube_m", "cube_t"];
const Z_WALL: f32 = 2.3;

/// An ice cube that is not spawned yet: when (seconds after the wall is placed), where its base is, which height.
struct Cube {
    delay: f32,
    ground: Vec2,
    variant: usize,
}

/// The wall. It lives as long as its timer runs. `pending` are the cubes that have not started to grow.
#[derive(Component)]
struct Wall {
    timer: Timer,
    pending: Vec<Cube>,
}

/// An emitter of a wall. When the wall is gone it stops and removes itself after the last particle.
#[derive(Component)]
struct WallEmitter(Entity);

/// An ice cube sprite of a wall. It fades with the wall and is removed when the wall is gone.
#[derive(Component)]
struct IceCube(Entity);

pub struct WallPlugin;

impl Plugin for WallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_effects).add_systems(
            Update,
            (launch, grow_walls, fade_cubes, expire_walls, end_emitters).chain(),
        );
    }
}

fn load_effects(server: Res<AssetServer>, mut library: ResMut<EffectLibrary>) {
    library.0.push(server.load(FLAME));
}

/// Where the wall stands and how big it is, for a player at `from` aiming at `aim`.
fn wall_rect(from: Vec2, aim: Vec2) -> Rect2 {
    let to_aim = aim - from;
    let direction = if to_aim == Vec2::ZERO {
        Vec2::X
    } else {
        to_aim.normalize()
    };
    let distance = to_aim.length().clamp(MIN_DISTANCE, MAX_DISTANCE);
    let size = if direction.x.abs() >= direction.y.abs() {
        Vec2::new(THICKNESS, LENGTH)
    } else {
        Vec2::new(LENGTH, THICKNESS)
    };
    Rect2::new(from + direction * distance, size)
}

/// The direction of the wall line.
fn along(rect: Rect2) -> Vec2 {
    if rect.size.x > rect.size.y {
        Vec2::X
    } else {
        Vec2::Y
    }
}

/// Emitter positions along the wall line, centred, `spacing` apart.
fn line_points(rect: Rect2, spacing: f32) -> Vec<Vec2> {
    let along = along(rect);
    let length = rect.size.dot(along);
    let count = (length / spacing).round().max(1.0) as usize;
    (0..count)
        .map(|i| {
            let offset = (i as f32 + 0.5) / count as f32 - 0.5;
            rect.centre + along * offset * length
        })
        .collect()
}

/// A repeatable number from 0 to 1 for cube `index`. `salt` gives each use its own number. No random crate needed,
/// and a wall looks the same every time it is placed in the same spot.
fn scatter(index: usize, salt: u32) -> f32 {
    let mut x = (index as u32 + 1).wrapping_mul(0x9E37_79B1) ^ salt.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x % 1000) as f32 / 1000.0
}

/// The cubes of an ice wall: a row along the line with a little scatter across and along it, a height each, and a start
/// delay that grows with the distance from the middle.
fn cubes(rect: Rect2) -> Vec<Cube> {
    let along = along(rect);
    let across = along.perp().abs();
    let half = rect.size.dot(along) / 2.0;
    line_points(rect, CUBE_SPACING)
        .into_iter()
        .enumerate()
        .map(|(i, point)| {
            let from_middle = (point - rect.centre).dot(along).abs() / half;
            let jitter = (scatter(i, 1) * 2.0 - 1.0) * CUBE_JITTER.x * across
                + (scatter(i, 2) * 2.0 - 1.0) * CUBE_JITTER.y * along;
            Cube {
                delay: from_middle * GROW_SPREAD,
                ground: point + jitter,
                variant: (scatter(i, 3) * 3.0) as usize % 3,
            }
        })
        .collect()
}

fn launch(
    mut fired: MessageReader<FireEffect>,
    style: Res<RingStyle>,
    glow: Res<Glow>,
    server: Res<AssetServer>,
    existing: Query<Entity, With<Wall>>,
    mut commands: Commands,
) {
    for shot in fired.read() {
        let element = match shot.effect {
            FIRE_WALL => Element::Fire,
            ICE_WALL => Element::Ice,
            _ => continue,
        };
        // One wall at a time: a new one replaces the old one.
        for old in &existing {
            commands.entity(old).despawn();
        }
        let rect = wall_rect(shot.from, shot.aim);
        let pending = match element {
            Element::Ice => cubes(rect),
            Element::Fire => Vec::new(),
        };
        let wall = commands
            .spawn((
                Wall {
                    timer: Timer::from_seconds(WALL_SECS, TimerMode::Once),
                    pending,
                },
                Body(rect),
                Blocker,
                MarchTrigger,
            ))
            .id();
        if element == Element::Fire {
            for point in line_points(rect, FLAME_SPACING) {
                let mut flame = commands.spawn((
                    ParticleEffectHandle(server.load(FLAME)),
                    Transform::from_translation(point.extend(Z_WALL)),
                    WallEmitter(wall),
                ));
                look(&style, Element::Fire).insert(&glow, &mut flame);
            }
        }
    }
}

/// Starts the cubes whose time has come. A cube is a sprite with its base on the ground point. Cubes lower on the screen
/// get a higher z, so they are drawn over the cubes behind them.
fn grow_walls(
    mut walls: Query<(Entity, &mut Wall)>,
    server: Res<AssetServer>,
    mut commands: Commands,
) {
    for (entity, mut wall) in &mut walls {
        let elapsed = wall.timer.elapsed_secs();
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut wall.pending)
            .into_iter()
            .partition(|cube| cube.delay <= elapsed);
        wall.pending = later;
        for cube in due {
            commands.spawn((
                IceCube(entity),
                AseAnimation {
                    aseprite: server.load(CUBE_FILE),
                    animation: Animation::tag(CUBE_TAGS[cube.variant])
                        .with_repeat(AnimationRepeat::Count(0)),
                },
                Sprite::default(),
                Anchor::BOTTOM_CENTER,
                Transform::from_translation(cube.ground.extend(Z_WALL - cube.ground.y * 0.0005)),
            ));
        }
    }
}

/// The cubes fade out over the last `FADE_SECS` of their wall (alpha only), and go when the wall goes.
fn fade_cubes(
    walls: Query<&Wall>,
    mut cubes: Query<(Entity, &IceCube, &mut Sprite)>,
    mut commands: Commands,
) {
    for (entity, cube, mut sprite) in &mut cubes {
        match walls.get(cube.0) {
            Ok(wall) => {
                sprite.color = Color::WHITE.with_alpha(fade_alpha(wall.timer.elapsed_secs()));
            }
            Err(_) => commands.entity(entity).despawn(),
        }
    }
}

/// Opacity of the cubes `age` seconds after the wall is placed: 1 until the fade starts, then down to 0 at the end.
fn fade_alpha(age: f32) -> f32 {
    ((WALL_SECS - age) / FADE_SECS).clamp(0.0, 1.0)
}

fn expire_walls(time: Res<Time>, mut walls: Query<(Entity, &mut Wall)>, mut commands: Commands) {
    for (entity, mut wall) in &mut walls {
        if wall.timer.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn end_emitters(
    walls: Query<(), With<Wall>>,
    mut emitters: Query<(Entity, &WallEmitter, &mut ParticleSpawnerState)>,
    mut commands: Commands,
) {
    for (entity, emitter, mut state) in &mut emitters {
        if !walls.contains(emitter.0) {
            state.active = false;
            commands
                .entity(entity)
                .remove::<WallEmitter>()
                .insert(OneShot::Despawn);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cubes_are_opaque_until_the_fade_and_gone_at_the_end() {
        assert_eq!(fade_alpha(0.0), 1.0);
        assert_eq!(fade_alpha(WALL_SECS - FADE_SECS), 1.0);
        assert!((fade_alpha(WALL_SECS - FADE_SECS / 2.0) - 0.5).abs() < 1e-5);
        assert_eq!(fade_alpha(WALL_SECS), 0.0);
    }

    #[test]
    fn the_wall_stands_across_the_aim() {
        let sideways = wall_rect(Vec2::ZERO, Vec2::new(300.0, 10.0));
        assert_eq!(sideways.size, Vec2::new(THICKNESS, LENGTH));
        let upward = wall_rect(Vec2::ZERO, Vec2::new(10.0, 300.0));
        assert_eq!(upward.size, Vec2::new(LENGTH, THICKNESS));
    }

    #[test]
    fn the_distance_is_clamped() {
        assert_eq!(
            wall_rect(Vec2::ZERO, Vec2::new(500.0, 0.0)).centre.x,
            MAX_DISTANCE
        );
        assert_eq!(
            wall_rect(Vec2::ZERO, Vec2::new(5.0, 0.0)).centre.x,
            MIN_DISTANCE
        );
    }

    #[test]
    fn emitters_cover_the_line() {
        let rect = Rect2::new(Vec2::new(100.0, 0.0), Vec2::new(THICKNESS, LENGTH));
        let points = line_points(rect, 22.0);
        assert_eq!(points.len(), 10);
        assert!(points.iter().all(|p| p.x == 100.0));
        assert!(points.iter().all(|p| p.y.abs() < LENGTH / 2.0));
    }

    #[test]
    fn cubes_grow_from_the_middle_outwards_and_stay_on_the_wall() {
        let rect = Rect2::new(Vec2::new(100.0, 0.0), Vec2::new(THICKNESS, LENGTH));
        let cubes = cubes(rect);
        assert!(cubes.len() >= 12);
        assert!(cubes.iter().all(|c| (0.0..=GROW_SPREAD).contains(&c.delay)));
        // The delay is smallest in the middle and largest at the ends.
        let middle = cubes
            .iter()
            .min_by(|a, b| a.delay.total_cmp(&b.delay))
            .unwrap();
        assert!(middle.ground.y.abs() < CUBE_SPACING);
        // Every cube base is inside the wall rectangle.
        let wall_area = Rect2::new(rect.centre, rect.size + Vec2::splat(1.0));
        assert!(
            cubes
                .iter()
                .all(|c| wall_area.closest_point(c.ground) == c.ground)
        );
        // All three heights are used.
        assert!((0..3).all(|v| cubes.iter().any(|c| c.variant == v)));
    }
}
