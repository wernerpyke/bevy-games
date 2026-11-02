//! The hero: pack character 9, animated in code (`adr-rt3`). The root entity is at the hero's feet. The sprite is a
//! child, so the walking bob moves the sprite and not the feet that collision uses.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::Controlled;
use game_kit::collision::{CollisionSettings, CollisionWorld, SlideAssist, move_feet};
use game_kit::depth::DepthSort;

use crate::collision::FEET_HALF_WIDTH;
use crate::map::{MAP_CELLS, map_rect};

/// Walk speed in art pixels per second: 5 cells per second (`adr-rt4`).
const WALK_SPEED: f32 = 80.0;
/// Height of the hero's canvas in `characters.aseprite` (16 wide, 17 high).
const CANVAS_HEIGHT: f32 = 17.0;
/// Walking bob: the sprite lifts one pixel, this many times a second.
const BOB_RATE: f32 = 6.0;

pub struct HeroPlugin;

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hero)
            .add_systems(Update, (move_hero, animate_hero).chain());
    }
}

#[derive(Component, Default)]
struct Hero {
    facing_left: bool,
    walking: bool,
    /// Seconds spent walking, for the bob.
    walk_time: f32,
}

#[derive(Component)]
struct HeroSprite;

fn spawn_hero(mut commands: Commands, server: Res<AssetServer>) {
    let start = MAP_CELLS.as_vec2() * crate::camera::CELL_SIZE / 2.0;
    commands
        .spawn((
            Controlled,
            Hero::default(),
            SlideAssist::default(),
            DepthSort { feet_offset: 0.0 },
            Transform::from_translation(start.extend(0.0)),
            Visibility::default(),
        ))
        .with_child((
            HeroSprite,
            AseAnimation {
                aseprite: server.load("art/characters/characters.aseprite"),
                animation: Animation::tag("char_9"),
            },
            Sprite::default(),
            Transform::from_xyz(0.0, CANVAS_HEIGHT / 2.0, 0.0),
        ));
}

/// The direction the arrow keys or WASD give, not normalised.
fn input_direction(input: &ButtonInput<KeyCode>) -> Vec2 {
    let mut direction = Vec2::ZERO;
    if input.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
        direction.x -= 1.0;
    }
    if input.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
        direction.x += 1.0;
    }
    if input.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        direction.y += 1.0;
    }
    if input.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        direction.y -= 1.0;
    }
    direction
}

fn move_hero(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    world: Res<CollisionWorld>,
    settings: Res<CollisionSettings>,
    hero: Single<(&mut Transform, &mut Hero, &mut SlideAssist)>,
) {
    let (mut transform, mut hero, mut assist) = hero.into_inner();
    let direction = input_direction(&input);
    hero.walking = direction != Vec2::ZERO;
    if !hero.walking {
        assist.reset();
        return;
    }
    if direction.x != 0.0 {
        hero.facing_left = direction.x < 0.0;
    }
    let dt = time.delta_secs();
    let old = transform.translation.truncate();
    let step = direction.normalize() * WALK_SPEED * dt;
    let feet = move_feet(&settings, &mut assist, old, step, dt, world.edges());
    let area = map_rect();
    let feet = feet.clamp(
        area.min + Vec2::new(FEET_HALF_WIDTH, 0.0),
        area.max - Vec2::new(FEET_HALF_WIDTH, 0.0),
    );
    transform.translation.x = feet.x;
    transform.translation.y = feet.y;
}

fn animate_hero(
    time: Res<Time>,
    mut hero: Single<&mut Hero>,
    mut sprite: Single<(&mut Sprite, &mut Transform), With<HeroSprite>>,
) {
    if hero.walking {
        hero.walk_time += time.delta_secs();
    } else {
        hero.walk_time = 0.0;
    }
    let (sprite, transform) = &mut *sprite;
    sprite.flip_x = hero.facing_left;
    let lift = if hero.walking {
        (hero.walk_time * BOB_RATE) as i32 % 2
    } else {
        0
    };
    transform.translation.y = CANVAS_HEIGHT / 2.0 + lift as f32;
}
