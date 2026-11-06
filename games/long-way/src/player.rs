use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::camera::map_rect;
use crate::collision::{PLAYER_BODY_HEIGHT, PLAYER_FEET_OFFSET, PLAYER_HALF_WIDTH};
use crate::map::{CurrentMap, MapEntity, player_start_feet};
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

/// Spawns the player with the feet at the centre of the `player_start` cell of the map. A map without the marker
/// is reported and the player starts at the bottom middle.
pub fn spawn_player(mut commands: Commands, server: Res<AssetServer>, map: Res<CurrentMap>) {
    let feet = player_start_feet(&map.0).unwrap_or_else(|| {
        error!(
            "map {}: no player_start marker, starting at the bottom middle",
            map.0.name
        );
        let area = map_rect(map.0.size);
        Vec2::new((area.min.x + area.max.x) / 2.0, area.min.y)
    });
    let start = feet + Vec2::new(0.0, PLAYER_FEET_OFFSET);
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
        Transform::from_translation(start.extend(0.0)),
    ));
}

fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    map: Res<CurrentMap>,
    world: Res<CollisionWorld>,
    settings: Res<CollisionSettings>,
    mut query: Query<(&mut Transform, &mut Player, &mut SlideAssist)>,
) {
    let area = map_rect(map.0.size);

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
        // The feet stay inside the map, and the body stays below its top edge.
        let feet = feet.clamp(
            area.min + Vec2::new(PLAYER_HALF_WIDTH, 0.0),
            area.max - Vec2::new(PLAYER_HALF_WIDTH, PLAYER_BODY_HEIGHT),
        );
        transform.translation.x = feet.x;
        transform.translation.y = feet.y + PLAYER_FEET_OFFSET;

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
