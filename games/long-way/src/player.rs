use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::camera::half_view;
use crate::collision::PLAYER_FEET_OFFSET;
use crate::map::MapEntity;
use game_kit::collision::{CollisionSettings, CollisionWorld, SlideAssist, move_feet};
use game_kit::depth::DepthSort;

/// Walk speed in art pixels per second.
const WALK_SPEED: f32 = 300.0;
/// Playback speed multiplier for the walking animations. 1.0 is the speed set in Aseprite.
const WALK_ANIMATION_SPEED: f32 = 2.5;
/// The `Run` tag in `Hunter.aseprite` is the sideways walk. Set to match the way the art faces.
const WALK_ART_FACES_RIGHT: bool = true;
/// Draw the `Weapon` layer of `Hunter.aseprite`. The `background` layer is never drawn.
const SHOW_WEAPON: bool = false;
/// Half the 128x128 canvas of `Hunter.aseprite`. Keeps the whole canvas on screen.
const HALF_SIZE: f32 = 64.0;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, (move_player, animate_player).chain());
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Facing {
    Up,
    Down,
    Side,
}

#[derive(Component)]
pub struct Player {
    facing: Facing,
    walking: bool,
    /// Tag currently playing, so the animation only restarts when it changes.
    playing: &'static str,
}

/// Layers of `Hunter.aseprite` that the game draws, by name.
fn hunter_layers() -> Vec<String> {
    let mut layers = vec!["Body".to_string()];
    if SHOW_WEAPON {
        layers.push("Weapon".to_string());
    }
    layers
}

pub fn spawn_player(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn((
        MapEntity,
        game_kit::Controlled,
        Player {
            facing: Facing::Down,
            walking: false,
            playing: "Idle",
        },
        AseAnimation {
            aseprite: server
                .load_builder()
                .with_settings::<AsepriteLoaderSettings>(|settings| {
                    settings.layer_selection = LayerSelectionSetting::Mask(hunter_layers());
                })
                .load("art/characters/Hunter.aseprite"),
            animation: Animation::tag("Idle").with_repeat(AnimationRepeat::Loop),
        },
        Sprite::default(),
        DepthSort {
            feet_offset: PLAYER_FEET_OFFSET,
        },
        SlideAssist::default(),
        Transform::default(),
    ));
}

fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    window: Single<&Window>,
    world: Res<CollisionWorld>,
    settings: Res<CollisionSettings>,
    mut query: Query<(&mut Transform, &mut Player, &mut SlideAssist)>,
) {
    let limit = (half_view(&window) - Vec2::splat(HALF_SIZE)).max(Vec2::ZERO);

    let mut direction = Vec2::ZERO;
    if input.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if input.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    if input.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if input.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }

    for (mut transform, mut player, mut assist) in &mut query {
        player.walking = direction != Vec2::ZERO;
        if !player.walking {
            assist.reset();
            continue;
        }
        let step = direction.normalize() * WALK_SPEED * time.delta_secs();
        // Collision works on the feet, not the centre of the canvas.
        let old_feet = transform.translation.truncate() - Vec2::new(0.0, PLAYER_FEET_OFFSET);
        let feet = move_feet(
            &settings,
            &mut assist,
            old_feet,
            step,
            time.delta_secs(),
            world.edges(),
        );
        transform.translation.x = feet.x;
        transform.translation.y = feet.y + PLAYER_FEET_OFFSET;
        transform.translation.x = transform.translation.x.clamp(-limit.x, limit.x);
        transform.translation.y = transform.translation.y.clamp(-limit.y, limit.y);

        // Horizontal movement wins when walking diagonally, so the sideways walk plays.
        if direction.x != 0.0 {
            player.facing = Facing::Side;
            let faces_right = direction.x > 0.0;
            transform.scale.x = if faces_right == WALK_ART_FACES_RIGHT {
                1.0
            } else {
                -1.0
            };
        } else if direction.y > 0.0 {
            player.facing = Facing::Up;
        } else {
            player.facing = Facing::Down;
        }
    }
}

fn animate_player(mut query: Query<(&mut AseAnimation, &mut Player)>) {
    for (mut ase, mut player) in &mut query {
        let tag = match (player.walking, player.facing) {
            (false, _) => "Idle",
            (true, Facing::Side) => "Run",
            (true, Facing::Up) => "Up",
            (true, Facing::Down) => "Down",
        };
        if tag != player.playing {
            ase.animation.play_loop(tag);
            ase.animation.speed = if player.walking {
                WALK_ANIMATION_SPEED
            } else {
                1.0
            };
            player.playing = tag;
        }
    }
}
