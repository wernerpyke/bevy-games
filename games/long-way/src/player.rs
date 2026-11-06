use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;

use crate::camera::MAP_ORIGIN;
use crate::collision::{PLAYER_BODY_HEIGHT, PLAYER_FEET_OFFSET, PLAYER_HALF_WIDTH};
use crate::map::player_start_feet;
use game_kit::collision::{CollisionSettings, CollisionWorld, SlideAssist, move_feet};
use game_kit::depth::DepthSort;
use game_kit::map::Facing as EntryFacing;
use game_kit::map_transition::{ActiveMaps, Arrival, MapRegistry, MapTransition, clamp_to_union};

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
pub fn spawn_player(
    mut commands: Commands,
    server: Res<AssetServer>,
    registry: Res<MapRegistry>,
    active: Res<ActiveMaps>,
) {
    let Some(registered) = registry.get(&active.current) else {
        error!(
            "player: the start map `{}` is not registered",
            active.current
        );
        return;
    };
    let map = &registered.map;
    let feet = player_start_feet(map, MAP_ORIGIN).unwrap_or_else(|| {
        error!(
            "map {}: no player_start marker, starting at the bottom middle",
            map.name
        );
        let width = map.size.x as f32 * map.cell_size;
        Vec2::new(MAP_ORIGIN.x + width / 2.0, MAP_ORIGIN.y)
    });
    let start = feet + Vec2::new(0.0, PLAYER_FEET_OFFSET);
    commands.spawn((
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
    active: Res<ActiveMaps>,
    transition: Res<MapTransition>,
    world: Res<CollisionWorld>,
    settings: Res<CollisionSettings>,
    mut query: Query<(&mut Transform, &mut Player, &mut SlideAssist)>,
) {
    // The feet stay inside a map, and the body stays below its top edge. They may cross from one map to another only
    // through a doorway of a seamless exit.
    let walkable = active.walkable(
        Vec2::new(PLAYER_HALF_WIDTH, 0.0),
        Vec2::new(PLAYER_HALF_WIDTH, PLAYER_BODY_HEIGHT),
    );

    // The player stands still while the screen fades.
    if transition.locks_input() {
        for (_, mut player, mut assist) in &mut query {
            player.walking = false;
            assist.reset();
        }
        return;
    }

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
        let feet = clamp_to_union(&walkable, old_feet, feet);
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

/// Puts the player in the entry after a map transition (a hook of `game_kit::map_transition`): the feet go to the
/// arrival position and the player looks the way the entry says.
pub fn arrive_player(
    In(arrival): In<Arrival>,
    active: Res<ActiveMaps>,
    mut query: Query<(&mut Transform, &mut Player, &mut SlideAssist)>,
) {
    // An entry at the edge of a map can put the feet where the body would be cut off by the edge of the view (the top
    // of the map) or outside the walkable area. Move them to the nearest place the player can stand, as `move_player` does.
    let walkable = active.walkable(
        Vec2::new(PLAYER_HALF_WIDTH, 0.0),
        Vec2::new(PLAYER_HALF_WIDTH, PLAYER_BODY_HEIGHT),
    );
    let feet = clamp_to_union(&walkable, arrival.feet, arrival.feet);
    for (mut transform, mut player, mut assist) in &mut query {
        transform.translation.x = feet.x;
        transform.translation.y = feet.y + PLAYER_FEET_OFFSET;
        assist.reset();
        player.walking = false;
        let (facing, faces_right) = match arrival.facing {
            None => continue,
            Some(EntryFacing::Up) => (Facing::Up, None),
            Some(EntryFacing::Down) => (Facing::Down, None),
            Some(EntryFacing::Left) => (Facing::Side, Some(false)),
            Some(EntryFacing::Right) => (Facing::Side, Some(true)),
        };
        player.facing = facing;
        if let Some(right) = faces_right {
            transform.scale.x = if right == WALK_ART_FACES_RIGHT {
                1.0
            } else {
                -1.0
            };
        }
    }
}
