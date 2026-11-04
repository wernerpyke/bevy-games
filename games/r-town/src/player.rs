//! The player: pack character 9, animated in code (`adr-rt3`). The root entity is at the player's feet. The sprite is a
//! child, so the walking bob moves the sprite and not the feet that collision uses.

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use game_kit::Controlled;
use game_kit::camera::CameraBounds;
use game_kit::collision::{CollisionSettings, CollisionWorld, SlideAssist, move_feet};
use game_kit::depth::DepthSort;

use crate::collision::FEET_HALF_WIDTH;
use crate::input::{InputSet, PlayerInput};
use crate::map::{MapEntity, PlayerStart};

/// Walk speed in art pixels per second: 5 cells per second (`adr-rt4`).
pub const WALK_SPEED: f32 = 80.0;
/// Height of the player's canvas in `characters.aseprite` (16 wide, 17 high).
const CANVAS_HEIGHT: f32 = 17.0;
/// Walking bob: the sprite lifts one pixel, this many times a second.
const BOB_RATE: f32 = 6.0;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player).add_systems(
            Update,
            (move_player, animate_player).chain().after(InputSet),
        );
    }
}

#[derive(Component)]
pub struct Player {
    /// The way the player looks, a unit vector in one of 8 directions. The last non-zero move direction, right at
    /// the start. Weapons fire along it.
    pub facing: Vec2,
    facing_left: bool,
    walking: bool,
    /// Seconds spent walking, for the bob.
    walk_time: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            facing: Vec2::X,
            facing_left: false,
            walking: false,
            walk_time: 0.0,
        }
    }
}

/// The facing after the player asked for `direction` this frame: that direction as a unit vector, or `current`
/// when no key is down.
fn next_facing(current: Vec2, direction: Vec2) -> Vec2 {
    direction.try_normalize().unwrap_or(current)
}

#[derive(Component)]
struct PlayerSprite;

/// Creates the player at the start cell. Also run again when the map is reloaded (F1).
pub(crate) fn spawn_player(
    mut commands: Commands,
    server: Res<AssetServer>,
    player_start: Res<PlayerStart>,
) {
    let start = player_start.0.as_vec2() * crate::camera::CELL_SIZE;
    commands
        .spawn((
            Controlled,
            Player::default(),
            MapEntity,
            SlideAssist::default(),
            DepthSort { feet_offset: 0.0 },
            Transform::from_translation(start.extend(0.0)),
            Visibility::default(),
        ))
        .with_child((
            PlayerSprite,
            AseAnimation {
                aseprite: server.load("art/characters/characters.aseprite"),
                animation: Animation::tag("char_9"),
            },
            Sprite::default(),
            Transform::from_xyz(0.0, CANVAS_HEIGHT / 2.0, 0.0),
        ));
}

fn move_player(
    input: Res<PlayerInput>,
    time: Res<Time>,
    world: Res<CollisionWorld>,
    settings: Res<CollisionSettings>,
    bounds: Res<CameraBounds>,
    player: Single<(&mut Transform, &mut Player, &mut SlideAssist)>,
) {
    let (mut transform, mut player, mut assist) = player.into_inner();
    let direction = input.move_dir;
    player.walking = direction != Vec2::ZERO;
    if !player.walking {
        assist.reset();
        return;
    }
    player.facing = next_facing(player.facing, direction);
    if direction.x != 0.0 {
        player.facing_left = direction.x < 0.0;
    }
    let dt = time.delta_secs();
    let old = transform.translation.truncate();
    let step = direction.normalize() * WALK_SPEED * dt;
    let feet = move_feet(&settings, &mut assist, old, step, dt, world.edges());
    let area = bounds.0;
    let feet = feet.clamp(
        area.min + Vec2::new(FEET_HALF_WIDTH, 0.0),
        area.max - Vec2::new(FEET_HALF_WIDTH, 0.0),
    );
    transform.translation.x = feet.x;
    transform.translation.y = feet.y;
}

fn animate_player(
    time: Res<Time>,
    mut player: Single<&mut Player>,
    mut sprite: Single<(&mut Sprite, &mut Transform), With<PlayerSprite>>,
) {
    if player.walking {
        player.walk_time += time.delta_secs();
    } else {
        player.walk_time = 0.0;
    }
    let (sprite, transform) = &mut *sprite;
    sprite.flip_x = player.facing_left;
    let lift = if player.walking {
        (player.walk_time * BOB_RATE) as i32 % 2
    } else {
        0
    };
    transform.translation.y = CANVAS_HEIGHT / 2.0 + lift as f32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_player_faces_right() {
        assert_eq!(Player::default().facing, Vec2::X);
    }

    #[test]
    fn facing_follows_the_move_direction_as_a_unit_vector() {
        assert_eq!(next_facing(Vec2::X, Vec2::new(0.0, -1.0)), Vec2::NEG_Y);
        let diagonal = next_facing(Vec2::X, Vec2::new(-1.0, 1.0));
        assert!((diagonal.length() - 1.0).abs() < 1e-5);
        assert!(diagonal.x < 0.0 && diagonal.y > 0.0);
    }

    #[test]
    fn facing_is_kept_when_no_key_is_down() {
        assert_eq!(next_facing(Vec2::NEG_X, Vec2::ZERO), Vec2::NEG_X);
    }
}
